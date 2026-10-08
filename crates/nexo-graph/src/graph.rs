//! Modelo do grafo de um script (VIS-020, VIS-014, VIS-022): nós, fios, comentários e variáveis. É o que o
//! editor edita e o que vai para `scripts/*.graph.ron`. `connect`, `add_variable` e `validate` checam na hora,
//! para o editor mostrar o motivo de uma recusa sem esperar a compilação.

use crate::library::{self, NodeDef, PinDef};
use crate::types::{PinType, Value, Variable};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fmt;

/// Versão do formato do arquivo de grafo. Os campos acrescentados depois (comentários, próximo id) têm valor
/// padrão, então um arquivo sem eles continua abrindo.
pub const FORMAT: u32 = 1;

/// Nome do nó de reroute (VIS-014): um ponto de passagem que não muda o fio. O tipo vem do fio que chega.
pub const REROUTE: &str = "Reroute";

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
    /// Id único dentro do grafo. Não muda e não é reaproveitado depois de remover o nó.
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

/// Caixa de comentário no canvas (VIS-014). Não entra na compilação.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    pub id: u64,
    pub text: String,
    pub position: [f64; 2],
    pub size: [f64; 2],
}

/// Erro de validação ou de compilação. `node` é o nó onde o problema foi achado, quando há um.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileError {
    pub node: Option<u64>,
    pub message: String,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CompileError {}

pub(crate) fn erro(node: u64, message: impl Into<String>) -> CompileError {
    CompileError {
        node: Some(node),
        message: message.into(),
    }
}

