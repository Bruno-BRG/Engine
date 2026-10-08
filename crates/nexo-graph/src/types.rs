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

    /// Tipo de pino de uma propriedade do registro (VIS-021). Inteiro e máscara viram inteiro; enum, cor e UID
    /// viram texto, como são escritos. A lista de pontos não tem pino: `None`.
    pub fn of_prop(kind: PropKind) -> Option<PinType> {
        match kind {
            PropKind::Float => Some(PinType::Float),
            PropKind::Int | PropKind::Mask => Some(PinType::Int),
            PropKind::Bool => Some(PinType::Bool),
            PropKind::Str | PropKind::Enum | PropKind::Color | PropKind::Uid => Some(PinType::String),
            PropKind::Vec2 => Some(PinType::Vec2),
            PropKind::Points => None,
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

    /// Valor de uma propriedade do registro, a partir do JSON que ele guarda (o padrão, ou o valor da cena). `None`
    /// quando o tipo não tem pino (lista de pontos) ou o JSON não tem o formato do tipo.
    pub fn from_json(kind: PropKind, json: &serde_json::Value) -> Option<Value> {
        Some(match kind {
            PropKind::Float => Value::Float(json.as_f64()?),
            PropKind::Int | PropKind::Mask => Value::Int(json.as_f64()?.round() as i64),
            PropKind::Bool => Value::Bool(json.as_bool()?),
            PropKind::Str | PropKind::Enum | PropKind::Color | PropKind::Uid => {
                Value::String(json.as_str()?.to_string())
            }
            PropKind::Vec2 => {
                let par = json.as_array().filter(|a| a.len() == 2)?;
                Value::Vec2([par[0].as_f64()?, par[1].as_f64()?])
            }
            PropKind::Points => return None,
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
    fn mapa_de_tipos_do_registro_para_pinos() {
        assert_eq!(PinType::of_prop(PropKind::Float), Some(PinType::Float));
        assert_eq!(PinType::of_prop(PropKind::Int), Some(PinType::Int));
        assert_eq!(PinType::of_prop(PropKind::Mask), Some(PinType::Int));
        assert_eq!(PinType::of_prop(PropKind::Bool), Some(PinType::Bool));
        for kind in [PropKind::Str, PropKind::Enum, PropKind::Color, PropKind::Uid] {
            assert_eq!(PinType::of_prop(kind), Some(PinType::String), "{kind:?}");
        }
        assert_eq!(PinType::of_prop(PropKind::Vec2), Some(PinType::Vec2));
        // Lista de pontos não tem pino: os nós de propriedade a recusam.
        assert_eq!(PinType::of_prop(PropKind::Points), None);
    }

    #[test]
    fn valor_de_json_do_registro_no_tipo_do_pino() {
        assert_eq!(Value::from_json(PropKind::Float, &json!(0.5)), Some(Value::Float(0.5)));
        assert_eq!(Value::from_json(PropKind::Mask, &json!(-1)), Some(Value::Int(-1)));
        assert_eq!(Value::from_json(PropKind::Bool, &json!(true)), Some(Value::Bool(true)));
        assert_eq!(
            Value::from_json(PropKind::Color, &json!("#ffffff")),
            Some(Value::String("#ffffff".into()))
        );
        assert_eq!(Value::from_json(PropKind::Vec2, &json!([1.0, 2.0])), Some(Value::Vec2([1.0, 2.0])));
        // Formato que não bate com o tipo: nada, em vez de um valor inventado.
        assert_eq!(Value::from_json(PropKind::Vec2, &json!([1.0])), None);
        assert_eq!(Value::from_json(PropKind::Float, &json!("x")), None);
        assert_eq!(Value::from_json(PropKind::Points, &json!([])), None);
    }
}
