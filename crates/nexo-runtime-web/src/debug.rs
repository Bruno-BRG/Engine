//! Depuração no editor (DBG-004): dados do desenho de física para o viewport. O método só lê a
//! reprodução atual; a alternância de exibição fica no TypeScript.
//!
//! Script debugger bridge (VIS-031, DBG-003). The methods only read or change the current play session, and the
//! TypeScript decides when to call them. Breakpoints and the followed instance outlive a single play session.

use crate::editor::{SceneEditor, js_err, to_json};
use crate::host::Mundo;
use crate::scripts::{self, Instancia, registrar};
use nexo_core::Uid;
use nexo_graph::{Host, TraceEvent, VmError};
use serde_json::{Value as Json, json};
use std::collections::{BTreeMap, BTreeSet};
use wasm_bindgen::prelude::*;

/// Trace event as JSON: {"Executou": {evento, node}} or {"Valor": {node, valor}}, as the TypeScript reads it.
fn evento_json(e: &TraceEvent) -> Json {
    match e {
        TraceEvent::Executou { evento, node } => json!({"Executou": {"evento": evento, "node": node}}),
        TraceEvent::Valor { node, valor } => json!({"Valor": {"node": node, "valor": valor}}),
    }
}

#[wasm_bindgen]
impl SceneEditor {
    /// Formas, contatos e velocidades da física agora, em JSON: {shapes, contacts, velocities}, em pixels do
    /// mundo. `null` fora da reprodução, porque só a reprodução tem física.
    pub fn debug_draw_json(&self) -> Result<String, JsError> {
        let draw = self.play_physics().map(|p| p.debug_draw());
        to_json(&draw)
    }

    /// Breakpoints per instance, as JSON: {"<node UID>": [ids of the script's nodes]}. Replaces the previous ones.
    /// Each instance gets the breakpoints of its own UID, so a script on two nodes has two breakpoint sets (VIS-031).
    pub fn set_breakpoints(&mut self, json: &str) -> Result<(), JsError> {
        let mapa: BTreeMap<String, Vec<u64>> = serde_json::from_str(json).map_err(js_err)?;
        let mut pontos = BTreeMap::new();
        for (uid, nos) in mapa {
            pontos.insert(uid.parse::<Uid>().map_err(js_err)?, nos.into_iter().collect::<BTreeSet<u64>>());
        }
        self.pontos = pontos;
        self.atualizar_depuracao();
        Ok(())
    }

    /// Chooses the instance the debugger follows. Only it may record a trace (DBG-003). An empty string clears the choice.
    pub fn debug_select(&mut self, uid: &str) -> Result<(), JsError> {
        self.selecionada = if uid.is_empty() { None } else { Some(uid.parse::<Uid>().map_err(js_err)?) };
        self.atualizar_depuracao();
        Ok(())
    }

    /// Tells the runtime whether the Depurador panel is open. An open panel lets the followed instance record a trace.
    pub fn debug_panel(&mut self, aberto: bool) {
        self.painel_aberto = aberto;
        self.atualizar_depuracao();
    }

    /// Trace of the followed instance since the last call, as JSON (a list of events, see `evento_json`). Empties the
    /// queue. An empty list without a play session or without a followed instance.
    pub fn debug_trace(&mut self) -> Result<String, JsError> {
        let escolhida = self.selecionada;
        let eventos: Vec<Json> = match (self.play.as_mut(), escolhida) {
            (Some(p), Some(uid)) => match p.instancias.iter_mut().find(|i| i.dono == uid) {
                Some(inst) => inst.tomar_rastro().iter().map(evento_json).collect(),
                None => Vec::new(),
            },
            _ => Vec::new(),
        };
        to_json(&eventos)
    }

    /// State of the instances of the play session, as JSON: [{uid, pausada, node, evento}]. `pausada` is also true at
    /// a frame-end pause, where `node` and `evento` are null. An empty list without a play session.
    pub fn debug_state(&self) -> Result<String, JsError> {
        let estado: Vec<Json> = match &self.play {
            Some(p) => p
                .instancias
                .iter()
                .map(|i| {
                    let pausa = i.pausa();
                    json!({
                        "uid": i.dono.to_string(),
                        "pausada": i.pausada(),
                        "node": pausa.map(|x| x.node),
                        "evento": pausa.map(|x| x.evento.as_str()),
                    })
                })
                .collect(),
            None => Vec::new(),
        };
        to_json(&estado)
    }

    /// Continuar (VIS-031): instance `uid` leaves the pause, runs the rest of the frame once, and the next frames up to
    /// the next pause. The other instances do not change. A script error goes to the console.
    pub fn debug_continue(&mut self, uid: &str) -> Result<(), JsError> {
        self.depurar(uid, |inst, host| inst.continuar(host))
    }

    /// Próxima operação (VIS-031): instance `uid` runs one operation and stops before the next, even when that one
    /// belongs to the next event or frame. A script error goes to the console.
    pub fn debug_step(&mut self, uid: &str) -> Result<(), JsError> {
        self.depurar(uid, |inst, host| inst.passo(host))
    }
}

impl SceneEditor {
    /// Applies the breakpoints and the trace policy to the instances of the play session, if there is one.
    fn atualizar_depuracao(&mut self) {
        if let Some(p) = &mut self.play {
            scripts::aplicar_pontos(&mut p.instancias, &self.pontos);
            scripts::atualizar_rastro(&mut p.instancias, self.selecionada, self.painel_aberto);
        }
    }

    /// Runs a debugger action on instance `uid`. An execution error goes to the console, as in regular frames.
    fn depurar(
        &mut self,
        uid: &str,
        acao: impl FnOnce(&mut Instancia, &mut dyn Host) -> Result<(), VmError>,
    ) -> Result<(), JsError> {
        let uid = uid.parse::<Uid>().map_err(js_err)?;
        let Some(p) = self.play.as_mut() else {
            return Ok(());
        };
        let Some(inst) = p.instancias.iter_mut().find(|i| i.dono == uid) else {
            return Ok(());
        };
        let resultado = {
            let mut mundo = Mundo {
                cena: &mut p.scene,
                fisica: &mut p.physics,
                audio: &mut p.audio,
                entrada: &self.input,
                ajustes: &self.settings,
                ponteiro: self.pointer,
                console: &mut self.console,
                dono: uid,
            };
            let host: &mut dyn Host = &mut mundo;
            acao(inst, host)
        };
        registrar(inst, &p.scene, &mut self.console, resultado);
        self.atualizar_depuracao();
        Ok(())
    }
}
