//! Biblioteca de nós v1 (VIS-021, subconjunto). Cada tipo de nó é uma entrada de dados: o editor lista esta
//! tabela no menu de busca (VIS-011) e o compilador reconhece o nó pelo `kind`. Para acrescentar um nó, basta
//! uma definição aqui e um ramo em `ir.rs`.

use crate::types::{PinType, Value};
use serde::Serialize;
use std::sync::OnceLock;

/// Pino de um nó. `ty` vazio quer dizer "o tipo da variável que o nó nomeia" (GetVariable e SetVariable).
/// `default` vazio torna a entrada obrigatória: sem fio e sem valor embutido, a compilação falha.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PinDef {
    pub name: &'static str,
    pub label: &'static str,
    pub ty: Option<PinType>,
    pub default: Option<Value>,
}

/// Um tipo de nó da biblioteca.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeDef {
    /// Nome do tipo no grafo e no compilador: "Add", "OnProcess"...
    pub kind: &'static str,
    pub category: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    /// Evento: começa um programa e não tem entrada de fluxo.
    pub event: bool,
    pub inputs: Vec<PinDef>,
    pub outputs: Vec<PinDef>,
}

impl PinDef {
    fn new(name: &'static str, label: &'static str, ty: PinType) -> Self {
        Self {
            name,
            label,
            ty: Some(ty),
            default: None,
        }
    }

    /// Pino cujo tipo segue a variável nomeada pelo próprio nó.
    fn of_variable(name: &'static str, label: &'static str) -> Self {
        Self {
            name,
            label,
            ty: None,
            default: None,
        }
    }

    /// Pino cujo tipo segue o fio que chega ao reroute (VIS-014). Sem `ty`, como o de variável, mas a
    /// resolução é por fio: ver `Graph::tipo_do_reroute`.
    fn segue_o_fio(name: &'static str, label: &'static str) -> Self {
        Self {
            name,
            label,
            ty: None,
            default: None,
        }
    }

    /// Valor usado quando o pino não tem fio nem valor embutido.
    fn padrao(mut self, value: Value) -> Self {
        self.default = Some(value);
        self
    }
}

/// Entrada de decimal com padrão; o caso mais comum da biblioteca.
fn decimal(name: &'static str, label: &'static str, padrao: f64) -> PinDef {
    PinDef::new(name, label, PinType::Float).padrao(Value::Float(padrao))
}

