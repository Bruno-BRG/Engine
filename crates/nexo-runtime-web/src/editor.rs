//! Editor de cena no WebAssembly. A árvore de nós fica aqui (SCN-005): o TypeScript consulta
//! o estado e envia operações de forma síncrona, sem ida e volta pelo Tauri a cada movimento.
//! A reprodução guarda uma cópia da cena e um relógio; a edição fica intacta enquanto joga.

use nexo_core::query::{self, Draw};
use nexo_core::timer::{TimerEvent, Timers};
use nexo_core::{AudioPlayback, Input, InputMap, ProjectSettings, Scene, Uid, registry};
use nexo_physics::{Physics, check_reference};
use serde::Serialize;
use serde_json::Value;
use wasm_bindgen::prelude::*;

/// Reprodução: cópia da cena, relógio, mundo de física, áudio e temporizadores montados a partir dela.
pub(crate) struct Play {
    /// `pub(crate)` para `input_devices` ler as propriedades dos nós (joystick de toque).
    pub(crate) scene: Scene,
    time: f64,
    physics: Physics,
    audio: AudioPlayback,
    timers: Timers,
}

#[wasm_bindgen]
pub struct SceneEditor {
    scene: Scene,
    /// `pub(crate)`: os métodos de entrada em `input_devices` leem a reprodução e escrevem a entrada.
    pub(crate) play: Option<Play>,
    /// Configurações do projeto (settings.ron). O mapa de entrada usado pelo jogo é `settings.input`.
    settings: ProjectSettings,
    pub(crate) input: Input,
}

