//! Scripts visuais na reprodução (VIS-023, CON-004). Cada nó com grafo ganha uma instância: o grafo compilado, um
//! agendador e um contexto. `set_scripts` guarda os grafos (RON) por UID de nó. `play_start` os compila, e cada
//! quadro (`tick`) roda os eventos pelo Host (`host.rs`). Na reprodução, um `set_scripts` recarrega a quente os grafos
//! alterados: as variáveis de mesmo nome e tipo mantêm o valor (`Scheduler::recarregar`).

use crate::editor::{Play, SceneEditor, js_err};
use crate::host::{Mensagem, Mundo, Ponteiro};
use nexo_core::{Input, ProjectSettings, Scene, Uid};
use nexo_graph::{Compiled, Contato, Context, Graph, Scheduler, compile};
use nexo_physics::{EventKind, PhysicsEvent, STEP};
use std::collections::{BTreeMap, BTreeSet};
use wasm_bindgen::prelude::*;

/// Uma instância de script: o grafo de um nó, com o agendador e o contexto dele.
pub(crate) struct Instancia {
    /// Nó dono do script: a saída "self" dos eventos e o nó das mensagens do console.
    pub(crate) dono: Uid,
    /// Texto RON que originou o grafo compilado. Quando muda, a instância é recarregada.
    texto: String,
    grafo: Compiled,
    agenda: Scheduler,
    contexto: Context,
    /// Erros de execução já escritos no console. Cada mensagem aparece uma vez, até a próxima recarga.
    erros: BTreeSet<String>,
}

#[wasm_bindgen]
impl SceneEditor {
    /// Guarda os grafos de script: um objeto JSON {"<uid do nó>": "<texto RON do grafo>"}, que substitui o anterior.
    /// Não compila aqui: um grafo com erro faz `play_start` falhar, com a mensagem do script. Na reprodução, os grafos
    /// alterados são recarregados na hora (recarga a quente); se um deles tem erro, o erro vai ao console e a versão
    /// anterior segue. Dá erro só para JSON ou UID inválidos.
    pub fn set_scripts(&mut self, json: &str) -> Result<(), JsError> {
        let mapa: BTreeMap<String, String> = serde_json::from_str(json).map_err(js_err)?;
        let mut textos = BTreeMap::new();
        for (uid, texto) in mapa {
            textos.insert(uid.parse::<Uid>().map_err(js_err)?, texto);
        }
        self.scripts = textos;
        if let Some(p) = &mut self.play {
            sincronizar(&mut p.instancias, &p.scene, &self.scripts, &mut self.console);
        }
        Ok(())
    }

    /// Mensagens do console desde a última chamada, em JSON: [{level, node, message}] (CON-004). `level` é "info",
    /// "warning" ou "error"; `node` é o UID do nó dono do script. A fila é esvaziada.
    pub fn console_messages(&mut self) -> Result<String, JsError> {
        let mensagens = std::mem::take(&mut self.console);
        serde_json::to_string(&mensagens).map_err(js_err)
    }

    /// Posição do ponteiro na reprodução: x e y em pixels do canvas, e a mesma posição no mundo, como o viewport a
    /// desenha. Os scripts leem em "Posição do mouse (mundo)" e "(tela)".
    pub fn pointer_event(&mut self, x: f64, y: f64, world_x: f64, world_y: f64) {
        self.pointer = Ponteiro { tela: [x, y], mundo: [world_x, world_y] };
    }
}

/// Compila os scripts de uma reprodução, na ordem da árvore. Se algum grafo tem erro, devolve todos, uma linha por
/// script, com o nome do nó. Sem erro, devolve as instâncias com o evento "Ao ficar pronto" agendado.
pub(crate) fn iniciar(cena: &Scene, textos: &BTreeMap<Uid, String>) -> Result<Vec<Instancia>, String> {
    let mut instancias = Vec::new();
    let mut erros = Vec::new();
    for uid in cena.pre_order() {
        let Some(texto) = textos.get(&uid) else {
            continue;
        };
        match Instancia::novo(uid, texto) {
            Ok(inst) => instancias.push(inst),
            Err(erro) => erros.push(format!("Script de '{}': {erro}", nome_de(cena, uid))),
        }
    }
    if erros.is_empty() { Ok(instancias) } else { Err(erros.join("\n")) }
}

/// Alinha as instâncias da reprodução com os grafos recebidos: remove as de nós sem script ou sem nó, recarrega as
/// de grafos alterados e cria as dos scripts novos. Erros de compilação vão ao console, e a instância anterior segue.
pub(crate) fn sincronizar(
    instancias: &mut Vec<Instancia>,
    cena: &Scene,
    textos: &BTreeMap<Uid, String>,
    console: &mut Vec<Mensagem>,
) {
    instancias.retain(|i| textos.contains_key(&i.dono) && cena.node(i.dono).is_some());
    for uid in cena.pre_order() {
        let Some(texto) = textos.get(&uid) else {
            continue;
        };
        match instancias.iter_mut().find(|i| i.dono == uid) {
            Some(inst) if inst.texto == *texto => {}
            Some(inst) => match compilar(texto) {
                Ok(novo) => inst.recarregar(novo, texto),
                Err(erro) => {
                    // O texto novo fica registrado: o mesmo erro não volta ao console a cada recarga.
                    inst.texto = texto.clone();
                    console.push(erro_no_console(cena, uid, format!("recarga falhou, segue a versão anterior: {erro}")));
                }
            },
            None => match Instancia::novo(uid, texto) {
                Ok(inst) => instancias.push(inst),
                Err(erro) => console.push(erro_no_console(cena, uid, erro)),
            },
        }
    }
}

