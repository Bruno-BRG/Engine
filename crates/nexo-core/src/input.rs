//! Mapa de ações de entrada (INP-001, INP-002, INP-004). O jogo consulta ações ("ui_left", "jump"), não
//! teclas: a mesma ação aceita vários códigos e pode ser religada sem mudar o código.
//!
//! Códigos que uma ação aceita:
//! - teclas: os de `KeyboardEvent.code` ("KeyA", "ArrowLeft", "Space"), que não mudam com o layout;
//! - botões do mouse: "mouse_left", "mouse_right", "mouse_middle";
//! - botões do controle: "gamepad_a" a "gamepad_back" e "dpad_up" a "dpad_right" (mapeamento padrão);
//! - eixos do controle: os de `GAMEPAD_AXES`. Com sinal, "gamepad_left_x-" é o lado negativo (esquerda,
//!   cima) e "gamepad_left_x+" o positivo (direita, baixo). Sem sinal, vale o módulo, para os dois lados.
//!
//! Um joystick de toque (INP-004) não tem código: empurra a força da ação direto (`Input::set_stick`).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Eixos analógicos do controle, pelo nome sem sinal. É o nome que o TypeScript envia (`set_axis`).
pub const GAMEPAD_AXES: [&str; 4] = ["gamepad_left_x", "gamepad_left_y", "gamepad_right_x", "gamepad_right_y"];

/// Módulo abaixo do qual o analógico conta como parado. Acima dele, a força volta de 0 a 1 sem salto.
pub const STICK_DEADZONE: f64 = 0.2;

/// Força mínima para `action_pressed`. Tecla e botão valem 1; um analógico precisa passar da metade.
pub const PRESS_THRESHOLD: f64 = 0.5;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InputMap {
    /// Ação para os códigos que a acionam.
    pub actions: BTreeMap<String, Vec<String>>,
}

impl Default for InputMap {
    fn default() -> Self {
        Self::defaults()
    }
}

impl InputMap {
    /// Ações padrão dos projetos novos (INP-002): setas, WASD e cruz do controle para mover; Enter, Espaço
    /// e A do controle para confirmar; Esc e B para cancelar.
    pub fn defaults() -> InputMap {
        let pairs: [(&str, &[&str]); 6] = [
            ("ui_left", &["ArrowLeft", "KeyA", "dpad_left"]),
            ("ui_right", &["ArrowRight", "KeyD", "dpad_right"]),
            ("ui_up", &["ArrowUp", "KeyW", "dpad_up"]),
            ("ui_down", &["ArrowDown", "KeyS", "dpad_down"]),
            ("ui_accept", &["Enter", "Space", "gamepad_a"]),
            ("ui_cancel", &["Escape", "gamepad_b"]),
        ];
        let actions = pairs
            .iter()
            .map(|(name, keys)| (name.to_string(), keys.iter().map(|k| k.to_string()).collect()))
            .collect();
        InputMap { actions }
    }

    /// Liga uma tecla à ação. Não duplica a tecla se ela já estiver ligada.
    pub fn bind(&mut self, action: &str, key: &str) {
        let keys = self.actions.entry(action.to_string()).or_default();
        if !keys.iter().any(|k| k == key) {
            keys.push(key.to_string());
        }
    }

    /// Desliga uma tecla da ação. Uma ação sem teclas continua existindo.
    pub fn unbind(&mut self, action: &str, key: &str) {
        if let Some(keys) = self.actions.get_mut(action) {
            keys.retain(|k| k != key);
        }
    }

    /// Religa a ação só a esta tecla (INP-005): as ligações anteriores da ação são trocadas e a
    /// ação é criada se não existia. Outras ações não mudam, mesmo que usem a mesma tecla.
    pub fn rebind(&mut self, action: &str, key: &str) {
        self.actions.insert(action.to_string(), vec![key.to_string()]);
    }

    /// Remove a ação inteira.
    pub fn remove_action(&mut self, action: &str) {
        self.actions.remove(action);
    }

    pub fn keys(&self, action: &str) -> &[String] {
        self.actions.get(action).map_or(&[], Vec::as_slice)
    }

