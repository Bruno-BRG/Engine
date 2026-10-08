//! Mapa de ações de entrada (INP-001, INP-002). O jogo consulta ações ("ui_left", "jump"), não
//! teclas: a mesma ação aceita várias teclas e pode ser religada sem mudar o código.
//! As teclas são os códigos de `KeyboardEvent.code` ("KeyA", "ArrowLeft", "Space"), que não mudam
//! com o layout do teclado.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InputMap {
    /// Ação para as teclas que a acionam.
    pub actions: BTreeMap<String, Vec<String>>,
}

impl Default for InputMap {
    fn default() -> Self {
        Self::defaults()
    }
}

impl InputMap {
    /// Ações padrão dos projetos novos (INP-002): setas e WASD para mover, Enter e Espaço para confirmar.
    pub fn defaults() -> InputMap {
        let pairs: [(&str, &[&str]); 6] = [
            ("ui_left", &["ArrowLeft", "KeyA"]),
            ("ui_right", &["ArrowRight", "KeyD"]),
            ("ui_up", &["ArrowUp", "KeyW"]),
            ("ui_down", &["ArrowDown", "KeyS"]),
            ("ui_accept", &["Enter", "Space"]),
            ("ui_cancel", &["Escape"]),
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

/// Teclas pressionadas no instante atual. O editor alimenta com os eventos de teclado da janela
/// enquanto a reprodução roda.
#[derive(Clone, Debug, Default)]
pub struct Input {
    down: BTreeSet<String>,
}

impl Input {
    pub fn set_key(&mut self, key: &str, down: bool) {
        if down {
            self.down.insert(key.to_string());
        } else {
            self.down.remove(key);
        }
    }

    /// Solta todas as teclas. Usado quando a janela perde o foco, para não ficar "preso".
    pub fn release_all(&mut self) {
        self.down.clear();
    }

    pub fn is_key_down(&self, key: &str) -> bool {
        self.down.contains(key)
    }

    /// True se alguma tecla ligada à ação está pressionada.
    pub fn action_pressed(&self, map: &InputMap, action: &str) -> bool {
        map.keys(action).iter().any(|k| self.down.contains(k))
    }

    /// -1, 0 ou 1: negativo se a ação `neg` está pressionada, positivo se `pos` está.
    pub fn axis(&self, map: &InputMap, neg: &str, pos: &str) -> f64 {
        let n = self.action_pressed(map, neg);
        let p = self.action_pressed(map, pos);
        match (n, p) {
            (true, false) => -1.0,
            (false, true) => 1.0,
            _ => 0.0,
        }
    }

    /// Vetor das quatro ações, com comprimento no máximo 1 (diagonal não é mais rápida).
    pub fn vector(&self, map: &InputMap, left: &str, right: &str, up: &str, down: &str) -> [f64; 2] {
        let x = self.axis(map, left, right);
        let y = self.axis(map, up, down);
        let len = (x * x + y * y).sqrt();
        if len > 1.0 { [x / len, y / len] } else { [x, y] }
    }
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
    fn soltar_tudo_limpa_o_estado() {
        let map = InputMap::defaults();
        let mut input = Input::default();
        input.set_key("Space", true);
        input.release_all();
        assert!(!input.action_pressed(&map, "ui_accept"));
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
        let text = map.to_ron().unwrap();
        assert_eq!(InputMap::from_ron(&text).unwrap(), map);
    }
}
