//! Depuração no editor (DBG-004): dados do desenho de física para o viewport. O método só lê a
//! reprodução atual; a alternância de exibição fica no TypeScript.

use crate::editor::to_json;
use crate::SceneEditor;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl SceneEditor {
    /// Formas, contatos e velocidades da física agora, em JSON: {shapes, contacts, velocities}, em pixels do
    /// mundo. `null` fora da reprodução, porque só a reprodução tem física.
    pub fn debug_draw_json(&self) -> Result<String, JsError> {
        let draw = self.play_physics().map(|p| p.debug_draw());
        to_json(&draw)
    }
}