fn build() -> Vec<NodeDef> {
    let exec = || PinDef::new("exec", "Fluxo", PinType::Flow);
    vec![
        NodeDef {
            kind: "OnReady",
            category: "Eventos",
            label: "Ao ficar pronto",
            description: "Roda uma vez, quando o nó entra na cena (On Ready).",
            event: true,
            inputs: vec![],
            outputs: vec![exec(), PinDef::new("self", "Este nó", PinType::Node)],
        },
        NodeDef {
            kind: "OnProcess",
            category: "Eventos",
            label: "A cada quadro",
            description: "Roda a cada quadro (On Process). Fornece o delta, em segundos.",
            event: true,
            inputs: vec![],
            outputs: vec![
                exec(),
                PinDef::new("delta", "Delta (s)", PinType::Float),
                PinDef::new("self", "Este nó", PinType::Node),
            ],
        },
        NodeDef {
            kind: "Branch",
            category: "Fluxo",
            label: "Se (Branch)",
            description: "Segue por Verdadeiro ou por Falso, conforme a condição.",
            event: false,
            inputs: vec![
                exec(),
                PinDef::new("condition", "Condição", PinType::Bool).padrao(Value::Bool(false)),
            ],
            outputs: vec![
                PinDef::new("true", "Verdadeiro", PinType::Flow),
                PinDef::new("false", "Falso", PinType::Flow),
            ],
        },
        NodeDef {
            kind: "Sequence",
            category: "Fluxo",
            label: "Sequência",
            description: "Roda os dois caminhos de saída, um depois do outro.",
            event: false,
            inputs: vec![exec()],
            outputs: vec![
                PinDef::new("then_0", "Passo 1", PinType::Flow),
                PinDef::new("then_1", "Passo 2", PinType::Flow),
            ],
        },
        NodeDef {
            kind: "Add",
            category: "Matemática",
            label: "Somar",
            description: "A + B, em decimal.",
            event: false,
            inputs: vec![decimal("a", "A", 0.0), decimal("b", "B", 0.0)],
            outputs: vec![PinDef::new("result", "Resultado", PinType::Float)],
        },
        NodeDef {
            kind: "Multiply",
            category: "Matemática",
            label: "Multiplicar",
            description: "A * B, em decimal.",
            event: false,
            inputs: vec![decimal("a", "A", 1.0), decimal("b", "B", 1.0)],
            outputs: vec![PinDef::new("result", "Resultado", PinType::Float)],
        },
        NodeDef {
            kind: "Greater",
            category: "Comparação",
            label: "Maior que",
            description: "Verdadeiro quando A é maior que B.",
            event: false,
            inputs: vec![decimal("a", "A", 0.0), decimal("b", "B", 0.0)],
            outputs: vec![PinDef::new("result", "Resultado", PinType::Bool)],
        },
        NodeDef {
            kind: "Print",
            category: "Depuração",
            label: "Imprimir",
            description: "Escreve um texto no console do editor.",
            event: false,
            inputs: vec![
                exec(),
                PinDef::new("msg", "Texto", PinType::String).padrao(Value::String(String::new())),
            ],
            outputs: vec![exec()],
        },
        NodeDef {
            kind: "Destroy",
            category: "Nó",
            label: "Destruir nó",
            description: "Remove o nó da cena.",
            event: false,
            inputs: vec![exec(), PinDef::new("node", "Nó", PinType::Node)],
            outputs: vec![exec()],
        },
        NodeDef {
            kind: "ActionPressed",
            category: "Entrada",
            label: "Ação pressionada",
            description: "Verdadeiro enquanto a ação de entrada está pressionada.",
            event: false,
            inputs: vec![PinDef::new("action", "Ação", PinType::String)],
            outputs: vec![PinDef::new("pressed", "Pressionada", PinType::Bool)],
        },
        NodeDef {
            kind: "GetProperty",
            category: "Propriedades",
            label: "Ler propriedade",
            description: "Lê uma propriedade numérica de um nó.",
            event: false,
            inputs: vec![
                PinDef::new("node", "Nó", PinType::Node),
                PinDef::new("name", "Propriedade", PinType::String),
            ],
            outputs: vec![PinDef::new("value", "Valor", PinType::Float)],
        },
        NodeDef {
            kind: "SetProperty",
            category: "Propriedades",
            label: "Definir propriedade",
            description: "Grava uma propriedade numérica de um nó.",
            event: false,
            inputs: vec![
                exec(),
                PinDef::new("node", "Nó", PinType::Node),
                PinDef::new("name", "Propriedade", PinType::String),
                decimal("value", "Valor", 0.0),
            ],
            outputs: vec![exec()],
        },
        NodeDef {
            kind: "GetVariable",
            category: "Variáveis",
            label: "Ler variável",
            description: "Lê uma variável do script. O nome é fixo no nó.",
            event: false,
            inputs: vec![PinDef::new("name", "Variável", PinType::String)],
            outputs: vec![PinDef::of_variable("value", "Valor")],
        },
        NodeDef {
            kind: "SetVariable",
            category: "Variáveis",
            label: "Definir variável",
            description: "Grava uma variável do script. O nome é fixo no nó.",
            event: false,
            inputs: vec![
                exec(),
                PinDef::new("name", "Variável", PinType::String),
                PinDef::of_variable("value", "Valor"),
            ],
            outputs: vec![exec()],
        },
        NodeDef {
            kind: "Reroute",
            category: "Organização",
            label: "Ponto de passagem (Reroute)",
            description: "Passa o fio adiante sem mudar nada. O tipo é o do fio que chega.",
            event: false,
            inputs: vec![PinDef::segue_o_fio("in", "Entrada")],
            outputs: vec![PinDef::segue_o_fio("out", "Saída")],
        },
    ]
}

/// Todos os tipos de nó da v1, na ordem em que aparecem no menu.
pub fn library() -> &'static [NodeDef] {
    static LIB: OnceLock<Vec<NodeDef>> = OnceLock::new();
    LIB.get_or_init(build)
}

/// Definição do tipo de nó com o `kind` dado.
pub fn find(kind: &str) -> Option<&'static NodeDef> {
    library().iter().find(|d| d.kind == kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn biblioteca_v1_serializa_para_o_editor() {
        assert_eq!(library().len(), 15);
        assert!(find("Branch").is_some_and(|d| !d.event));
        assert!(find("OnProcess").is_some_and(|d| d.event));
        assert!(find("Nada").is_none());
        // O editor lê esta tabela como JSON.
        let json = serde_json::to_value(library()).unwrap();
        assert_eq!(json[0]["kind"], "OnReady");
    }
}