    pub fn to_ron(&self) -> Result<String, String> {
        let cfg = ron::ser::PrettyConfig::new().struct_names(false);
        ron::ser::to_string_pretty(self, cfg).map_err(|e| e.to_string())
    }

    pub fn from_ron(text: &str) -> Result<InputMap, String> {
        ron::from_str(text).map_err(|e| format!("Mapa de entrada ilegível: {e}"))
    }
}

/// Entrada no instante atual. O editor alimenta com os eventos da janela (teclado, mouse e toque) e com
/// o estado do controle a cada quadro da reprodução.
#[derive(Clone, Debug, Default)]
pub struct Input {
    /// Códigos digitais pressionados: teclas, botões do mouse e botões do controle.
    down: BTreeSet<String>,
    /// Eixos do controle, em [-1, 1], pelo nome sem sinal (ver `GAMEPAD_AXES`).
    axes: BTreeMap<String, f64>,
    /// Força das ações vinda de joysticks de toque, em (0, 1].
    strength: BTreeMap<String, f64>,
}

impl Input {
    pub fn set_key(&mut self, key: &str, down: bool) {
        if down {
            self.down.insert(key.to_string());
        } else {
            self.down.remove(key);
        }
    }

    /// Valor de um eixo do controle, em [-1, 1]. Nome fora de `GAMEPAD_AXES` é ignorado; NaN vira 0.
    pub fn set_axis(&mut self, axis: &str, value: f64) {
        if GAMEPAD_AXES.contains(&axis) {
            let v = if value.is_nan() { 0.0 } else { value.clamp(-1.0, 1.0) };
            self.axes.insert(axis.to_string(), v);
        }
    }

    /// Força de uma ação vinda de um dispositivo virtual, em [0, 1]. Zero solta a ação.
    pub fn set_strength(&mut self, action: &str, value: f64) {
        let v = if value.is_nan() { 0.0 } else { value.clamp(0.0, 1.0) };
        if v > 0.0 {
            self.strength.insert(action.to_string(), v);
        } else {
            self.strength.remove(action);
        }
    }

    /// Joystick de toque (INP-004): o polegar em (x, y), com y crescendo para baixo. Cada lado do vetor vira
    /// força de uma ação, na ordem `actions` = [esquerda, direita, cima, baixo]. Nome vazio é ignorado.
    pub fn set_stick(&mut self, actions: [&str; 4], x: f64, y: f64) {
        let [left, right, up, down] = actions;
        for (name, v) in [(left, -x), (right, x), (up, -y), (down, y)] {
            if !name.is_empty() {
                self.set_strength(name, v);
            }
        }
    }

    /// Solta todas as teclas, botões, eixos e forças. Usado quando a janela perde o foco, para não ficar "preso".
    pub fn release_all(&mut self) {
        self.down.clear();
        self.axes.clear();
        self.strength.clear();
    }

    pub fn is_key_down(&self, key: &str) -> bool {
        self.down.contains(key)
    }

    /// Força da ação agora, de 0 a 1: o maior valor entre os códigos ligados a ela e a força virtual.
    pub fn action_value(&self, map: &InputMap, action: &str) -> f64 {
        let from_codes = map.keys(action).iter().map(|code| self.code_value(code)).fold(0.0, f64::max);
        from_codes.max(self.strength.get(action).copied().unwrap_or(0.0))
    }

    /// True se a força da ação chega a `PRESS_THRESHOLD`: alguma tecla ligada está pressionada, ou um eixo ligado
    /// passou do limiar no lado certo.
    pub fn action_pressed(&self, map: &InputMap, action: &str) -> bool {
        self.action_value(map, action) >= PRESS_THRESHOLD
    }

    /// -1 a 1: a força de `pos` menos a de `neg`. Com teclas, vale -1, 0 ou 1; com analógico, o valor no meio.
    pub fn axis(&self, map: &InputMap, neg: &str, pos: &str) -> f64 {
        (self.action_value(map, pos) - self.action_value(map, neg)).clamp(-1.0, 1.0)
    }

