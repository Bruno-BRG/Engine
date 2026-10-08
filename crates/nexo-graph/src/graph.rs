//! Modelo do grafo de um script (VIS-020, VIS-022): nós, fios e variáveis. É o que o editor edita e o que
//! vai para `scripts/*.graph.ron`. `connect` e `add_variable` validam na hora, para o editor mostrar o motivo
//! de uma recusa sem esperar a compilação.

use crate::library::{self, NodeDef, PinDef};
use crate::types::{PinType, Value, Variable};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// Versão do formato do arquivo de grafo, no mesmo espírito do formato de cena (SCN-004).
pub const FORMAT: u32 = 1;

/// Um pino de um nó: o id do nó e o nome do pino.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PinRef {
    pub node: u64,
    pub pin: String,
}

/// Fio que sai da saída `from` e chega na entrada `to`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wire {
    pub from: PinRef,
    pub to: PinRef,
}

/// Um nó do grafo: uma instância de um tipo da biblioteca.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    /// Id único dentro do grafo, a partir de 1.
    pub id: u64,
    /// Tipo na biblioteca, por exemplo "Add" ou "OnProcess".
    pub kind: String,
    pub position: [f64; 2],
    /// Valores embutidos nas entradas. Têm prioridade sobre o padrão da biblioteca.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, Value>,
}

impl GraphNode {
    /// Rótulo para as mensagens de erro, por exemplo "Add#3".
    pub(crate) fn etiqueta(&self) -> String {
        format!("{}#{}", self.kind, self.id)
    }
}

/// Grafo de um script. Os campos são públicos para o editor e os testes. `connect` e `add_variable`
/// são as entradas que validam.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Graph {
    pub nodes: Vec<GraphNode>,
    pub wires: Vec<Wire>,
    pub variables: Vec<Variable>,
}

/// Forma do arquivo RON. Como o `SceneFile` de nexo-core, o formato fica num campo à parte.
#[derive(Serialize, Deserialize)]
struct GraphFile {
    format: u32,
    #[serde(default)]
    nodes: Vec<GraphNode>,
    #[serde(default)]
    wires: Vec<Wire>,
    #[serde(default)]
    variables: Vec<Variable>,
}

/// Definição do tipo do nó na biblioteca, ou erro quando o tipo não existe.
pub(crate) fn def_of(node: &GraphNode) -> Result<&'static NodeDef, String> {
    library::find(&node.kind).ok_or_else(|| format!("tipo de nó desconhecido: '{}'", node.kind))
}

impl Graph {
    /// Texto RON do grafo, no formato atual.
    pub fn to_ron(&self) -> Result<String, String> {
        let file = GraphFile {
            format: FORMAT,
            nodes: self.nodes.clone(),
            wires: self.wires.clone(),
            variables: self.variables.clone(),
        };
        let cfg = ron::ser::PrettyConfig::new().struct_names(false);
        ron::ser::to_string_pretty(&file, cfg).map_err(|e| e.to_string())
    }

    /// Lê o texto RON de um grafo. Recusa formatos que este crate não conhece.
    pub fn from_ron(text: &str) -> Result<Graph, String> {
        let file: GraphFile = ron::from_str(text).map_err(|e| format!("Grafo ilegível: {e}"))?;
        if file.format != FORMAT {
            return Err(format!("Formato de grafo desconhecido: {}", file.format));
        }
        Ok(Graph {
            nodes: file.nodes,
            wires: file.wires,
            variables: file.variables,
        })
    }

    /// Nó com o id dado.
    pub fn node(&self, id: u64) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Variável com o nome dado.
    pub fn variable(&self, name: &str) -> Option<&Variable> {
        self.variables.iter().find(|v| v.name == name)
    }

    /// Adiciona um nó do tipo `kind` e devolve o id novo.
    pub fn add_node(&mut self, kind: &str, position: [f64; 2]) -> Result<u64, String> {
        if library::find(kind).is_none() {
            return Err(format!("tipo de nó desconhecido: '{kind}'"));
        }
        let id = self.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
        self.nodes.push(GraphNode {
            id,
            kind: kind.to_string(),
            position,
            values: BTreeMap::new(),
        });
        Ok(id)
    }

    /// Grava o valor embutido de uma entrada de dados. O tipo do valor tem de ser o do pino.
    pub fn set_value(&mut self, node: u64, pin: &str, value: Value) -> Result<(), String> {
        let (ty, etiqueta) = {
            let n = self
                .node(node)
                .ok_or_else(|| format!("nó {node} não existe no grafo"))?;
            (self.input_type(n, pin)?, n.etiqueta())
        };
        if value.ty() != ty {
            return Err(format!(
                "{etiqueta}: a entrada '{pin}' é {} e o valor é {}",
                ty.nome(),
                value.ty().nome()
            ));
        }
        if let Some(n) = self.nodes.iter_mut().find(|n| n.id == node) {
            n.values.insert(pin.to_string(), value);
        }
        Ok(())
    }