/// Grafo de um script. Os campos são públicos para o editor e os testes. `connect`, `add_variable` e
/// `validate` são as entradas que checam.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Graph {
    pub nodes: Vec<GraphNode>,
    pub wires: Vec<Wire>,
    pub variables: Vec<Variable>,
    pub comments: Vec<Comment>,
    /// Próximo id a tentar. Ids de nós e comentários nunca são reaproveitados.
    proximo_id: u64,
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
    #[serde(default)]
    comments: Vec<Comment>,
    #[serde(default)]
    proximo_id: u64,
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
            comments: self.comments.clone(),
            proximo_id: self.proximo_id,
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
            comments: file.comments,
            proximo_id: file.proximo_id,
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

    /// Id livre para um nó ou comentário. Maior que todo id em uso e maior que qualquer id já dado.
    fn novo_id(&mut self) -> u64 {
        let usado = self
            .nodes
            .iter()
            .map(|n| n.id)
            .chain(self.comments.iter().map(|c| c.id))
            .max()
            .unwrap_or(0);
        let id = self.proximo_id.max(usado + 1);
        self.proximo_id = id + 1;
        id
    }

    /// Adiciona um nó do tipo `kind` e devolve o id novo.
    pub fn add_node(&mut self, kind: &str, position: [f64; 2]) -> Result<u64, String> {
        if library::find(kind).is_none() {
            return Err(format!("tipo de nó desconhecido: '{kind}'"));
        }
        let id = self.novo_id();
        self.nodes.push(GraphNode {
            id,
            kind: kind.to_string(),
            position,
            values: BTreeMap::new(),
        });
        Ok(id)
    }

    /// Adiciona um reroute (VIS-014) e devolve o id. Ele começa sem tipo: o primeiro fio que chega define.
    pub fn add_reroute(&mut self, position: [f64; 2]) -> u64 {
        self.add_node(REROUTE, position)
            .expect("o reroute está sempre na biblioteca")
    }

    /// Remove o nó e todos os fios que o tocam (VIS-020). Os ids dos outros nós não mudam.
    pub fn remove_node(&mut self, id: u64) -> Result<(), String> {
        let pos = self
            .nodes
            .iter()
            .position(|n| n.id == id)
            .ok_or_else(|| format!("nó {id} não existe no grafo"))?;
        self.nodes.remove(pos);
        self.wires.retain(|w| w.from.node != id && w.to.node != id);
        Ok(())
    }

    /// Remove o fio exato de `from` até `to`. Erro se ele não existe.
    pub fn disconnect(&mut self, from: (u64, &str), to: (u64, &str)) -> Result<(), String> {
        let pos = self
            .wires
            .iter()
            .position(|w| {
                w.from.node == from.0 && w.from.pin == from.1 && w.to.node == to.0 && w.to.pin == to.1
            })
            .ok_or_else(|| format!("não há fio de {}.{} para {}.{}", from.0, from.1, to.0, to.1))?;
        self.wires.remove(pos);
        Ok(())
    }

    /// Remove o fio que chega na entrada `to`, se houver. Devolve se removeu alguma coisa.
    pub fn disconnect_input(&mut self, to: (u64, &str)) -> bool {
        let antes = self.wires.len();
        self.wires.retain(|w| !(w.to.node == to.0 && w.to.pin == to.1));
        self.wires.len() != antes
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

    /// Remove uma variável. Os nós que a nomeiam ficam inválidos até serem corrigidos; `validate` aponta.
    pub fn remove_variable(&mut self, name: &str) -> Result<(), String> {
        let pos = self
            .variables
            .iter()
            .position(|v| v.name == name)
            .ok_or_else(|| format!("variável '{name}' não existe"))?;
        self.variables.remove(pos);
        Ok(())
    }

    /// Adiciona uma caixa de comentário (VIS-014) e devolve o id.
    pub fn add_comment(&mut self, text: &str, position: [f64; 2], size: [f64; 2]) -> u64 {
        let id = self.novo_id();
        self.comments.push(Comment {
            id,
            text: text.to_string(),
            position,
            size,
        });
        id
    }

    /// Remove uma caixa de comentário. Comentários não têm fios, então nada mais muda.
    pub fn remove_comment(&mut self, id: u64) -> Result<(), String> {
        let pos = self
            .comments
            .iter()
            .position(|c| c.id == id)
            .ok_or_else(|| format!("comentário {id} não existe no grafo"))?;
        self.comments.remove(pos);
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

    /// Tipo do fio de `from` até `to`, se as duas pontas existem, têm o pino e o tipo bate. A entrada de um
    /// reroute aceita qualquer tipo: o reroute copia o tipo da saída que o alimenta.
    pub(crate) fn endpoint_type(&self, from: &PinRef, to: &PinRef) -> Result<PinType, String> {
        let a = self
            .node(from.node)
            .ok_or_else(|| format!("nó {} não existe no grafo", from.node))?;
        let b = self
            .node(to.node)
            .ok_or_else(|| format!("nó {} não existe no grafo", to.node))?;
        let out = self.output_type(a, &from.pin)?;
        let inp = if b.kind == REROUTE && to.pin == "in" {
            out
        } else {
            self.input_type(b, &to.pin)?
        };
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

    /// Tipo de uma saída. Nós de variável seguem o tipo da variável que nomeiam; reroutes seguem o fio.
    pub(crate) fn output_type(&self, node: &GraphNode, pin: &str) -> Result<PinType, String> {
        let def = def_of(node)?;
        let p = def
            .outputs
            .iter()
            .find(|p| p.name == pin)
            .ok_or_else(|| format!("{} não tem a saída '{pin}'", node.etiqueta()))?;
        self.resolve(node, p)
    }

    /// Tipo de uma entrada. Nós de variável seguem o tipo da variável que nomeiam; reroutes seguem o fio.
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
        if node.kind == REROUTE {
            return self.tipo_do_reroute(node);
        }
        match pin.ty {
            Some(ty) => Ok(ty),
            None => Ok(self.variable_of(node)?.ty),
        }
    }

    /// Tipo que um reroute passa adiante: o da saída que o alimenta, seguindo a cadeia de reroutes. Erro se a
    /// cadeia não tem origem (entrada sem fio) ou se ela dá a volta sobre si mesma.
    fn tipo_do_reroute(&self, node: &GraphNode) -> Result<PinType, String> {
        let mut vistos = HashSet::new();
        let mut atual = node.id;
        loop {
            if !vistos.insert(atual) {
                return Err(format!("ciclo de reroute no nó {atual}"));
            }
            let w = self
                .wires
                .iter()
                .find(|w| w.to.node == atual && w.to.pin == "in")
                .ok_or_else(|| format!("reroute {atual} sem fio de entrada: tipo indefinido"))?;
            let origem = self
                .node(w.from.node)
                .ok_or_else(|| format!("nó {} não existe no grafo", w.from.node))?;
            if origem.kind == REROUTE {
                atual = origem.id;
                continue;
            }
            return self.output_type(origem, &w.from.pin);
        }
    }

    /// Variável nomeada pelo nó. O nome é um valor embutido: não pode vir de fio, porque a variável é fixa na
    /// compilação.
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

    pub(crate) fn is_flow_wire(&self, w: &Wire) -> bool {
        self.node(w.from.node)
            .and_then(|n| self.output_type(n, &w.from.pin).ok())
            == Some(PinType::Flow)
    }

    /// Procura um ciclo nos fios de uma classe (fluxo ou dados). Devolve um nó do ciclo, se achar.
    fn ciclo(&self, flow: bool) -> Option<u64> {
        // 0 = não visitado, 1 = no caminho atual, 2 = já terminado.
        let mut cor: BTreeMap<u64, u8> = BTreeMap::new();
        self.nodes
            .iter()
            .find_map(|n| self.ciclo_a_partir(n.id, flow, &mut cor))
    }

    fn ciclo_a_partir(&self, id: u64, flow: bool, cor: &mut BTreeMap<u64, u8>) -> Option<u64> {
        match cor.get(&id).copied().unwrap_or(0) {
            1 => return Some(id),
            2 => return None,
            _ => {}
        }
        cor.insert(id, 1);
        for w in self
            .wires
            .iter()
            .filter(|w| w.from.node == id && self.is_flow_wire(w) == flow)
        {
            if let Some(achado) = self.ciclo_a_partir(w.to.node, flow, cor) {
                return Some(achado);
            }
        }
        cor.insert(id, 2);
        None
    }

    /// Validação completa do grafo, sem compilar (VIS-020). Confere ids, tipos de nó, valores embutidos,
    /// fios (tipo e entrada única), entradas obrigatórias de todos os nós (alcançados ou não), nomes de
    /// variável, padrões de variável e ciclos de fluxo e de dados. Devolve todos os erros de uma vez.
    pub fn validate(&self) -> Vec<CompileError> {
        let mut erros = Vec::new();
        let mut ids = HashSet::new();
        for id in self
            .nodes
            .iter()
            .map(|n| n.id)
            .chain(self.comments.iter().map(|c| c.id))
        {
            if !ids.insert(id) {
                erros.push(CompileError {
                    node: Some(id),
                    message: format!("id repetido: {id}"),
                });
            }
        }

        for node in &self.nodes {
            let def = match def_of(node) {
                Ok(def) => def,
                Err(msg) => {
                    erros.push(erro(node.id, msg));
                    continue;
                }
            };
            for (pin, value) in &node.values {
                match self.input_type(node, pin) {
                    Ok(ty) if value.ty() != ty => erros.push(erro(
                        node.id,
                        format!(
                            "valor de '{pin}' em {} é {} mas o pino é {}",
                            node.etiqueta(),
                            value.ty().nome(),
                            ty.nome()
                        ),
                    )),
                    Ok(_) => {}
                    Err(msg) => erros.push(erro(node.id, msg)),
                }
            }
            for p in &def.inputs {
                let ligada = self
                    .wires
                    .iter()
                    .any(|w| w.to.node == node.id && w.to.pin == p.name);
                if !ligada && !node.values.contains_key(p.name) && p.default.is_none() {
                    erros.push(erro(
                        node.id,
                        format!(
                            "entrada obrigatória '{}' de {} não está conectada",
                            p.name,
                            node.etiqueta()
                        ),
                    ));
                }
            }
            if node.kind == "GetVariable" || node.kind == "SetVariable" {
                if let Err(msg) = self.variable_of(node) {
                    erros.push(erro(node.id, msg));
                }
            }
        }

        let mut ligadas = HashSet::new();
        for w in &self.wires {
            if let Err(msg) = self.endpoint_type(&w.from, &w.to) {
                erros.push(erro(w.to.node, msg));
                continue;
            }
            if !ligadas.insert(&w.to) {
                erros.push(erro(
                    w.to.node,
                    format!(
                        "a entrada '{}' do nó {} está ligada mais de uma vez",
                        w.to.pin, w.to.node
                    ),
                ));
            }
        }

        let mut nomes = HashSet::new();
        for v in &self.variables {
            if v.name.trim().is_empty() {
                erros.push(CompileError {
                    node: None,
                    message: "nome de variável vazio".into(),
                });
            } else if !nomes.insert(&v.name) {
                erros.push(CompileError {
                    node: None,
                    message: format!("variável '{}' declarada mais de uma vez", v.name),
                });
            }
            if v.ty == PinType::Flow {
                erros.push(CompileError {
                    node: None,
                    message: format!("variável '{}' não pode ser do tipo fluxo", v.name),
                });
            }
            if v.default.ty() != v.ty {
                erros.push(CompileError {
                    node: None,
                    message: format!(
                        "padrão de '{}' é {} mas a variável é {}",
                        v.name,
                        v.default.ty().nome(),
                        v.ty.nome()
                    ),
                });
            }
        }

        if let Some(id) = self.ciclo(true) {
            erros.push(erro(id, format!("ciclo de fluxo no nó {id}")));
        }
        if let Some(id) = self.ciclo(false) {
            erros.push(erro(id, format!("ciclo de dados no nó {id}")));
        }
        erros
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

    #[test]
    fn remover_no_leva_os_fios_junto() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let p = adiciona(&mut g, "Print");
        let q = adiciona(&mut g, "Print");
        g.connect((ev, "exec"), (p, "exec")).unwrap();
        g.connect((p, "exec"), (q, "exec")).unwrap();
        g.remove_node(p).unwrap();
        assert!(g.wires.is_empty());
        assert!(g.node(p).is_none());
        assert!(g.remove_node(p).unwrap_err().contains("não existe"));
    }

    #[test]
    fn desconectar_remove_so_o_fio_pedido() {
        let mut g = Graph::default();
        let a = adiciona(&mut g, "Add");
        let b = adiciona(&mut g, "Add");
        let m = adiciona(&mut g, "Multiply");
        g.connect((a, "result"), (m, "a")).unwrap();
        g.connect((b, "result"), (m, "b")).unwrap();
        g.disconnect((a, "result"), (m, "a")).unwrap();
        assert_eq!(g.wires.len(), 1);
        assert_eq!(g.wires[0].to.pin, "b");
        assert!(g.disconnect((a, "result"), (m, "a")).is_err());
        assert!(g.disconnect_input((m, "b")));
        assert!(!g.disconnect_input((m, "b")));
        assert!(g.wires.is_empty());
    }

    #[test]
    fn ids_nao_sao_reaproveitados() {
        let mut g = Graph::default();
        let a = adiciona(&mut g, "Add");
        let b = adiciona(&mut g, "Add");
        g.remove_node(a).unwrap();
        // Remover o último id não pode fazer o próximo voltar a ser um id usado antes.
        g.remove_node(b).unwrap();
        let c = adiciona(&mut g, "Add");
        assert!(c > b, "novo id {c} reaproveitou {b}");
        let comentario = g.add_comment("oi", [0.0, 0.0], [10.0, 10.0]);
        assert!(comentario > c);
    }

    #[test]
    fn comentario_e_reroute_ida_e_volta_pelo_ron() {
        let mut g = Graph::default();
        g.add_comment("Movimento do jogador", [10.0, 20.0], [300.0, 120.0]);
        let r = g.add_reroute([50.0, 50.0]);
        let ev = adiciona(&mut g, "OnReady");
        g.connect((ev, "exec"), (r, "in")).unwrap();
        let texto = g.to_ron().unwrap();
        let de_volta = Graph::from_ron(&texto).unwrap();
        assert_eq!(de_volta, g);
        assert_eq!(de_volta.comments[0].text, "Movimento do jogador");
    }

    #[test]
    fn reroute_passa_o_tipo_do_fio() {
        let mut g = Graph::default();
        let soma = adiciona(&mut g, "Add");
        let r = g.add_reroute([0.0, 0.0]);
        let destino = adiciona(&mut g, "Add");
        let cmp = adiciona(&mut g, "Greater");
        let r_bool = g.add_reroute([0.0, 0.0]);
        let destino_bool = adiciona(&mut g, "Multiply");
        // Decimal atravessa o reroute e chega num pino decimal.
        g.connect((soma, "result"), (r, "in")).unwrap();
        g.connect((r, "out"), (destino, "a")).unwrap();
        // Booleano atravessa outro reroute, mas não entra num pino decimal.
        g.connect((cmp, "result"), (r_bool, "in")).unwrap();
        let erro = g.connect((r_bool, "out"), (destino_bool, "a")).unwrap_err();
        assert!(erro.contains("incompatíveis"), "{erro}");
        assert!(g.validate().iter().all(|e| e.node != Some(destino)));
    }

    #[test]
    fn reroute_sem_entrada_tem_tipo_indefinido() {
        let mut g = Graph::default();
        let r = g.add_reroute([0.0, 0.0]);
        let soma = adiciona(&mut g, "Add");
        let erro = g.connect((r, "out"), (soma, "a")).unwrap_err();
        assert!(erro.contains("tipo indefinido"), "{erro}");
    }

    #[test]
    fn validar_pega_entrada_obrigatoria_mesmo_sem_fluxo() {
        let mut g = Graph::default();
        // Nenhum evento alcança este nó, mas a validação completa aponta a entrada que falta.
        let sp = adiciona(&mut g, "SetProperty");
        let erros = g.validate();
        assert!(
            erros
                .iter()
                .any(|e| e.node == Some(sp) && e.message.contains("obrigatória")),
            "{erros:?}"
        );
    }

    #[test]
    fn validar_pega_variavel_inexistente() {
        let mut g = Graph::default();
        let le = adiciona(&mut g, "GetVariable");
        g.set_value(le, "name", Value::String("nada".into())).unwrap();
        let erros = g.validate();
        assert!(
            erros
                .iter()
                .any(|e| e.node == Some(le) && e.message.contains("não existe")),
            "{erros:?}"
        );
    }

    #[test]
    fn validar_pega_ciclo_de_dados_montado_a_mao() {
        let mut g = Graph::default();
        let a = adiciona(&mut g, "Add");
        let b = adiciona(&mut g, "Add");
        g.wires.push(Wire {
            from: PinRef { node: a, pin: "result".into() },
            to: PinRef { node: b, pin: "a".into() },
        });
        g.wires.push(Wire {
            from: PinRef { node: b, pin: "result".into() },
            to: PinRef { node: a, pin: "a".into() },
        });
        let erros = g.validate();
        assert!(erros.iter().any(|e| e.message.contains("ciclo de dados")), "{erros:?}");
    }

    #[test]
    fn validar_pega_variavel_repetida_e_padrao_errado() {
        let mut g = Graph::default();
        g.variables.push(Variable {
            name: "vida".into(),
            ty: PinType::Float,
            default: Value::Float(1.0),
            exported: false,
        });
        g.variables.push(Variable {
            name: "vida".into(),
            ty: PinType::Bool,
            default: Value::Float(2.0),
            exported: false,
        });
        let erros = g.validate();
        assert!(erros.iter().any(|e| e.message.contains("mais de uma vez")), "{erros:?}");
        assert!(erros.iter().any(|e| e.message.contains("padrão de 'vida'")), "{erros:?}");
    }
}