    /// Vetor das quatro ações, com comprimento no máximo 1 (diagonal não é mais rápida).
    pub fn vector(&self, map: &InputMap, left: &str, right: &str, up: &str, down: &str) -> [f64; 2] {
        let x = self.axis(map, left, right);
        let y = self.axis(map, up, down);
        let len = (x * x + y * y).sqrt();
        if len > 1.0 { [x / len, y / len] } else { [x, y] }
    }

    /// Força de um código de tecla, botão ou eixo.
    fn code_value(&self, code: &str) -> f64 {
        match parse_axis(code) {
            Some((axis, side)) => {
                let raw = self.axes.get(axis).copied().unwrap_or(0.0);
                let on_side = match side {
                    Some(true) => raw > 0.0,
                    Some(false) => raw < 0.0,
                    None => true,
                };
                if on_side { stick(raw) } else { 0.0 }
            }
            None if self.down.contains(code) => 1.0,
            None => 0.0,
        }
    }
}

/// Separa um código de eixo no nome do eixo e no lado: `Some(true)` para `+`, `Some(false)` para `-`,
/// `None` sem sinal. Devolve `None` se o código não é de eixo.
fn parse_axis(code: &str) -> Option<(&'static str, Option<bool>)> {
    GAMEPAD_AXES.iter().find_map(|&axis| {
        if code == axis {
            return Some((axis, None));
        }
        match code.strip_prefix(axis)? {
            "+" => Some((axis, Some(true))),
            "-" => Some((axis, Some(false))),
            _ => None,
        }
    })
}