fn js_err(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

fn parse_uid(s: &str) -> Result<Uid, JsError> {
    s.parse::<Uid>().map_err(js_err)
}

fn opt_uid(s: Option<String>) -> Result<Option<Uid>, JsError> {
    s.filter(|s| !s.is_empty()).map(|s| parse_uid(&s)).transpose()
}

fn to_json(v: &impl Serialize) -> Result<String, JsError> {
    serde_json::to_string(v).map_err(js_err)
}

#[wasm_bindgen]
impl SceneEditor {
    /// Editor com uma cena vazia. `seed` alimenta a geração de UIDs (use um número aleatório).
    #[wasm_bindgen(constructor)]
    pub fn new(seed: f64) -> SceneEditor {
        SceneEditor {
            scene: Scene::new("Principal", seed as u64),
            play: None,
            settings: ProjectSettings::default(),
            input: Input::default(),
        }
    }

    /// Registro de tipos de nó em JSON (propriedades, grupos, faixas e herança).
    pub fn registry() -> Result<String, JsError> {
        to_json(&registry::node_types())
    }

    /// Substitui a cena por uma nova com raiz `Node2D` e `Camera2D`.
    pub fn new_default(&mut self, name: &str, seed: f64) {
        self.scene = Scene::new_default(name, seed as u64);
        self.play = None;
    }

    /// Carrega o texto de uma cena. Aceita o formato atual e migra o formato 1 da Fase 0.
    pub fn load_text(&mut self, text: &str, seed: f64) -> Result<(), JsError> {
        self.scene = Scene::from_text(text, seed as u64).map_err(js_err)?;
        self.play = None;
        Ok(())
    }

    /// Texto RON da cena no formato atual, pronto para salvar.
    pub fn to_ron(&self) -> Result<String, JsError> {
        self.scene.to_ron().map_err(js_err)
    }

    /// Texto RON de uma cena feita só com a subárvore de `uid` (Salvar ramo como cena).
    pub fn branch_to_ron(&self, uid: &str, seed: f64) -> Result<String, JsError> {
        self.scene
            .branch_as_scene(parse_uid(uid)?, seed as u64)
            .and_then(|s| s.to_ron())
            .map_err(js_err)
    }

    // ---------- Operações de edição (sempre sobre a cena de edição) ----------

    pub fn add_node(&mut self, parent: Option<String>, kind: &str, index: Option<u32>) -> Result<String, JsError> {
        let parent = opt_uid(parent)?;
        let uid = self.scene.add_node(parent, kind, index.map(|i| i as usize)).map_err(js_err)?;
        Ok(uid.to_string())
    }

    pub fn remove_node(&mut self, uid: &str) -> Result<(), JsError> {
        self.scene.remove_node(parse_uid(uid)?).map_err(js_err)
    }

    pub fn rename_node(&mut self, uid: &str, name: &str) -> Result<(), JsError> {
        self.scene.rename_node(parse_uid(uid)?, name).map_err(js_err)
    }

    /// Define uma propriedade. `value_json` é o valor em JSON. Com `coalesce`, alterações com a
    /// mesma chave viram um passo de desfazer só até `end_group`.
    pub fn set_prop(&mut self, uid: &str, key: &str, value_json: &str, coalesce: Option<String>) -> Result<(), JsError> {
        let value: Value = serde_json::from_str(value_json).map_err(js_err)?;
        // Camada e material são nomes das configurações do projeto (PHY-020, PHY-023).
        check_reference(&self.settings, key, &value).map_err(js_err)?;
        self.scene
            .set_prop(parse_uid(uid)?, key, value, coalesce.as_deref())
            .map_err(js_err)
    }

    /// Move o nó para `parent` (ou raiz) na posição final `index` entre os irmãos.
    pub fn move_node(&mut self, uid: &str, parent: Option<String>, index: u32) -> Result<(), JsError> {
        self.scene
            .move_node(parse_uid(uid)?, opt_uid(parent)?, index as usize)
            .map_err(js_err)
    }

    pub fn duplicate_node(&mut self, uid: &str) -> Result<String, JsError> {
        Ok(self.scene.duplicate_node(parse_uid(uid)?).map_err(js_err)?.to_string())
    }

    pub fn copy_node(&mut self, uid: &str) -> Result<(), JsError> {
        self.scene.copy_node(parse_uid(uid)?).map_err(js_err)
    }

    pub fn paste_node(&mut self, parent: Option<String>) -> Result<Option<String>, JsError> {
        Ok(self.scene.paste_node(opt_uid(parent)?).map_err(js_err)?.map(|u| u.to_string()))
    }

    pub fn has_clipboard(&self) -> bool {
        self.scene.has_clipboard()
    }

    pub fn change_kind(&mut self, uid: &str, kind: &str) -> Result<(), JsError> {
        self.scene.change_kind(parse_uid(uid)?, kind).map_err(js_err)
    }

    /// Fecha o agrupamento de alterações (soltar o mouse, sair de um campo numérico).
    pub fn end_group(&mut self) {
        self.scene.end_group();
    }

    pub fn undo(&mut self) -> Option<String> {
        self.scene.undo()
    }

    pub fn redo(&mut self) -> Option<String> {
        self.scene.redo()
    }

    // ---------- Reprodução ----------

    /// Inicia a reprodução a partir de uma cópia da cena de edição, no tempo zero.
    /// A física é montada com as posições atuais dos nós.
    /// Os players com autoplay e os timers com autostart começam aqui.
    pub fn play_start(&mut self) {
        let scene = self.scene.play_copy();
        let physics = Physics::build(&scene, &self.settings);
        let mut audio = AudioPlayback::default();
        audio.autoplay(&scene);
        let mut timers = Timers::default();
        timers.autostart(&scene);
        self.play = Some(Play { scene, time: 0.0, physics, audio, timers });
    }

    pub fn play_stop(&mut self) {
        self.play = None;
    }

    pub fn is_playing(&self) -> bool {
        self.play.is_some()
    }

    /// Avança a reprodução em `dt` segundos: o relógio, a física (passos fixos), os temporizadores e a
    /// atenuação dos sons 2D pela posição da câmera.
    pub fn play_tick(&mut self, dt: f64) {
        if let Some(p) = &mut self.play {
            p.time += dt;
            p.physics.advance(&mut p.scene, dt);
            p.timers.advance(&p.scene, dt);
            p.audio.update(&p.scene, p.time);
        }
    }

    /// Comandos para o motor de áudio desde a última chamada, em JSON (contrato em `nexo_core::audio`).
    /// A fila é esvaziada. Fora da reprodução devolve uma lista vazia.
    pub fn audio_commands(&mut self) -> Result<String, JsError> {
        let commands = match &mut self.play {
            Some(p) => p.audio.drain(),
            None => Vec::new(),
        };
        to_json(&commands)
    }

    /// O motor avisa que o som do nó terminou (ou não pôde tocar). Sem isso, um Play sem reinício não tocaria
    /// o som de novo (ver `AudioPlayback::finished`).
    pub fn audio_finished(&mut self, node: &str) -> Result<(), JsError> {
        let uid = parse_uid(node)?;
        if let Some(p) = &mut self.play {
            p.audio.finished(uid);
        }
        Ok(())
    }

    /// Sinais dos temporizadores desde a última chamada, em JSON: [{node, kind}] (SCN-014). A fila é esvaziada.
    pub fn timer_events(&mut self) -> Result<String, JsError> {
        let events: Vec<TimerEvent> = match &mut self.play {
            Some(p) => p.timers.take_events(),
            None => Vec::new(),
        };
        to_json(&events)
    }

    pub fn play_time(&self) -> f64 {
        self.play.as_ref().map_or(0.0, |p| p.time)
    }

    /// Eventos de física desde a última chamada, em JSON: [{kind, node, other}]. A fila é esvaziada.
    pub fn physics_events(&mut self) -> Result<String, JsError> {
        let events = match &mut self.play {
            Some(p) => p.physics.take_events(),
            None => Vec::new(),
        };
        to_json(&events)
    }

    /// Velocidade de um CharacterBody2D durante a reprodução: [x, y] em pixels por segundo.
    pub fn character_velocity(&self, uid: &str) -> Result<Vec<f64>, JsError> {
        let uid = parse_uid(uid)?;
        Ok(self.play.as_ref().and_then(|p| p.physics.velocity(uid)).map(|v| v.to_vec()).unwrap_or_default())
    }

    /// Define a velocidade de um CharacterBody2D durante a reprodução. Retorna false se não há.
    pub fn set_character_velocity(&mut self, uid: &str, x: f64, y: f64) -> Result<bool, JsError> {
        let uid = parse_uid(uid)?;
        Ok(self.play.as_mut().is_some_and(|p| p.physics.set_velocity(uid, [x, y])))
    }

    /// True se o personagem está no chão no último passo da física.
    pub fn is_grounded(&self, uid: &str) -> Result<bool, JsError> {
        let uid = parse_uid(uid)?;
        Ok(self.play.as_ref().is_some_and(|p| p.physics.grounded(uid)))
    }

    /// Estado de contato de um CharacterBody2D (chão, parede, teto, ângulo do chão, velocidades) em JSON,
    /// ou "null" se o nó não é um personagem da reprodução (PHY-004).
    pub fn character_state(&self, uid: &str) -> Result<String, JsError> {
        let uid = parse_uid(uid)?;
        let state = self.play.as_ref().and_then(|p| p.physics.character_state(uid));
        to_json(&state)
    }

    // ---------- Consultas de física (PHY-030) ----------
    // Resultados em JSON: {hit, node, point, normal, distance}, com null onde não há acerto.
    // Sem reprodução, as consultas não acertam nada. A máscara usa os bits das camadas (-1 = todas).

    /// Resultado do RayCast2D `uid` agora. `null` se o nó não é um RayCast2D.
    pub fn ray_hit(&self, uid: &str) -> Result<String, JsError> {
        let uid = parse_uid(uid)?;
        let hit = self.play.as_ref().and_then(|p| p.physics.ray_hit(&p.scene, uid));
        to_json(&hit)
    }

    /// Resultado do ShapeCast2D `uid` agora. `null` se o nó não é um ShapeCast2D.
    pub fn shape_hit(&self, uid: &str) -> Result<String, JsError> {
        let uid = parse_uid(uid)?;
        let hit = self.play.as_ref().and_then(|p| p.physics.shape_hit(&p.scene, uid));
        to_json(&hit)
    }

    /// Raio sem nó: o primeiro colisor atingido a partir de (ox, oy) na direção (dx, dy).
    pub fn raycast(&self, ox: f64, oy: f64, dx: f64, dy: f64, distance: f64, mask: u32, com_areas: bool) -> Result<String, JsError> {
        let hit = match &self.play {
            Some(p) => p.physics.raycast([ox, oy], [dx, dy], distance, mask, com_areas),
            None => Default::default(),
        };
        to_json(&hit)
    }

    /// UIDs dos nós com algum colisor que contém o ponto (x, y), em JSON.
    pub fn overlap_point(&self, x: f64, y: f64, mask: u32, com_areas: bool) -> Result<String, JsError> {
        let nos = self.play.as_ref().map(|p| p.physics.overlap_point([x, y], mask, com_areas)).unwrap_or_default();
        to_json(&nos)
    }

    /// UIDs dos nós com algum colisor que toca o retângulo de centro (x, y) e tamanho (w, h), em JSON.
    pub fn overlap_rect(&self, x: f64, y: f64, w: f64, h: f64, mask: u32, com_areas: bool) -> Result<String, JsError> {
        let nos = self.play.as_ref().map(|p| p.physics.overlap_rect([x, y], [w, h], mask, com_areas)).unwrap_or_default();
        to_json(&nos)
    }

    // ---------- Entrada (INP-001, INP-002) ----------

    /// Registra uma tecla (código de `KeyboardEvent.code`) pressionada ou solta.
    pub fn key_event(&mut self, code: &str, down: bool) {
        self.input.set_key(code, down);
    }

    /// Solta todas as teclas. Chamado quando a janela perde o foco.
    pub fn release_keys(&mut self) {
        self.input.release_all();
    }

    /// True se alguma tecla da ação está pressionada.
    pub fn action_pressed(&self, action: &str) -> bool {
        self.input.action_pressed(&self.settings.input, action)
    }

    /// -1, 0 ou 1: negativo com `neg` pressionada, positivo com `pos`.
    pub fn axis(&self, neg: &str, pos: &str) -> f64 {
        self.input.axis(&self.settings.input, neg, pos)
    }

    /// Vetor [x, y] das quatro ações, com comprimento no máximo 1.
    pub fn input_vector(&self, left: &str, right: &str, up: &str, down: &str) -> Vec<f64> {
        self.input.vector(&self.settings.input, left, right, up, down).to_vec()
    }

    /// Mapa de ações em RON, para salvar no projeto.
    pub fn input_map_ron(&self) -> Result<String, JsError> {
        self.settings.input.to_ron().map_err(js_err)
    }

    /// Substitui o mapa de ações por um texto RON.
    pub fn set_input_map_ron(&mut self, text: &str) -> Result<(), JsError> {
        self.settings.input = InputMap::from_ron(text).map_err(js_err)?;
        Ok(())
    }

    /// Configurações do projeto (settings.ron) em RON.
    pub fn settings_ron(&self) -> Result<String, JsError> {
        self.settings.to_ron().map_err(js_err)
    }

    /// Carrega as configurações do projeto a partir de settings.ron. Texto vazio usa os padrões.
    pub fn set_settings_ron(&mut self, text: &str) -> Result<(), JsError> {
        self.settings = ProjectSettings::from_ron(text).map_err(js_err)?;
        Ok(())
    }

    /// Configurações do projeto em JSON (os mesmos nomes de campo do RON). É o formato do TypeScript.
    pub fn settings_json(&self) -> Result<String, JsError> {
        self.settings.to_json().map_err(js_err)
    }

    /// Substitui as configurações do projeto por um JSON. Texto vazio usa os padrões.
    pub fn set_settings_json(&mut self, text: &str) -> Result<(), JsError> {
        self.settings = ProjectSettings::from_json(text).map_err(js_err)?;
        Ok(())
    }

    /// Religa a ação só a esta tecla, em tempo de execução (INP-005). Vale para o jogo em curso.
    pub fn rebind(&mut self, action: &str, key: &str) {
        self.settings.input.rebind(action, key);
    }

    // ---------- Consultas (seguem o modo atual: edição ou reprodução) ----------

    fn view(&self) -> (&Scene, Option<f64>) {
        match &self.play {
            Some(p) => (&p.scene, Some(p.time)),
            None => (&self.scene, None),
        }
    }

    /// Árvore inteira em JSON: nós em ordem, transformações mundiais, propriedades completas,
    /// e o estado do histórico (desfazer e refazer).
    pub fn outline(&self) -> Result<String, JsError> {
        let (s, t) = self.view();
        to_json(&query::outline(s, t))
    }

    /// UID do sprite de cima sob o ponto do mundo, ou undefined.
    pub fn pick(&self, x: f64, y: f64) -> Option<String> {
        let (s, t) = self.view();
        query::pick(s, x, y, t).map(|u| u.to_string())
    }

    /// Limites alinhados aos eixos [minx, miny, maxx, maxy] do nó. Vazio se o nó não existe.
    pub fn bounds(&self, uid: &str) -> Result<Vec<f64>, JsError> {
        let (s, t) = self.view();
        Ok(query::bounds(s, parse_uid(uid)?, t).map(|b| b.to_vec()).unwrap_or_default())
    }

    /// Câmera atual: [x, y, zoom]. Vazio se a cena não tem câmera.
    pub fn camera(&self) -> Vec<f64> {
        let (s, t) = self.view();
        query::current_camera(s, t).map(|(x, y, z)| vec![x, y, z]).unwrap_or_default()
    }

    /// Sprites para o viewport: 12 floats por sprite, na ordem de desenho.
    /// `selected_json` é uma lista JSON de UIDs; `hovered` é o UID sob o mouse.
    /// O campo `_pad` vem zerado: o TypeScript preenche com o slot da textura.
    pub fn sprites(&self, selected_json: &str, hovered: Option<String>) -> Result<Vec<f32>, JsError> {
        let selected: Vec<String> = serde_json::from_str(selected_json).map_err(js_err)?;
        let selected: Vec<Uid> = selected.iter().map(|s| parse_uid(s)).collect::<Result<_, _>>()?;
        let hovered = opt_uid(hovered)?;
        let (s, t) = self.view();
        let draws: Vec<Draw> = query::draws(s, t, &selected, hovered);
        Ok(query::floats(&draws))
    }

    /// Contornos das formas de colisão, em JSON: [{uid, owner, points: [[x, y], ...]}] (VPT-005).
    pub fn shape_outlines(&self) -> Result<String, JsError> {
        let (s, t) = self.view();
        to_json(&query::shape_outlines(s, t))
    }

    /// Textura de cada sprite, na mesma ordem de `sprites()` (JSON: UID ou "" sem textura).
    pub fn sprite_textures(&self) -> Result<String, JsError> {
        let (s, t) = self.view();
        let names: Vec<String> = query::draws(s, t, &[], None)
            .iter()
            .map(|d| d.texture.map(|u| u.to_string()).unwrap_or_default())
            .collect();
        to_json(&names)
    }
}
