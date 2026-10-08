//! Áudio do runtime (AUD-001, AUD-003, AUD-004): a fila de comandos que o jogo envia ao motor de áudio
//! do editor, a atenuação dos players 2D e os efeitos dos barramentos.
//!
//! # Contrato `AudioCommand`
//!
//! O núcleo não toca som. Durante a reprodução ele empilha comandos; o WASM os entrega como JSON a cada
//! quadro (`SceneEditor::audio_commands`), e o motor WebAudio do editor (`editor/src/lib/audio/`) executa
//! cada um. Cada comando é um objeto com `type` em snake_case e os demais campos em camelCase:
//!
//! - `play`: `{node, stream, volumeDb, pitch, bus, looping, fromStart}`. Começa o áudio `stream` (UID de
//!   asset do tipo "audio") no nó `node`. Com `fromStart` falso e o nó já tocando, o comando é ignorado.
//! - `stop`: `{node}`. Para o som do nó. Nó que não toca é ignorado.
//! - `fade`: `{node, toDb, seconds}`. Leva o volume do nó a `toDb` em `seconds` segundos.
//! - `set_bus_volume`: `{bus, volumeDb}`. Volume do barramento durante a reprodução.
//! - `set_bus_mute`: `{bus, mute}`. Mudo do barramento durante a reprodução.
//! - `set_attenuation`: `{node, db}`. Atenuação espacial de um player 2D, em dB (zero ou negativo). O
//!   próprio núcleo envia este comando a cada quadro; quem emite comandos pela lógica visual não precisa.
//!
//! Do motor de volta para o núcleo: `audio_finished(node)`, quando um som termina sozinho ou não pode
//! tocar. Sem ele, o núcleo acharia que o som ainda toca e um `play` com `fromStart` falso não o repetiria.
//!
//! Volumes são em decibéis: 0 é o volume original e -80 ou menos é silêncio. UIDs vão como texto de 16
//! dígitos hexadecimais. Os comandos de barramento valem só para a reprodução: o mixer grava o volume e o
//! mudo em `settings.ron`, e estes comandos não mexem no arquivo.

use crate::query;
use crate::scene::{Node, Scene, Uid, value_of};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// Barramento padrão, criado em todo projeto novo. Player sem barramento válido toca nele.
pub const DEFAULT_BUS: &str = "Master";
/// Volume de silêncio em dB. Abaixo disso o motor usa silêncio absoluto.
pub const SILENCE_DB: f64 = -80.0;
/// Variação mínima (dB) para reenviar a atenuação de um player 2D. Evita comandos a cada quadro.
const ATTENUATION_STEP_DB: f64 = 0.05;

/// Comando para o motor de áudio. Ver o contrato no topo do módulo.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum AudioCommand {
    Play {
        node: Uid,
        stream: Uid,
        volume_db: f64,
        pitch: f64,
        bus: String,
        looping: bool,
        from_start: bool,
    },
    Stop {
        node: Uid,
    },
    Fade {
        node: Uid,
        to_db: f64,
        seconds: f64,
    },
    SetBusVolume {
        bus: String,
        volume_db: f64,
    },
    SetBusMute {
        bus: String,
        mute: bool,
    },
    SetAttenuation {
        node: Uid,
        db: f64,
    },
}

/// Estado de áudio de uma reprodução: a fila de comandos e os sons que estão tocando.
///
/// Mora no `Play` do editor. O jogo empilha com `push`, `play_node` e `stop`; o WASM chama `update` a
/// cada quadro (depois da física) e `drain` para entregar a fila ao motor. Quando o motor termina um som
/// sozinho, avisa pelo WASM (`audio_finished`), que chega aqui como `finished`.
#[derive(Debug, Default)]
pub struct AudioPlayback {
    queue: Vec<AudioCommand>,
    /// Sons tocando, pelo UID do nó. O valor é a última atenuação enviada (None: ainda não enviada).
    voices: BTreeMap<Uid, Option<f64>>,
}

