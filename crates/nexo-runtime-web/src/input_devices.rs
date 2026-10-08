//! Entrada dos dispositivos no editor WASM (INP-001, INP-004): eixos do controle e joystick de toque.
//! Ficam neste arquivo, e não em editor.rs, porque outros trechos também editam aquele arquivo.
//! Os botões do mouse e do controle entram pelo `key_event`, pois são códigos como as teclas.

use nexo_core::scene::value_of;
use nexo_core::Uid;
use serde_json::Value;
use wasm_bindgen::prelude::*;

use crate::editor::SceneEditor;

#[wasm_bindgen]
impl SceneEditor {
    /// Valor de um eixo do controle em [-1, 1]: "gamepad_left_x", "gamepad_left_y", "gamepad_right_x" ou
    /// "gamepad_right_y". O TypeScript envia a cada quadro da reprodução. Nome desconhecido é ignorado.
    /// The triggers "gamepad_lt" and "gamepad_rt" are also accepted, unsigned, from 0 to 1 (INP-001).
    pub fn set_axis(&mut self, axis: &str, value: f64) {
        self.input.set_axis(axis, value);
    }

    /// Joystick de toque (INP-004): o polegar do nó VirtualJoystick `uid` em (x, y), de -1 a 1, com y para
    /// baixo. Empurra as ações de direção do próprio nó (`action_left` a `action_down`). Fora da reprodução,
    /// ou se o nó não é um VirtualJoystick, não faz nada.
    pub fn set_joystick(&mut self, uid: &str, x: f64, y: f64) -> Result<(), JsError> {
        let uid: Uid = uid.parse().map_err(|e: String| JsError::new(&e))?;
        let Some(play) = &self.play else { return Ok(()) };
        let Some(node) = play.scene.node(uid) else { return Ok(()) };
        if node.kind != "VirtualJoystick" {
            return Ok(());
        }
        let nome = |prop: &str| match value_of(node, prop) {
            Value::String(s) => s,
            _ => String::new(),
        };
        let acoes = [nome("action_left"), nome("action_right"), nome("action_up"), nome("action_down")];
        self.input.set_stick(acoes.each_ref().map(String::as_str), x, y);
        Ok(())
    }
}