    /// Declara uma variável do script. O padrão tem de ser do mesmo tipo da variável.
    pub fn add_variable(&mut self, name: &str, ty: PinType, default: Value, exported: bool) -> Result<(), String> {
        if name.trim().is_empty() {
            return Err("nome de variável vazio".into());
        }
        if ty == PinType::Flow {
            return Err(format!("variável '{name}' não pode ser do tipo fluxo"));
        }
        if default.ty() != ty {
            return Err(format!(
                "padrão de '{name}' é {} mas a variável é {}",
                default.ty().nome(),
                ty.nome()
            ));
        }
        if self.variable(name).is_some() {
            return Err(format!("já existe uma variável '{name}'"));
        }
        self.variables.push(Variable {
            name: name.to_string(),
            ty,
            default,
            exported,
        });
        Ok(())
    }

    /// Liga a saída `from` (nó, pino) à entrada `to`. Recusa pino inexistente, tipos diferentes, entrada já
    /// ligada e ciclo. Em caso de erro, o grafo fica como estava.
    pub fn connect(&mut self, from: (u64, &str), to: (u64, &str)) -> Result<(), String> {
        let from = PinRef {
            node: from.0,
            pin: from.1.to_string(),
        };
        let to = PinRef {
            node: to.0,
            pin: to.1.to_string(),
        };
        let ty = self.endpoint_type(&from, &to)?;
        // Cada entrada aceita um fio só. Vale para fluxo e para dados.
        if self.wires.iter().any(|w| w.to == to) {
            return Err(format!("a entrada '{}' do nó {} já está conectada", to.pin, to.node));
        }
        // Há ciclo se o nó de destino já alcança o de origem pelos fios da mesma classe.
        let flow = ty == PinType::Flow;
        if self.reaches(to.node, from.node, flow) {
            let classe = if flow { "de fluxo" } else { "de dados" };
            return Err(format!(
                "ligação criaria um ciclo {classe}: o nó {} já alcança o nó {}",
                to.node, from.node
            ));
        }
        self.wires.push(Wire { from, to });
        Ok(())
    }

    /// Tipo do fio de `from` até `to`, se as duas pontas existem, têm o pino e o tipo bate.
    pub(crate) fn endpoint_type(&self, from: &PinRef, to: &PinRef) -> Result<PinType, String> {
        let a = self
            .node(from.node)
            .ok_or_else(|| format!("nó {} não existe no grafo", from.node))?;
        let b = self
            .node(to.node)
            .ok_or_else(|| format!("nó {} não existe no grafo", to.node))?;
        let out = self.output_type(a, &from.pin)?;
        let inp = self.input_type(b, &to.pin)?;
        if out != inp {
            return Err(format!(
                "tipos incompatíveis: {}.{} ({}) não pode ligar em {}.{} ({})",
                a.etiqueta(),
                from.pin,
                out.nome(),
                b.etiqueta(),
                to.pin,
                inp.nome()
            ));
        }
        Ok(out)
    }

    /// Tipo de uma saída. Nós de variável seguem o tipo da variável que nomeiam.
    pub(crate) fn output_type(&self, node: &GraphNode, pin: &str) -> Result<PinType, String> {
        let def = def_of(node)?;
        let p = def
            .outputs
            .iter()
            .find(|p| p.name == pin)
            .ok_or_else(|| format!("{} não tem a saída '{pin}'", node.etiqueta()))?;
        self.resolve(node, p)
    }

    /// Tipo de uma entrada. Nós de variável seguem o tipo da variável que nomeiam.
    pub(crate) fn input_type(&self, node: &GraphNode, pin: &str) -> Result<PinType, String> {
        let def = def_of(node)?;
        let p = def
            .inputs
            .iter()
            .find(|p| p.name == pin)
            .ok_or_else(|| format!("{} não tem a entrada '{pin}'", node.etiqueta()))?;
        self.resolve(node, p)
    }

    fn resolve(&self, node: &GraphNode, pin: &PinDef) -> Result<PinType, String> {
        match pin.ty {
            Some(ty) => Ok(ty),
            None => Ok(self.variable_of(node)?.ty),
        }
    }

    /// Variável nomeada pelo nó. O nome é um valor embutido: não pode vir de fio, porque a variável é
    /// fixa na compilação.
    pub(crate) fn variable_of(&self, node: &GraphNode) -> Result<&Variable, String> {
        let name = match node.values.get("name") {
            Some(Value::String(s)) => s.as_str(),
            _ => return Err(format!("{} precisa do nome da variável embutido", node.etiqueta())),
        };
        self.variable(name)
            .ok_or_else(|| format!("variável '{name}' não existe"))
    }