impl AudioPlayback {
    /// Empilha um comando. `Play` e `Stop` também atualizam a lista de sons tocando.
    pub fn push(&mut self, cmd: AudioCommand) {
        match &cmd {
            // Um novo Play reinicia a atenuação: o motor cria um som novo, sem atenuação enviada ainda.
            AudioCommand::Play { node, .. } => {
                self.voices.insert(*node, None);
            }
            AudioCommand::Stop { node } => {
                self.voices.remove(node);
            }
            _ => {}
        }
        self.queue.push(cmd);
    }

    /// Toca o player `uid` com os valores do próprio nó (AUD-001). Retorna false se o nó não existe, não é
    /// um player, não tem áudio ou, com `from_start` falso, já está tocando (o som segue sem reiniciar).
    pub fn play_node(&mut self, scene: &Scene, uid: Uid, from_start: bool) -> bool {
        let Some(node) = scene.node(uid) else {
            return false;
        };
        if !is_player(&node.kind) {
            return false;
        }
        if !from_start && self.voices.contains_key(&uid) {
            return false;
        }
        let Some(stream) = value_of(node, "stream")
            .as_str()
            .and_then(|s| s.parse::<Uid>().ok())
        else {
            return false;
        };
        let bus = value_of(node, "bus")
            .as_str()
            .filter(|b| !b.is_empty())
            .unwrap_or(DEFAULT_BUS)
            .to_string();
        self.push(AudioCommand::Play {
            node: uid,
            stream,
            volume_db: num(node, "volume_db", 0.0),
            pitch: num(node, "pitch_scale", 1.0),
            bus,
            looping: value_of(node, "loop").as_bool().unwrap_or(false),
            from_start,
        });
        true
    }

    /// Começa os players com `autoplay` ligado. Chamado ao iniciar a reprodução.
    pub fn autoplay(&mut self, scene: &Scene) {
        for uid in scene.pre_order() {
            let Some(node) = scene.node(uid) else {
                continue;
            };
            if is_player(&node.kind) && value_of(node, "autoplay").as_bool().unwrap_or(false) {
                self.play_node(scene, uid, true);
            }
        }
    }

    /// Para o som do nó. Sempre envia o comando: o motor ignora um Stop sem som, e assim um aviso de fim
    /// que chegou atrasado não faz o Stop se perder.
    pub fn stop(&mut self, uid: Uid) {
        self.push(AudioCommand::Stop { node: uid });
    }

    /// O motor avisa que o som do nó acabou sozinho (sem repetir) ou que não pôde tocar. Sem esse aviso, um
    /// som já terminado continuaria registrado como tocando, e um Play com `from_start` falso não o tocaria de novo.
    pub fn finished(&mut self, uid: Uid) {
        self.voices.remove(&uid);
    }

    /// True se o nó tem um som tocando (desde o último Stop).
    pub fn is_playing(&self, uid: Uid) -> bool {
        self.voices.contains_key(&uid)
    }

    /// Atualiza a atenuação dos players 2D que estão tocando, pela distância à câmera atual (AUD-001).
    /// Chamado a cada quadro da reprodução, com a posição já atualizada pela física. Emite
    /// `set_attenuation` só quando a atenuação muda de forma perceptível.
    pub fn update(&mut self, scene: &Scene, time: f64) {
        let spatial: Vec<Uid> = self
            .voices
            .keys()
            .copied()
            .filter(|u| scene.node(*u).is_some_and(|n| n.kind == PLAYER_2D))
            .collect();
        if spatial.is_empty() {
            return;
        }
        let listener = query::current_camera(scene, Some(time)).map(|(x, y, _)| [x, y]);
        let world: HashMap<Uid, query::Xf> = query::world_map(scene, Some(time))
            .into_iter()
            .map(|(u, xf, _)| (u, xf))
            .collect();
        for uid in spatial {
            let Some(node) = scene.node(uid) else {
                continue;
            };
            let db = match (listener, world.get(&uid)) {
                (Some([lx, ly]), Some(xf)) => attenuation_db(
                    (xf.x - lx).hypot(xf.y - ly),
                    num(node, "max_distance", 0.0),
                    num(node, "attenuation", 1.0),
                ),
                _ => 0.0,
            };
            let sent = self.voices.get(&uid).copied().flatten();
            if sent.is_none_or(|s| (s - db).abs() >= ATTENUATION_STEP_DB) {
                self.voices.insert(uid, Some(db));
                self.queue
                    .push(AudioCommand::SetAttenuation { node: uid, db });
            }
        }
    }

