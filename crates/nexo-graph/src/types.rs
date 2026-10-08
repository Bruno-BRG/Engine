//! Tipos de pino e valores do grafo. `Flow` é a ordem de execução (o fio branco do editor); os demais
//! carregam dados, nas cores de docs/04-visual-programming.md.

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
