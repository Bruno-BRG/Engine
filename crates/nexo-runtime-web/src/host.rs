//! Host da reprodução (VIS-023, INP-003, CON-004, AUD-004): a ponte entre a VM dos scripts visuais e o mundo da
//! reprodução. A cena, a física, o áudio, a entrada e o console entram por referências emprestadas, montadas a cada
//! quadro de cada script. O `Mundo` guarda o nó dono do script, para o console dizer de onde veio cada mensagem.

use nexo_core::audio::DEFAULT_BUS;
use nexo_core::registry::{self, PropKind};
use nexo_core::scene::{as_vec2, value_of};
use nexo_core::{AudioCommand, AudioPlayback, Input, ProjectSettings, Scene, Uid};
use nexo_graph::{Host, LogLevel, Value};
use nexo_physics::Physics;
use serde::Serialize;
use serde_json::json;

/// Ponteiro do mouse na reprodução: a posição na tela (pixels do canvas) e no mundo, como o viewport o desenha.
/// O TypeScript envia com `pointer_event`.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Ponteiro {
    pub tela: [f64; 2],
    pub mundo: [f64; 2],
}

/// Mensagem do console (CON-004), na ordem em que os scripts a escreveram. O TypeScript as lê em JSON.
#[derive(Clone, Debug, Serialize)]
pub(crate) struct Mensagem {
    /// "info", "warning" ou "error".
    pub level: &'static str,
    /// Nó que escreveu a mensagem: o dono do script.
    pub node: Uid,
    pub message: String,
}

/// O mundo visto por um script durante um quadro: as partes da reprodução e do projeto que o Host usa, e o nó dono.
pub(crate) struct Mundo<'a> {
    pub cena: &'a mut Scene,
    pub fisica: &'a mut Physics,
    pub audio: &'a mut AudioPlayback,
    pub entrada: &'a Input,
    pub ajustes: &'a ProjectSettings,
    pub ponteiro: Ponteiro,
    pub console: &'a mut Vec<Mensagem>,
    /// Nó que roda o script: é a saída "self" dos eventos, e o dono das mensagens do console.
    pub dono: Uid,
}

/// Componente de uma propriedade de vetor: "position_x" é o X de "position". Os scripts usam esses nomes (Mover até,
/// Tween) como se fossem propriedades numéricas.
fn componente(nome: &str) -> Option<(&'static str, usize)> {
    match nome {
        "position_x" => Some(("position", 0)),
        "position_y" => Some(("position", 1)),
        "scale_x" => Some(("scale", 0)),
        "scale_y" => Some(("scale", 1)),
        _ => None,
    }
}

fn e_vetor(kind: &str, prop: &str) -> bool {
    registry::prop_def(kind, prop).is_some_and(|d| d.kind == PropKind::Vec2)
}

/// Decimal value: a decimal, or an integer (which converts).
fn as_decimal(value: &Value) -> Option<f64> {
    match value {
        Value::Float(f) => Some(*f),
        Value::Int(n) => Some(*n as f64),
        _ => None,
    }
}

/// JSON the registry stores for a property of kind `kind`, from the script's value (VIS-021). `None`
/// when the value is not of that kind: the write is refused. Decimal and integer convert; the core's `coerce`
/// rounds and clamps the number.
fn json_for_kind(kind: PropKind, value: &Value) -> Option<serde_json::Value> {
    match (kind, value) {
        (PropKind::Float, Value::Float(f)) => Some(json!(f)),
        (PropKind::Float, Value::Int(n)) => Some(json!(*n as f64)),
        (PropKind::Int | PropKind::Mask, Value::Int(n)) => Some(json!(n)),
        (PropKind::Int | PropKind::Mask, Value::Float(f)) => Some(json!(f)),
        (PropKind::Bool, Value::Bool(b)) => Some(json!(b)),
        (PropKind::Str | PropKind::Enum | PropKind::Color | PropKind::Uid, Value::String(s)) => Some(json!(s)),
        (PropKind::Vec2, Value::Vec2(v)) => Some(json!(v)),
        _ => None,
    }
}