    /// Entrega os comandos empilhados desde a última chamada e esvazia a fila.
    pub fn drain(&mut self) -> Vec<AudioCommand> {
        std::mem::take(&mut self.queue)
    }
}

/// Players de áudio reconhecidos pelo núcleo. `AudioStreamPlayer2D` é o único com posição.
pub const PLAYER: &str = "AudioStreamPlayer";
pub const PLAYER_2D: &str = "AudioStreamPlayer2D";

fn is_player(kind: &str) -> bool {
    kind == PLAYER || kind == PLAYER_2D
}

fn num(node: &Node, key: &str, default: f64) -> f64 {
    value_of(node, key).as_f64().unwrap_or(default)
}

/// Ganho linear de um volume em dB: 0 dB é 1, -6 dB é cerca de 0,5 e -80 dB ou menos é silêncio (0).
pub fn db_to_gain(db: f64) -> f64 {
    if db <= SILENCE_DB {
        0.0
    } else {
        10f64.powf(db / 20.0)
    }
}

/// Atenuação espacial de um emissor a `distance` do ouvinte, em dB (zero ou negativo).
///
/// Fora de `max_distance` o som fica em silêncio. `curve` é o expoente da queda do ganho linear
/// `(1 - distância / alcance) ^ curve`: 0 não atenua até o alcance, 1 cai linearmente e valores maiores
/// caem mais depressa. Alcance zero ou negativo silencia tudo.
pub fn attenuation_db(distance: f64, max_distance: f64, curve: f64) -> f64 {
    if !distance.is_finite() || max_distance <= 0.0 || distance >= max_distance {
        return SILENCE_DB;
    }
    if curve <= 0.0 {
        return 0.0;
    }
    let falloff = (1.0 - distance.max(0.0) / max_distance).clamp(0.0, 1.0);
    let gain = falloff.powf(curve);
    (20.0 * gain.log10()).max(SILENCE_DB)
}

// ---------- Efeitos dos barramentos (AUD-003) ----------

/// Reverberação por convolução com um impulso gerado (ConvolverNode no motor).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Reverb {
    pub enabled: bool,
    /// Parte do sinal reverberado, de 0 a 1.
    pub mix: f64,
    /// Tempo de cauda em segundos.
    pub decay: f64,
}

impl Default for Reverb {
    fn default() -> Self {
        Reverb {
            enabled: false,
            mix: 0.3,
            decay: 1.5,
        }
    }
}

/// Eco com realimentação (DelayNode com ganho de realimentação).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Delay {
    pub enabled: bool,
    /// Tempo entre os ecos, em segundos.
    pub time: f64,
    /// Quanto do eco volta para a linha de atraso, de 0 a 0,95. Mais alto, ecos mais longos.
    pub feedback: f64,
    /// Parte do sinal com eco, de 0 a 1.
    pub mix: f64,
}

impl Default for Delay {
    fn default() -> Self {
        Delay {
            enabled: false,
            time: 0.25,
            feedback: 0.35,
            mix: 0.3,
        }
    }
}

/// Filtro passa-baixas: corta as frequências acima de `cutoff_hz`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LowPass {
    pub enabled: bool,
    /// Frequência de corte em hertz.
    pub cutoff_hz: f64,
}

impl Default for LowPass {
    fn default() -> Self {
        LowPass {
            enabled: false,
            cutoff_hz: 1200.0,
        }
    }
}

/// Compressor de dinâmica: reduz o volume dos picos acima de `threshold_db`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Compressor {
    pub enabled: bool,
    /// Limiar em dB, de -100 a 0.
    pub threshold_db: f64,
    /// Taxa de compressão acima do limiar: 4 significa 4 dB de entrada para 1 dB de saída.
    pub ratio: f64,
}

impl Default for Compressor {
    fn default() -> Self {
        Compressor {
            enabled: false,
            threshold_db: -24.0,
            ratio: 4.0,
        }
    }
}

