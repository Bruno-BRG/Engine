//! Temporizadores (SCN-014): o nó `Timer` conta `wait_time` segundos e emite o sinal `timeout`.
//!
//! Como o áudio, o estado existe só durante a reprodução: a cena de edição não muda. Os sinais vão para
//! uma fila que o WASM entrega ao TypeScript (`SceneEditor::timer_events`), e a lógica visual os lerá
//! quando o grafo se conectar.

use crate::scene::{Scene, Uid, value_of};
use serde::Serialize;
use std::collections::BTreeMap;

/// Tipo de nó do temporizador.
pub const TIMER: &str = "Timer";
/// Menor espera aceita, em segundos. Evita um laço sem fim com espera zero.
pub const MIN_WAIT: f64 = 0.01;
/// Sinais no máximo por timer e por quadro. Um quadro muito longo não gera uma rajada: o atraso perdido é
/// descartado.
const MAX_FIRES_PER_STEP: usize = 16;

/// Sinal emitido por um timer. `kind` é o nome do sinal (hoje só "timeout").
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct TimerEvent {
    pub node: Uid,
    pub kind: &'static str,
}

/// Timers em contagem e a fila de sinais da reprodução.
#[derive(Debug, Default)]
pub struct Timers {
    /// Timers em contagem, com o tempo acumulado desde o último sinal.
    running: BTreeMap<Uid, f64>,
    events: Vec<TimerEvent>,
}

impl Timers {
    /// Começa a contar o timer `uid`, do zero. Retorna false se o nó não existe ou não é um Timer.
    pub fn start(&mut self, scene: &Scene, uid: Uid) -> bool {
        if scene.node(uid).is_some_and(|n| n.kind == TIMER) {
            self.running.insert(uid, 0.0);
            true
        } else {
            false
        }
    }

    /// Para o timer. Não emite sinal.
    pub fn stop(&mut self, uid: Uid) {
        self.running.remove(&uid);
    }

    pub fn is_running(&self, uid: Uid) -> bool {
        self.running.contains_key(&uid)
    }

    /// Começa os timers com `autostart` ligado. Chamado ao iniciar a reprodução.
    pub fn autostart(&mut self, scene: &Scene) {
        for uid in scene.pre_order() {
            let Some(node) = scene.node(uid) else {
                continue;
            };
            if node.kind == TIMER && value_of(node, "autostart").as_bool().unwrap_or(false) {
                self.start(scene, uid);
            }
        }
    }

    /// Avança os timers em `dt` segundos. Cada vez que a espera acaba, empilha um sinal `timeout`. Timer de
    /// disparo único (`one_shot`) para no primeiro sinal.
    pub fn advance(&mut self, scene: &Scene, dt: f64) {
        let uids: Vec<Uid> = self.running.keys().copied().collect();
        for uid in uids {
            let Some(node) = scene.node(uid).filter(|n| n.kind == TIMER) else {
                self.running.remove(&uid);
                continue;
            };
            let Some(&previous) = self.running.get(&uid) else {
                continue;
            };
            let wait = value_of(node, "wait_time")
                .as_f64()
                .unwrap_or(1.0)
                .max(MIN_WAIT);
            let one_shot = value_of(node, "one_shot").as_bool().unwrap_or(false);

            let mut elapsed = previous + dt;
            let mut fires = 0;
            let mut stop = false;
            while elapsed >= wait && fires < MAX_FIRES_PER_STEP {
                elapsed -= wait;
                fires += 1;
                self.events.push(TimerEvent {
                    node: uid,
                    kind: "timeout",
                });
                if one_shot {
                    stop = true;
                    break;
                }
            }
            if fires == MAX_FIRES_PER_STEP {
                elapsed %= wait;
            }
            if stop {
                self.running.remove(&uid);
            } else {
                self.running.insert(uid, elapsed);
            }
        }
    }

    /// Entrega os sinais desde a última chamada e esvazia a fila.
    pub fn take_events(&mut self) -> Vec<TimerEvent> {
        std::mem::take(&mut self.events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cena_com_timer(wait: f64, one_shot: bool) -> (Scene, Uid) {
        let mut s = Scene::new("Teste", 5);
        let t = s.add_node(None, TIMER, None).unwrap();
        s.set_prop(t, "wait_time", json!(wait), None).unwrap();
        s.set_prop(t, "one_shot", json!(one_shot), None).unwrap();
        (s, t)
    }

    #[test]
    fn timer_dispara_a_cada_espera_sem_disparo_unico() {
        let (s, t) = cena_com_timer(0.5, false);
        let mut timers = Timers::default();
        assert!(timers.start(&s, t));
        timers.advance(&s, 0.3);
        assert!(timers.take_events().is_empty());
        timers.advance(&s, 0.3);
        assert_eq!(
            timers.take_events(),
            vec![TimerEvent {
                node: t,
                kind: "timeout"
            }]
        );
        // Continua contando, com o resto do tempo acumulado.
        timers.advance(&s, 0.5);
        assert_eq!(timers.take_events().len(), 1);
        assert!(timers.is_running(t));
    }

    #[test]
    fn timer_de_disparo_unico_para_no_primeiro_sinal() {
        let (s, t) = cena_com_timer(0.2, true);
        let mut timers = Timers::default();
        timers.start(&s, t);
        timers.advance(&s, 0.5);
        assert_eq!(timers.take_events().len(), 1);
        assert!(!timers.is_running(t));
        timers.advance(&s, 1.0);
        assert!(timers.take_events().is_empty());
    }

    #[test]
    fn autostart_inicia_so_os_timers_marcados() {
        let mut s = Scene::new("Teste", 5);
        let on = s.add_node(None, TIMER, None).unwrap();
        s.set_prop(on, "autostart", json!(true), None).unwrap();
        let off = s.add_node(None, TIMER, None).unwrap();
        let mut timers = Timers::default();
        timers.autostart(&s);
        assert!(timers.is_running(on));
        assert!(!timers.is_running(off));
    }

    #[test]
    fn start_recusa_no_que_nao_e_timer() {
        let s = Scene::new_default("Teste", 5);
        let raiz = s.roots()[0];
        let mut timers = Timers::default();
        assert!(!timers.start(&s, raiz));
        assert!(!timers.start(&s, Uid(0xdead)));
    }

    #[test]
    fn stop_para_sem_emitir_sinal() {
        let (s, t) = cena_com_timer(0.1, false);
        let mut timers = Timers::default();
        timers.start(&s, t);
        timers.stop(t);
        timers.advance(&s, 1.0);
        assert!(timers.take_events().is_empty());
    }

    #[test]
    fn quadro_longo_nao_gera_rajada_sem_fim() {
        let (s, t) = cena_com_timer(0.01, false);
        let mut timers = Timers::default();
        timers.start(&s, t);
        timers.advance(&s, 10.0);
        assert_eq!(timers.take_events().len(), MAX_FIRES_PER_STEP);
        assert!(timers.is_running(t));
    }

    #[test]
    fn sinal_serializa_com_node_e_kind() {
        let ev = TimerEvent {
            node: Uid(0xab),
            kind: "timeout",
        };
        assert_eq!(
            serde_json::to_string(&ev).unwrap(),
            r#"{"node":"00000000000000ab","kind":"timeout"}"#
        );
    }
}