/// Módulo de um analógico depois da zona morta, de 0 a 1.
fn stick(raw: f64) -> f64 {
    let m = raw.abs();
    if m <= STICK_DEADZONE { 0.0 } else { ((m - STICK_DEADZONE) / (1.0 - STICK_DEADZONE)).min(1.0) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padroes_ligam_setas_e_wasd() {
        let map = InputMap::defaults();
        assert!(map.keys("ui_left").contains(&"ArrowLeft".to_string()));
        assert!(map.keys("ui_left").contains(&"KeyA".to_string()));
        assert!(map.keys("ui_accept").contains(&"Space".to_string()));
        assert!(map.keys("acao_inexistente").is_empty());
    }

    #[test]
    fn padroes_incluem_a_cruz_e_os_botoes_a_e_b_do_controle() {
        // INP-002: as seis ações padrão existem e têm botões do controle.
        let map = InputMap::defaults();
        assert!(map.keys("ui_left").contains(&"dpad_left".to_string()));
        assert!(map.keys("ui_right").contains(&"dpad_right".to_string()));
        assert!(map.keys("ui_up").contains(&"dpad_up".to_string()));
        assert!(map.keys("ui_down").contains(&"dpad_down".to_string()));
        assert!(map.keys("ui_accept").contains(&"gamepad_a".to_string()));
        assert!(map.keys("ui_cancel").contains(&"gamepad_b".to_string()));
        for acao in ["ui_left", "ui_right", "ui_up", "ui_down", "ui_accept", "ui_cancel"] {
            assert!(!map.keys(acao).is_empty(), "{acao} sem códigos");
        }
    }

    #[test]
    fn botao_do_controle_aciona_a_acao_padrao() {
        let map = InputMap::defaults();
        let mut input = Input::default();
        input.set_key("dpad_left", true);
        assert!(input.action_pressed(&map, "ui_left"));
        assert_eq!(input.axis(&map, "ui_left", "ui_right"), -1.0);
        input.set_key("dpad_left", false);
        input.set_key("gamepad_a", true);
        assert!(input.action_pressed(&map, "ui_accept"));
        assert!(!input.action_pressed(&map, "ui_cancel"));
    }

    #[test]
    fn botao_do_mouse_e_codigo_como_tecla() {
        let mut map = InputMap::defaults();
        map.bind("shoot", "mouse_left");
        let mut input = Input::default();
        input.set_key("mouse_left", true);
        assert!(input.action_pressed(&map, "shoot"));
        input.set_key("mouse_left", false);
        assert!(!input.action_pressed(&map, "shoot"));
    }

    #[test]
    fn acao_acionada_por_qualquer_tecla_ligada() {
        let map = InputMap::defaults();
        let mut input = Input::default();
        assert!(!input.action_pressed(&map, "ui_left"));
        input.set_key("KeyA", true);
        assert!(input.action_pressed(&map, "ui_left"));
        input.set_key("KeyA", false);
        assert!(!input.action_pressed(&map, "ui_left"));
    }

    #[test]
    fn eixo_e_vetor_normalizado() {
        let map = InputMap::defaults();
        let mut input = Input::default();
        input.set_key("ArrowRight", true);
        assert_eq!(input.axis(&map, "ui_left", "ui_right"), 1.0);
        input.set_key("ArrowLeft", true);
        assert_eq!(input.axis(&map, "ui_left", "ui_right"), 0.0);
        input.set_key("ArrowLeft", false);
        input.set_key("ArrowUp", true);
        let v = input.vector(&map, "ui_left", "ui_right", "ui_up", "ui_down");
        let s = std::f64::consts::FRAC_1_SQRT_2;
        assert!((v[0] - s).abs() < 1e-9 && (v[1] + s).abs() < 1e-9, "{v:?}");
    }

    #[test]
    fn eixo_com_sinal_separa_os_dois_lados() {
        let mut map = InputMap::defaults();
        map.bind("ui_left", "gamepad_left_x-");
        map.bind("ui_right", "gamepad_left_x+");
        let mut input = Input::default();
        input.set_axis("gamepad_left_x", 0.8);
        // (0,8 - 0,2) / (1 - 0,2) = 0,75
        assert!((input.axis(&map, "ui_left", "ui_right") - 0.75).abs() < 1e-9);
        assert!(input.action_pressed(&map, "ui_right"));
        assert!(!input.action_pressed(&map, "ui_left"));
        input.set_axis("gamepad_left_x", -0.8);
        assert!((input.axis(&map, "ui_left", "ui_right") + 0.75).abs() < 1e-9);
        assert!(input.action_pressed(&map, "ui_left"));
        assert!(!input.action_pressed(&map, "ui_right"));
    }

    #[test]
    fn zona_morta_e_limiar_do_analogico() {
        let mut map = InputMap::defaults();
        map.bind("run", "gamepad_right_y");
        let mut input = Input::default();
        input.set_axis("gamepad_right_y", 0.15);
        assert_eq!(input.action_value(&map, "run"), 0.0);
        // 0,5 passa da zona morta, mas (0,5 - 0,2) / 0,8 = 0,375 fica abaixo do limiar de 0,5.
        input.set_axis("gamepad_right_y", 0.5);
        assert!((input.action_value(&map, "run") - 0.375).abs() < 1e-9);
        assert!(!input.action_pressed(&map, "run"));
        // Sem sinal, o módulo conta para os dois lados.
        input.set_axis("gamepad_right_y", -1.0);
        assert!((input.action_value(&map, "run") - 1.0).abs() < 1e-9);
        assert!(input.action_pressed(&map, "run"));
    }

    #[test]
    fn eixo_desconhecido_e_ignorado_e_nan_vira_zero() {
        let mut map = InputMap::defaults();
        map.bind("ui_right", "gamepad_left_x+");
        let mut input = Input::default();
        input.set_axis("gamepad_perna", 1.0);
        assert_eq!(input.action_value(&map, "ui_right"), 0.0);
        input.set_axis("gamepad_left_x", f64::NAN);
        assert_eq!(input.action_value(&map, "ui_right"), 0.0);
        input.set_axis("gamepad_left_x", 3.0);
        assert!((input.action_value(&map, "ui_right") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn joystick_de_toque_empurra_as_acoes_de_direcao() {
        // INP-004: o polegar em (x, y) vira força nas ações esquerda, direita, cima e baixo.
        let map = InputMap::defaults();
        let mut input = Input::default();
        let acoes = ["ui_left", "ui_right", "ui_up", "ui_down"];
        input.set_stick(acoes, 0.6, -0.8);
        assert!((input.axis(&map, "ui_left", "ui_right") - 0.6).abs() < 1e-9);
        assert!((input.axis(&map, "ui_up", "ui_down") + 0.8).abs() < 1e-9);
        assert!(input.action_pressed(&map, "ui_right"));
        assert!(input.action_pressed(&map, "ui_up"));
        assert!(!input.action_pressed(&map, "ui_left"));
        input.set_stick(acoes, 0.0, 0.0);
        assert_eq!(input.vector(&map, "ui_left", "ui_right", "ui_up", "ui_down"), [0.0, 0.0]);
    }

    #[test]
    fn forca_virtual_e_tecla_valem_o_maior() {
        let map = InputMap::defaults();
        let mut input = Input::default();
        input.set_strength("ui_right", 0.3);
        input.set_key("ArrowRight", true);
        assert_eq!(input.action_value(&map, "ui_right"), 1.0);
        input.set_key("ArrowRight", false);
        assert!((input.action_value(&map, "ui_right") - 0.3).abs() < 1e-9);
        input.set_strength("ui_right", 0.0);
        assert_eq!(input.action_value(&map, "ui_right"), 0.0);
    }

    #[test]
    fn soltar_tudo_limpa_o_estado() {
        let map = InputMap::defaults();
        let mut input = Input::default();
        input.set_key("Space", true);
        input.set_axis("gamepad_left_x", 1.0);
        input.set_stick(["ui_left", "ui_right", "ui_up", "ui_down"], 1.0, 0.0);
        input.release_all();
        assert!(!input.action_pressed(&map, "ui_accept"));
        assert_eq!(input.axis(&map, "ui_left", "ui_right"), 0.0);
    }

    #[test]
    fn religar_nao_duplica_e_desligar_remove() {
        let mut map = InputMap::defaults();
        map.bind("ui_accept", "Enter");
        assert_eq!(map.keys("ui_accept").iter().filter(|k| *k == "Enter").count(), 1);
        map.bind("jump", "KeyZ");
        assert_eq!(map.keys("jump"), ["KeyZ".to_string()]);
        map.unbind("jump", "KeyZ");
        assert!(map.keys("jump").is_empty());
    }

    #[test]
    fn mapa_ida_e_volta_ron() {
        let mut map = InputMap::defaults();
        map.bind("jump", "KeyZ");
        map.bind("shoot", "mouse_left");
        map.bind("run", "gamepad_left_x+");
        let text = map.to_ron().unwrap();
        assert_eq!(InputMap::from_ron(&text).unwrap(), map);
    }

    #[test]
    fn religar_troca_as_teclas_da_acao_e_cria_acao_nova() {
        let mut map = InputMap::defaults();
        map.bind("ui_accept", "KeyZ");
        map.rebind("ui_accept", "KeyX");
        assert_eq!(map.keys("ui_accept"), ["KeyX".to_string()]);
        map.rebind("jump", "KeyJ");
        assert_eq!(map.keys("jump"), ["KeyJ".to_string()]);
    }

    #[test]
    fn religar_nao_mexe_nas_outras_acoes_que_usam_a_mesma_tecla() {
        // Space é de ui_accept por padrão. Religar "jump" a Space não tira Space de ui_accept.
        let mut map = InputMap::defaults();
        map.rebind("jump", "Space");
        assert!(map.keys("ui_accept").contains(&"Space".to_string()));
        let mut input = Input::default();
        input.set_key("Space", true);
        assert!(input.action_pressed(&map, "jump"));
        assert!(input.action_pressed(&map, "ui_accept"));
    }

    #[test]
    fn religar_em_tempo_de_execucao_muda_quem_esta_pressionado() {
        let mut map = InputMap::defaults();
        let mut input = Input::default();
        map.rebind("jump", "KeyZ");
        input.set_key("KeyZ", true);
        assert!(input.action_pressed(&map, "jump"));
        map.rebind("jump", "KeyX");
        assert!(!input.action_pressed(&map, "jump"));
        input.set_key("KeyX", true);
        assert!(input.action_pressed(&map, "jump"));
    }

    #[test]
    fn mapa_ida_e_volta_json_com_nomes_de_acao() {
        let mut map = InputMap::defaults();
        map.bind("jump", "KeyZ");
        let json = serde_json::to_value(&map).unwrap();
        assert_eq!(json["actions"]["jump"][0], "KeyZ");
        let back: InputMap = serde_json::from_value(json).unwrap();
        assert_eq!(back, map);
    }
}