/// Os quatro efeitos de um barramento. Cada efeito só age quando `enabled` é verdadeiro.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct BusEffects {
    pub reverb: Reverb,
    pub delay: Delay,
    pub low_pass: LowPass,
    pub compressor: Compressor,
}

impl BusEffects {
    /// True quando todos os efeitos estão nos padrões. Nesse caso o arquivo de configurações não grava nada.
    pub fn is_default(&self) -> bool {
        *self == BusEffects::default()
    }

    /// Leva cada parâmetro para a faixa que o motor aceita. Valor não numérico volta ao padrão.
    pub fn normalize(&mut self) {
        let d = BusEffects::default();
        self.reverb.mix = clean(self.reverb.mix, 0.0, 1.0, d.reverb.mix);
        self.reverb.decay = clean(self.reverb.decay, 0.1, 10.0, d.reverb.decay);
        self.delay.time = clean(self.delay.time, 0.01, 2.0, d.delay.time);
        self.delay.feedback = clean(self.delay.feedback, 0.0, 0.95, d.delay.feedback);
        self.delay.mix = clean(self.delay.mix, 0.0, 1.0, d.delay.mix);
        self.low_pass.cutoff_hz =
            clean(self.low_pass.cutoff_hz, 20.0, 20000.0, d.low_pass.cutoff_hz);
        self.compressor.threshold_db = clean(
            self.compressor.threshold_db,
            -100.0,
            0.0,
            d.compressor.threshold_db,
        );
        self.compressor.ratio = clean(self.compressor.ratio, 1.0, 20.0, d.compressor.ratio);
    }
}