    /// True se `goal` é alcançável a partir de `start`, seguindo só os fios de fluxo (`flow` = true) ou só os
    /// de dados (`flow` = false).
    fn reaches(&self, start: u64, goal: u64, flow: bool) -> bool {
        let mut pilha = vec![start];
        let mut vistos = HashSet::new();
        while let Some(id) = pilha.pop() {
            if id == goal {
                return true;
            }
            if !vistos.insert(id) {
                continue;
            }
            for w in self.wires.iter().filter(|w| w.from.node == id) {
                if self.is_flow_wire(w) == flow {
                    pilha.push(w.to.node);
                }
            }
        }
        false
    }

    fn is_flow_wire(&self, w: &Wire) -> bool {
        self.node(w.from.node)
            .and_then(|n| self.output_type(n, &w.from.pin).ok())
            == Some(PinType::Flow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adiciona(g: &mut Graph, kind: &str) -> u64 {
        g.add_node(kind, [0.0, 0.0]).unwrap()
    }

    #[test]
    fn tipo_incompativel_e_recusado() {
        let mut g = Graph::default();
        let cmp = adiciona(&mut g, "Greater");
        let soma = adiciona(&mut g, "Add");
        // Booleano não liga em decimal.
        let erro = g.connect((cmp, "result"), (soma, "a")).unwrap_err();
        assert!(erro.contains("incompatíveis"), "{erro}");
        assert!(g.wires.is_empty());
    }

    #[test]
    fn no_e_pino_desconhecidos_sao_recusados() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let erro = g.connect((ev, "exec"), (99, "exec")).unwrap_err();
        assert!(erro.contains("não existe"), "{erro}");
        let soma = adiciona(&mut g, "Add");
        let erro = g.connect((ev, "exec"), (soma, "entrada_falsa")).unwrap_err();
        assert!(erro.contains("não tem a entrada"), "{erro}");
    }

    #[test]
    fn entrada_ja_ligada_e_recusada() {
        let mut g = Graph::default();
        let a = adiciona(&mut g, "Add");
        let b = adiciona(&mut g, "Add");
        let m = adiciona(&mut g, "Multiply");
        g.connect((a, "result"), (m, "a")).unwrap();
        let erro = g.connect((b, "result"), (m, "a")).unwrap_err();
        assert!(erro.contains("já está conectada"), "{erro}");
    }

    #[test]
    fn ciclo_de_fluxo_e_recusado() {
        let mut g = Graph::default();
        let a = adiciona(&mut g, "Sequence");
        let b = adiciona(&mut g, "Sequence");
        g.connect((a, "then_0"), (b, "exec")).unwrap();
        let erro = g.connect((b, "then_0"), (a, "exec")).unwrap_err();
        assert!(erro.contains("ciclo de fluxo"), "{erro}");
        // Um fio que volta para o próprio nó também é ciclo.
        let c = adiciona(&mut g, "Sequence");
        let erro = g.connect((c, "then_0"), (c, "exec")).unwrap_err();
        assert!(erro.contains("ciclo de fluxo"), "{erro}");
    }

    #[test]
    fn ciclo_de_dados_e_recusado() {
        let mut g = Graph::default();
        let soma = adiciona(&mut g, "Add");
        let erro = g.connect((soma, "result"), (soma, "a")).unwrap_err();
        assert!(erro.contains("ciclo de dados"), "{erro}");
    }

    #[test]
    fn tipo_de_no_desconhecido_e_recusado() {
        let mut g = Graph::default();
        assert!(
            g.add_node("NaoExiste", [0.0, 0.0])
                .unwrap_err()
                .contains("desconhecido")
        );
    }

    #[test]
    fn variavel_com_padrao_de_outro_tipo_e_recusada() {
        let mut g = Graph::default();
        let erro = g
            .add_variable("vida", PinType::Float, Value::Bool(true), false)
            .unwrap_err();
        assert!(erro.contains("decimal"), "{erro}");
        g.add_variable("vida", PinType::Float, Value::Float(10.0), true)
            .unwrap();
        assert!(
            g.add_variable("vida", PinType::Float, Value::Float(1.0), false)
                .is_err()
        );
    }

    #[test]
    fn grafo_ida_e_volta_pelo_ron() {
        let mut g = Graph::default();
        let ev = g.add_node("OnProcess", [-120.0, 40.5]).unwrap();
        let soma = g.add_node("Add", [60.0, 40.5]).unwrap();
        g.set_value(soma, "b", Value::Float(2.5)).unwrap();
        g.connect((ev, "delta"), (soma, "a")).unwrap();
        g.add_variable("velocidade", PinType::Float, Value::Float(3.0), true)
            .unwrap();

        let texto = g.to_ron().unwrap();
        assert!(texto.contains("velocidade"));
        assert_eq!(Graph::from_ron(&texto).unwrap(), g);
        assert!(
            Graph::from_ron("(format: 9, nodes: [])")
                .unwrap_err()
                .contains("desconhecido")
        );
    }
}