/// Um quadro de todos os scripts da reprodução. Por instância, nesta ordem: um evento de colisão ("Ao colidir" ou
/// "Ao entrar um corpo") por evento de física que aconteceu com o nó dono, com dt zero (uma espera não conta o tempo
/// de novo); um "OnPhysicsProcess" por passo de física (com o passo fixo); o "OnProcess" do quadro, com o estado
/// ativo. Erro de execução vai ao console uma vez por mensagem; a instância segue.
pub(crate) fn tick(
    p: &mut Play,
    passos: usize,
    ajustes: &ProjectSettings,
    entrada: &Input,
    ponteiro: Ponteiro,
    console: &mut Vec<Mensagem>,
    dt: f64,
) {
    let eventos = std::mem::take(&mut p.eventos_quadro);
    p.instancias.retain(|i| p.scene.node(i.dono).is_some());
    for inst in &mut p.instancias {
        let mut resultado = Ok(());
        let dono = inst.dono;
        let colisoes = eventos
            .iter()
            .filter(|e| e.node == dono)
            .filter_map(|e| Some((evento_de_colisao(e.kind)?, 0.0, Some(contato_de(e)))));
        let fisica = (0..passos).map(|_| ("OnPhysicsProcess", STEP, None));
        let quadro = colisoes.chain(fisica).chain(std::iter::once(("OnProcess", dt, None)));
        for (evento, passo, contato) in quadro {
            let mut mundo = Mundo {
                cena: &mut p.scene,
                fisica: &mut p.physics,
                audio: &mut p.audio,
                entrada,
                ajustes,
                ponteiro,
                console: &mut *console,
                dono,
            };
            inst.agenda.iniciar(&inst.grafo, evento);
            if let Some(contato) = contato {
                inst.contexto.contato = contato;
            }
            if evento == "OnProcess" {
                inst.agenda.iniciar_estado(&inst.grafo, &inst.contexto.estado);
            }
            resultado = inst.agenda.tick(&inst.grafo, &mut inst.contexto, &mut mundo, passo);
            if resultado.is_err() {
                break;
            }
        }
        if let Err(erro) = resultado {
            let mensagem = erro.to_string();
            if inst.erros.insert(mensagem.clone()) {
                console.push(erro_no_console(&p.scene, inst.dono, mensagem));
            }
        }
    }
}

/// Evento de script que um evento de física dispara: "Ao colidir" para colisão sólida e "Ao entrar um corpo" para
/// área. Os outros eventos de física ainda não têm evento de script.
fn evento_de_colisao(kind: EventKind) -> Option<&'static str> {
    match kind {
        EventKind::CollisionStarted => Some("OnCollision"),
        EventKind::BodyEntered => Some("OnBodyEntered"),
        _ => None,
    }
}

/// Contato de um evento de física, como os eventos de colisão do script o leem. Área não tem ponto nem normal.
fn contato_de(e: &PhysicsEvent) -> Contato {
    Contato {
        outro: e.other.0,
        ponto: e.point.unwrap_or([0.0, 0.0]),
        normal: e.normal.unwrap_or([0.0, 0.0]),
    }
}

impl Instancia {
    /// Instância nova: compila o grafo e agenda o evento "Ao ficar pronto", que roda no primeiro quadro.
    fn novo(dono: Uid, texto: &str) -> Result<Instancia, String> {
        let grafo = compilar(texto)?;
        let mut agenda = Scheduler::new();
        agenda.iniciar(&grafo, "OnReady");
        let contexto = grafo.new_context(dono.0);
        Ok(Instancia { dono, texto: texto.to_string(), grafo, agenda, contexto, erros: BTreeSet::new() })
    }

    /// Recarga a quente: troca o grafo e mantém as variáveis de mesmo nome e tipo (`Scheduler::recarregar`). As
    /// tarefas em curso são descartadas. Um estado que deixou de existir no grafo novo volta ao estado inicial.
    fn recarregar(&mut self, novo: Compiled, texto: &str) {
        self.contexto = self.agenda.recarregar(&self.contexto, &self.grafo, &novo);
        self.grafo = novo;
        self.texto = texto.to_string();
        self.erros.clear();
        let estado = self.contexto.estado.clone();
        if !estado.is_empty() && !self.grafo.programs.iter().any(|p| p.estado.as_deref() == Some(estado.as_str())) {
            self.contexto.estado = self.grafo.estado_inicial.clone().unwrap_or_default();
        }
    }
}

/// Lê e compila um grafo. O erro junta as mensagens de leitura ou de validação, cada uma com o nó onde está.
fn compilar(texto: &str) -> Result<Compiled, String> {
    let grafo = Graph::from_ron(texto)?;
    compile(&grafo).map_err(|erros| erros.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; "))
}

fn nome_de(cena: &Scene, uid: Uid) -> String {
    cena.node(uid).map_or_else(|| uid.to_string(), |n| n.name.clone())
}

fn erro_no_console(cena: &Scene, dono: Uid, texto: String) -> Mensagem {
    Mensagem { level: "error", node: dono, message: format!("Script de '{}': {texto}", nome_de(cena, dono)) }
}
