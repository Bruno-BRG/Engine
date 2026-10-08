//! Tipos de pino e valores do grafo. `Flow` é a ordem de execução (o fio branco do editor); os demais
//! carregam dados, nas cores de docs/04-visual-programming.md.

use nexo_core::registry::PropKind;
use serde::{Deserialize, Serialize};

/// Tipo de um pino. Uma saída só liga numa entrada do mesmo tipo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PinType {
    /// Ordem de execução. Não carrega dado.
    Flow,
    Bool,
    Int,
    Float,
    String,
    /// Par [x, y].
    Vec2,
    /// Referência a um nó da cena, pelo id.
    Node,
}

impl PinType {
    /// Nome em português, para as mensagens de erro.
    pub fn nome(self) -> &'static str {
        match self {
            PinType::Flow => "fluxo",
            PinType::Bool => "booleano",
            PinType::Int => "inteiro",
            PinType::Float => "decimal",
            PinType::String => "texto",
            PinType::Vec2 => "vetor 2D",
            PinType::Node => "nó",
        }
    }

    /// Pin type of a registry property (VIS-021). Integer and mask become integer; enum, colour and UID
    /// become text, as they are written. A list of points and a map have no pin: `None`.
    pub fn of_prop(kind: PropKind) -> Option<PinType> {
        match kind {
            PropKind::Float => Some(PinType::Float),
            PropKind::Int | PropKind::Mask => Some(PinType::Int),
            PropKind::Bool => Some(PinType::Bool),
            PropKind::Str | PropKind::Enum | PropKind::Color | PropKind::Uid => Some(PinType::String),
            PropKind::Vec2 => Some(PinType::Vec2),
            PropKind::Points | PropKind::Map => None,
        }
    }
}

/// Valor de uma entrada embutida, de um padrão ou de uma variável.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Vec2([f64; 2]),
    /// Id de um nó da cena.
    Node(u64),
}

impl Value {
    /// Tipo de pino que este valor preenche.
    pub fn ty(&self) -> PinType {
        match self {
            Value::Bool(_) => PinType::Bool,
            Value::Int(_) => PinType::Int,
            Value::Float(_) => PinType::Float,
            Value::String(_) => PinType::String,
            Value::Vec2(_) => PinType::Vec2,
            Value::Node(_) => PinType::Node,
        }
    }

    /// Value of a registry property, from the JSON it stores (the default, or the scene's value). `None`
    /// when the type has no pin (a list of points) or the JSON does not have the type's shape.
    pub fn from_json(kind: PropKind, json: &serde_json::Value) -> Option<Value> {
        Some(match kind {
            PropKind::Float => Value::Float(json.as_f64()?),
            PropKind::Int | PropKind::Mask => Value::Int(json.as_f64()?.round() as i64),
            PropKind::Bool => Value::Bool(json.as_bool()?),
            PropKind::Str | PropKind::Enum | PropKind::Color | PropKind::Uid => {
                Value::String(json.as_str()?.to_string())
            }
            PropKind::Vec2 => {
                let pair = json.as_array().filter(|a| a.len() == 2)?;
                Value::Vec2([pair[0].as_f64()?, pair[1].as_f64()?])
            }
            PropKind::Points | PropKind::Map => return None,
        })
    }
}

/// Nível de uma mensagem do console (CON-004): Print escreve Info, Warn escreve Warning, Error escreve Error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

/// Variável do script (VIS-022). Com `exported`, ela aparece no Inspector e cada instância guarda o seu valor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub ty: PinType,
    pub default: Value,
    #[serde(default)]
    pub exported: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn registry_types_map_to_pins() {
        assert_eq!(PinType::of_prop(PropKind::Float), Some(PinType::Float));
        assert_eq!(PinType::of_prop(PropKind::Int), Some(PinType::Int));
        assert_eq!(PinType::of_prop(PropKind::Mask), Some(PinType::Int));
        assert_eq!(PinType::of_prop(PropKind::Bool), Some(PinType::Bool));
        for kind in [PropKind::Str, PropKind::Enum, PropKind::Color, PropKind::Uid] {
            assert_eq!(PinType::of_prop(kind), Some(PinType::String), "{kind:?}");
        }
        assert_eq!(PinType::of_prop(PropKind::Vec2), Some(PinType::Vec2));
        // A list of points has no pin: property nodes refuse it.
        assert_eq!(PinType::of_prop(PropKind::Points), None);
        // A map (script_vars, VIS-022) has no pin either.
        assert_eq!(PinType::of_prop(PropKind::Map), None);
    }

    #[test]
    fn registry_json_becomes_pin_type_value() {
        assert_eq!(Value::from_json(PropKind::Float, &json!(0.5)), Some(Value::Float(0.5)));
        assert_eq!(Value::from_json(PropKind::Mask, &json!(-1)), Some(Value::Int(-1)));
        assert_eq!(Value::from_json(PropKind::Bool, &json!(true)), Some(Value::Bool(true)));
        assert_eq!(
            Value::from_json(PropKind::Color, &json!("#ffffff")),
            Some(Value::String("#ffffff".into()))
        );
        assert_eq!(Value::from_json(PropKind::Vec2, &json!([1.0, 2.0])), Some(Value::Vec2([1.0, 2.0])));
        // Shape that does not match the type: nothing, instead of an invented value.
        assert_eq!(Value::from_json(PropKind::Vec2, &json!([1.0])), None);
        assert_eq!(Value::from_json(PropKind::Float, &json!("x")), None);
        assert_eq!(Value::from_json(PropKind::Points, &json!([])), None);
        assert_eq!(Value::from_json(PropKind::Map, &json!({})), None);
    }
}