fn clean(v: f64, min: f64, max: f64, default: f64) -> f64 {
    if v.is_finite() {
        v.clamp(min, max)
    } else {
        default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Scene;
    use serde_json::json;

    /// Cena com um player 2D e uma câmera. O player fica em (x, 0) e a câmera na origem.
    fn cena_2d(x: f64, max: f64, curva: f64) -> (Scene, Uid) {
        let mut s = Scene::new_default("Teste", 7);
        let cam = s.roots()[0];
        let cam_uid = s.node(cam).unwrap().children[0];
        s.set_prop(cam_uid, "position", json!([0.0, 0.0]), None)
            .unwrap();
        let p = s.add_node(None, PLAYER_2D, None).unwrap();
        s.set_prop(p, "position", json!([x, 0.0]), None).unwrap();
        s.set_prop(p, "max_distance", json!(max), None).unwrap();
        s.set_prop(p, "attenuation", json!(curva), None).unwrap();
        s.set_prop(p, "stream", json!("00000000000000ab"), None)
            .unwrap();
        (s, p)
    }

    #[test]
    fn comando_play_serializa_com_tipo_e_campos_em_camel_case() {
        let cmd = AudioCommand::Play {
            node: Uid(0xab),
            stream: Uid(0xcd),
            volume_db: -6.0,
            pitch: 1.5,
            bus: "Música".into(),
            looping: true,
            from_start: false,
        };
        let text = serde_json::to_string(&cmd).unwrap();
        assert_eq!(
            text,
            r#"{"type":"play","node":"00000000000000ab","stream":"00000000000000cd","volumeDb":-6.0,"pitch":1.5,"bus":"Música","looping":true,"fromStart":false}"#
        );
        let volta: AudioCommand = serde_json::from_str(&text).unwrap();
        assert_eq!(volta, cmd);
    }

    #[test]
    fn todos_os_comandos_ida_e_volta_pelo_json() {
        let cmds = vec![
            AudioCommand::Stop { node: Uid(1) },
            AudioCommand::Fade {
                node: Uid(1),
                to_db: -40.0,
                seconds: 0.5,
            },
            AudioCommand::SetBusVolume {
                bus: "Master".into(),
                volume_db: -3.0,
            },
            AudioCommand::SetBusMute {
                bus: "Master".into(),
                mute: true,
            },
            AudioCommand::SetAttenuation {
                node: Uid(1),
                db: -12.0,
            },
        ];
        let text = serde_json::to_string(&cmds).unwrap();
        assert!(text.contains(r#""type":"set_bus_volume""#));
        assert!(text.contains(r#""type":"set_attenuation""#));
        assert!(text.contains(r#""toDb":-40.0"#));
        let volta: Vec<AudioCommand> = serde_json::from_str(&text).unwrap();
        assert_eq!(volta, cmds);
    }

    #[test]
    fn fila_entrega_em_ordem_e_esvazia() {
        let mut a = AudioPlayback::default();
        a.push(AudioCommand::SetBusMute {
            bus: "Master".into(),
            mute: true,
        });
        a.push(AudioCommand::SetBusMute {
            bus: "Master".into(),
            mute: false,
        });
        let primeiro = a.drain();
        assert_eq!(primeiro.len(), 2);
        assert_eq!(
            primeiro[0],
            AudioCommand::SetBusMute {
                bus: "Master".into(),
                mute: true
            }
        );
        assert!(a.drain().is_empty());
    }

    #[test]
    fn autoplay_toca_so_players_com_autoplay_e_audio() {
        let mut s = Scene::new_default("Teste", 3);
        let com = s.add_node(None, PLAYER, None).unwrap();
        s.set_prop(com, "autoplay", json!(true), None).unwrap();
        s.set_prop(com, "stream", json!("00000000000000ab"), None)
            .unwrap();
        // Autoplay sem áudio não toca.
        let sem_audio = s.add_node(None, PLAYER, None).unwrap();
        s.set_prop(sem_audio, "autoplay", json!(true), None)
            .unwrap();
        // Áudio sem autoplay não toca.
        let quieto = s.add_node(None, PLAYER, None).unwrap();
        s.set_prop(quieto, "stream", json!("00000000000000ac"), None)
            .unwrap();

        let mut a = AudioPlayback::default();
        a.autoplay(&s);
        let cmds = a.drain();
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            AudioCommand::Play {
                node,
                stream,
                volume_db,
                pitch,
                bus,
                looping,
                from_start,
            } => {
                assert_eq!(*node, com);
                assert_eq!(*stream, Uid(0xab));
                assert_eq!(*volume_db, 0.0);
                assert_eq!(*pitch, 1.0);
                assert_eq!(bus, "Master");
                assert!(!looping);
                assert!(from_start);
            }
            outro => panic!("comando inesperado: {outro:?}"),
        }
        assert!(a.is_playing(com));
        assert!(!a.is_playing(quieto));
    }

    #[test]
    fn play_sem_recomecar_nao_reinicia_som_tocando() {
        let (s, p) = cena_2d(0.0, 2000.0, 1.0);
        let mut a = AudioPlayback::default();
        assert!(a.play_node(&s, p, true));
        assert!(
            !a.play_node(&s, p, false),
            "o som já toca e from_start é falso"
        );
        assert!(a.play_node(&s, p, true), "from_start verdadeiro reinicia");
        assert_eq!(a.drain().len(), 2);
    }

    #[test]
    fn stop_sempre_envia_o_comando_e_limpa_o_som() {
        let (s, p) = cena_2d(0.0, 2000.0, 1.0);
        let mut a = AudioPlayback::default();
        a.stop(p);
        assert_eq!(a.drain(), vec![AudioCommand::Stop { node: p }]);
        a.play_node(&s, p, true);
        a.stop(p);
        assert!(!a.is_playing(p));
        assert_eq!(a.drain().last(), Some(&AudioCommand::Stop { node: p }));
    }

    #[test]
    fn som_terminado_toca_de_novo_sem_reiniciar() {
        let (s, p) = cena_2d(0.0, 2000.0, 1.0);
        let mut a = AudioPlayback::default();
        assert!(a.play_node(&s, p, true));
        // Sem o aviso de fim, o som ainda conta como tocando e o Play sem reinício é ignorado.
        assert!(!a.play_node(&s, p, false));
        a.finished(p);
        assert!(!a.is_playing(p));
        assert!(
            a.play_node(&s, p, false),
            "depois do fim, o som toca de novo"
        );
    }

    #[test]
    fn decibeis_viram_ganho_linear() {
        assert!((db_to_gain(0.0) - 1.0).abs() < 1e-12);
        assert!((db_to_gain(-6.0206) - 0.5).abs() < 1e-4);
        assert_eq!(db_to_gain(-80.0), 0.0);
        assert_eq!(db_to_gain(-200.0), 0.0);
    }

    #[test]
    fn atenuacao_cai_com_a_distancia_e_silencia_no_alcance() {
        // Curva 1 é linear: metade do alcance dá metade do ganho, cerca de -6 dB.
        assert!((attenuation_db(1000.0, 2000.0, 1.0) - (-6.0206)).abs() < 1e-3);
        assert_eq!(attenuation_db(0.0, 2000.0, 1.0), 0.0);
        assert_eq!(attenuation_db(2000.0, 2000.0, 1.0), SILENCE_DB);
        assert_eq!(attenuation_db(5000.0, 2000.0, 1.0), SILENCE_DB);
        // Curva zero não atenua antes do alcance.
        assert_eq!(attenuation_db(1500.0, 2000.0, 0.0), 0.0);
        // Alcance inválido silencia.
        assert_eq!(attenuation_db(10.0, 0.0, 1.0), SILENCE_DB);
    }

    #[test]
    fn atenuacao_2d_segue_a_distancia_a_camera() {
        let (mut s, p) = cena_2d(1000.0, 2000.0, 1.0);
        let mut a = AudioPlayback::default();
        a.play_node(&s, p, true);
        a.update(&s, 0.0);
        let cmds = a.drain();
        let primeiro = cmds
            .iter()
            .find_map(|c| match c {
                AudioCommand::SetAttenuation { db, .. } => Some(*db),
                _ => None,
            })
            .expect("a atenuação inicial é enviada");
        assert!((primeiro - (-6.0206)).abs() < 1e-3);

        // Sem mudança de distância, nada novo é enviado.
        a.update(&s, 0.0);
        assert!(
            a.drain()
                .iter()
                .all(|c| !matches!(c, AudioCommand::SetAttenuation { .. }))
        );

        // Câmera mais perto: a atenuação diminui e o comando é reenviado.
        let cam_uid = s.node(s.roots()[0]).unwrap().children[0];
        s.set_prop(cam_uid, "position", json!([900.0, 0.0]), None)
            .unwrap();
        a.update(&s, 0.0);
        let cmds = a.drain();
        assert!(
            cmds.iter()
                .any(|c| matches!(c, AudioCommand::SetAttenuation { node, .. } if *node == p))
        );
    }

    #[test]
    fn player_2d_sem_camera_fica_sem_atenuacao() {
        let mut s = Scene::new("Sem câmera", 9);
        let p = s.add_node(None, PLAYER_2D, None).unwrap();
        s.set_prop(p, "stream", json!("00000000000000ab"), None)
            .unwrap();
        let mut a = AudioPlayback::default();
        a.play_node(&s, p, true);
        a.update(&s, 0.0);
        let db: Vec<f64> = a
            .drain()
            .iter()
            .filter_map(|c| match c {
                AudioCommand::SetAttenuation { db, .. } => Some(*db),
                _ => None,
            })
            .collect();
        assert_eq!(db, vec![0.0]);
    }

    #[test]
    fn efeitos_nos_padroes_nao_entram_no_arquivo() {
        let efeitos = BusEffects::default();
        assert!(efeitos.is_default());
        let mut mudado = BusEffects::default();
        mudado.delay.enabled = true;
        assert!(!mudado.is_default());
    }

    #[test]
    fn efeitos_fora_da_faixa_voltam_para_o_limite() {
        let mut e = BusEffects::default();
        e.delay.feedback = 3.0;
        e.reverb.decay = f64::NAN;
        e.low_pass.cutoff_hz = 1.0;
        e.compressor.ratio = 99.0;
        e.normalize();
        assert_eq!(e.delay.feedback, 0.95);
        assert_eq!(e.reverb.decay, Reverb::default().decay);
        assert_eq!(e.low_pass.cutoff_hz, 20.0);
        assert_eq!(e.compressor.ratio, 20.0);
    }
}