/// Nível do console no contrato com o TypeScript. Print escreve "info", Warn "warning" e Error "error".
fn nivel(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Info => "info",
        LogLevel::Warning => "warning",
        LogLevel::Error => "error",
    }
}

impl Mundo<'_> {
    fn aviso(&mut self, message: String) {
        self.console.push(Mensagem { level: "warning", node: self.dono, message });
    }

    fn barramento_existe(&self, nome: &str) -> bool {
        self.ajustes.buses.iter().any(|b| b.name == nome)
    }
}

impl Host for Mundo<'_> {
    /// Reads the property as a value of its type. A vector component ("position_x") is decimal.
    fn get_property(&mut self, node: u64, name: &str) -> Option<Value> {
        let n = self.cena.node(Uid(node))?;
        if let Some((prop, i)) = componente(name) {
            if !e_vetor(&n.kind, prop) {
                return None;
            }
            return value_of(n, prop).as_array()?.get(i)?.as_f64().map(Value::Float);
        }
        let def = registry::prop_def(&n.kind, name)?;
        Value::from_json(def.kind, &value_of(n, name))
    }

    /// Writes without history: playback has its own copy of the scene, which is not part of undo. Refuses a value
    /// of another type than the property's.
    fn set_property(&mut self, node: u64, name: &str, value: Value) -> bool {
        let uid = Uid(node);
        let Some(n) = self.cena.node(uid) else {
            return false;
        };
        let (prop, valor) = if let Some((prop, i)) = componente(name) {
            if !e_vetor(&n.kind, prop) {
                return false;
            }
            let Some(x) = as_decimal(&value) else {
                return false;
            };
            let mut v = as_vec2(&value_of(n, prop));
            v[i] = x;
            (prop, json!(v))
        } else {
            let Some(def) = registry::prop_def(&n.kind, name) else {
                return false;
            };
            let Some(valor) = json_for_kind(def.kind, &value) else {
                return false;
            };
            (name, valor)
        };
        self.cena.set_runtime_prop(uid, prop, valor).is_ok()
    }

    fn action_pressed(&self, action: &str) -> bool {
        self.entrada.action_pressed(&self.ajustes.input, action)
    }

    fn action_just_pressed(&self, action: &str) -> bool {
        self.entrada.action_just_pressed(&self.ajustes.input, action)
    }

    fn action_released(&self, action: &str) -> bool {
        self.entrada.action_released(&self.ajustes.input, action)
    }

    fn axis(&self, negative: &str, positive: &str) -> f64 {
        self.entrada.axis(&self.ajustes.input, negative, positive)
    }

    fn vector(&self, left: &str, right: &str, up: &str, down: &str) -> [f64; 2] {
        self.entrada.vector(&self.ajustes.input, left, right, up, down)
    }

    fn mouse_world(&self) -> [f64; 2] {
        self.ponteiro.mundo
    }

    fn mouse_screen(&self) -> [f64; 2] {
        self.ponteiro.tela
    }

    fn log(&mut self, level: LogLevel, message: &str) {
        self.console.push(Mensagem { level: nivel(level), node: self.dono, message: message.to_string() });
    }

    /// Primeiro nó da árvore com o nome dado (ordem de árvore).
    fn find_node(&self, name: &str) -> Option<u64> {
        self.cena
            .pre_order()
            .into_iter()
            .find(|u| self.cena.node(*u).is_some_and(|n| n.name == name))
            .map(|u| u.0)
    }

    /// Não suportado ainda: a reprodução não tem o catálogo de cenas do projeto (o texto das cenas, para copiá-las).
    /// Devolve None, e a VM para com erro, que o console mostra uma vez por script.
    fn instance_scene(&mut self, _scene: &str, _position: [f64; 2]) -> Option<u64> {
        None
    }

    /// Remove o nó e os descendentes da cópia de reprodução, com os corpos de física e os sons deles.
    fn destroy(&mut self, node: u64) {
        let uid = Uid(node);
        if self.cena.node(uid).is_none() {
            return;
        }
        for u in self.cena.subtree(uid) {
            self.fisica.remove(u);
            if self.audio.is_playing(u) {
                self.audio.stop(u);
            }
        }
        let _ = self.cena.remove_runtime(uid);
    }

    /// Define a velocidade do personagem, que a física aplica no passo seguinte (60 Hz). O corpo anda no passo
    /// depois desta chamada, e o chão devolvido é o do último passo. A velocidade fica no personagem, como
    /// `velocity` do CharacterBody2D. None se o nó não é um personagem.
    fn move_and_slide(&mut self, node: u64, velocity: [f64; 2]) -> Option<bool> {
        let uid = Uid(node);
        if !self.fisica.set_velocity(uid, velocity) {
            return None;
        }
        Some(self.fisica.grounded(uid))
    }

    /// Comandos de áudio do script. Nó inexistente e barramento desconhecido viram aviso no console. Tocar em um
    /// barramento que não existe usa o padrão (Master), como o motor faz.
    fn audio(&mut self, command: AudioCommand) {
        let command = match command {
            AudioCommand::Play { node, stream, volume_db, pitch, bus, looping, from_start } => {
                if self.cena.node(node).is_none() {
                    self.aviso(format!("Tocar som: o nó {node} não existe"));
                    return;
                }
                let bus = if self.barramento_existe(&bus) {
                    bus
                } else {
                    self.aviso(format!("Tocar som: o barramento '{bus}' não existe; o som toca em {DEFAULT_BUS}"));
                    DEFAULT_BUS.to_string()
                };
                AudioCommand::Play { node, stream, volume_db, pitch, bus, looping, from_start }
            }
            AudioCommand::SetBusVolume { bus, volume_db } => {
                if !self.barramento_existe(&bus) {
                    self.aviso(format!("Volume do barramento: o barramento '{bus}' não existe"));
                    return;
                }
                AudioCommand::SetBusVolume { bus, volume_db }
            }
            outro => outro,
        };
        self.audio.push(command);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parts of the playback the host uses, with a Sprite2D scene.
    struct Parts {
        cena: Scene,
        fisica: Physics,
        audio: AudioPlayback,
        entrada: Input,
        ajustes: ProjectSettings,
        console: Vec<Mensagem>,
        uid: Uid,
    }

    impl Parts {
        fn create() -> Parts {
            let mut cena = Scene::new("teste", 1);
            let uid = cena.add_node(None, "Sprite2D", None).unwrap();
            let ajustes = ProjectSettings::default();
            let fisica = Physics::build(&cena, &ajustes);
            Parts {
                cena,
                fisica,
                audio: AudioPlayback::default(),
                entrada: Input::default(),
                ajustes,
                console: Vec::new(),
                uid,
            }
        }

        fn mundo(&mut self) -> Mundo<'_> {
            Mundo {
                cena: &mut self.cena,
                fisica: &mut self.fisica,
                audio: &mut self.audio,
                entrada: &self.entrada,
                ajustes: &self.ajustes,
                ponteiro: Ponteiro::default(),
                console: &mut self.console,
                dono: self.uid,
            }
        }
    }

    #[test]
    fn read_returns_property_type() {
        let mut p = Parts::create();
        let uid = p.uid.0;
        let mut m = p.mundo();
        assert_eq!(m.get_property(uid, "modulate"), Some(Value::String("#ffffff".into())));
        assert_eq!(m.get_property(uid, "position"), Some(Value::Vec2([0.0, 0.0])));
        assert_eq!(m.get_property(uid, "centered"), Some(Value::Bool(true)));
        assert_eq!(m.get_property(uid, "z_index"), Some(Value::Int(0)));
        assert_eq!(m.get_property(uid, "position_x"), Some(Value::Float(0.0)));
        assert_eq!(m.get_property(uid, "nao_existe"), None);
    }

    #[test]
    fn wrong_type_write_returns_false() {
        let mut p = Parts::create();
        let uid = p.uid.0;
        let mut m = p.mundo();
        assert!(m.set_property(uid, "modulate", Value::String("#ff0000".into())));
        // Decimal into a colour, text into a boolean, decimal into a vector, and a missing property: all refused.
        assert!(!m.set_property(uid, "modulate", Value::Float(1.0)));
        assert!(!m.set_property(uid, "centered", Value::String("sim".into())));
        assert!(!m.set_property(uid, "position", Value::Float(3.0)));
        assert!(!m.set_property(uid, "nao_existe", Value::Float(1.0)));
        assert_eq!(m.get_property(uid, "modulate"), Some(Value::String("#ff0000".into())));
        // Decimal and integer convert, as before typing; the core rounds the integer.
        assert!(m.set_property(uid, "z_index", Value::Float(4.6)));
        assert_eq!(m.get_property(uid, "z_index"), Some(Value::Int(5)));
        assert!(m.set_property(uid, "rotation", Value::Int(90)));
        assert_eq!(m.get_property(uid, "rotation"), Some(Value::Float(90.0)));
    }

    #[test]
    fn vector_component_stays_decimal() {
        let mut p = Parts::create();
        let uid = p.uid.0;
        let mut m = p.mundo();
        assert!(m.set_property(uid, "position_x", Value::Float(12.0)));
        assert_eq!(m.get_property(uid, "position"), Some(Value::Vec2([12.0, 0.0])));
        assert_eq!(m.get_property(uid, "position_x"), Some(Value::Float(12.0)));
        assert!(!m.set_property(uid, "position_x", Value::String("x".into())));
    }

    #[test]
    fn write_bool_succeeds() {
        let mut p = Parts::create();
        let uid = p.uid.0;
        let mut m = p.mundo();
        assert!(m.set_property(uid, "centered", Value::Bool(false)));
        assert_eq!(m.get_property(uid, "centered"), Some(Value::Bool(false)));
    }

    #[test]
    fn write_vec2_succeeds() {
        let mut p = Parts::create();
        let uid = p.uid.0;
        let mut m = p.mundo();
        assert!(m.set_property(uid, "position", Value::Vec2([3.0, 4.0])));
        assert_eq!(m.get_property(uid, "position"), Some(Value::Vec2([3.0, 4.0])));
        assert_eq!(m.get_property(uid, "position_y"), Some(Value::Float(4.0)));
    }

    #[test]
    fn write_enum_succeeds() {
        let mut p = Parts::create();
        let uid = p.uid.0;
        let mut m = p.mundo();
        // "circle" is one of the options of `shape`; the core refuses the others.
        assert!(m.set_property(uid, "shape", Value::String("circle".into())));
        assert_eq!(m.get_property(uid, "shape"), Some(Value::String("circle".into())));
    }

    #[test]
    fn write_uid_succeeds() {
        let mut p = Parts::create();
        let uid = p.uid.0;
        let mut m = p.mundo();
        // A uid is a hexadecimal asset id, so "ff" is valid.
        assert!(m.set_property(uid, "texture", Value::String("ff".into())));
        assert_eq!(m.get_property(uid, "texture"), Some(Value::String("ff".into())));
    }

    #[test]
    fn write_mask_succeeds() {
        let mut p = Parts::create();
        let body = p.cena.add_node(None, "RigidBody2D", None).unwrap().0;
        let mut m = p.mundo();
        assert!(m.set_property(body, "collision_mask", Value::Int(5)));
        assert_eq!(m.get_property(body, "collision_mask"), Some(Value::Int(5)));
    }
}
