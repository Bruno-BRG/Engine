//! Modelo do grafo de um script (VIS-020, VIS-014, VIS-022): nós, fios, comentários e variáveis. É o que o
//! editor edita e o que vai para `scripts/*.graph.ron`. `connect`, `add_variable` e `validate` checam na hora,
//! para o editor mostrar o motivo de uma recusa sem esperar a compilação.

use crate::library::{self, NodeDef, PinDef};
use crate::types::{PinType, Value, Variable};
use nexo_core::registry::{self, PropKind};
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

/// Tipo resolvido de um pino (VIS-013). `ty` vazio quer dizer que o pino ainda não tem tipo: um reroute sem
/// fio de entrada, ou uma variável que não existe.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PinInfo {
    pub node: u64,
    pub pin: String,
    pub output: bool,
    pub ty: Option<PinType>,
}

/// Ids novos do que foi colado (VIS-012): os nós e as caixas, na ordem do trecho.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Colados {
    pub nodes: Vec<u64>,
    pub comments: Vec<u64>,
}

/// Trecho copiado do grafo (VIS-012): os nós, os fios entre eles, as caixas de comentário e as variáveis que os
/// nós usam. Colar gera ids novos, então o trecho cola no mesmo grafo ou em outro script.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Fragment {
    pub nodes: Vec<GraphNode>,
    pub wires: Vec<Wire>,
    pub comments: Vec<Comment>,
    #[serde(default)]
    pub variables: Vec<Variable>,
}

/// Validation or compile error. `node` is the node where the problem was found, when there is one. `suggestion`
/// says how to fix it, when the error is a connection refusal with a clear fix (VIS-013).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileError {
    pub node: Option<u64>,
    pub message: String,
    pub suggestion: Option<&'static str>,
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
        suggestion: None,
    }
}

/// Validation error built from a refusal: the message and the suggestion both come from it.
pub(crate) fn error_from_refusal(node: u64, refusal: &Refusal) -> CompileError {
    CompileError {
        node: Some(node),
        message: refusal.message(),
        suggestion: Some(refusal.suggestion()),
    }
}

/// Why a connection is refused, or why the graph is invalid (VIS-013). `message()` says what happened and is the
/// text the editor shows, in Portuguese. `suggestion()` says how to fix it. The messages are the ones the editor
/// already showed: `collapse_to_function` and the tests compare them by substring.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The node does not exist in the graph.
    MissingNode { node: u64 },
    /// The node has no output (`output` is true) or no input (`output` is false) called `pin`. `label` is the
    /// node label, for example "Add#3".
    MissingPin {
        label: String,
        pin: String,
        output: bool,
    },
    /// The node kind is not in the library.
    UnknownKind { kind: String },
    /// The two ends have different types. `from` and `to` are "node.pin", with the node label.
    IncompatibleTypes {
        from: String,
        ty_from: PinType,
        to: String,
        ty_to: PinType,
    },
    /// The input already has a wire. Applies to flow and data inputs.
    InputOccupied { node: u64, pin: String },
    /// The connection would close a cycle of the given class (flow or data). `from` is the output, `to` the input.
    CycleOnConnect { flow: bool, from: u64, to: u64 },
    /// Validation found a cycle that is already in the graph.
    CycleInGraph { flow: bool, node: u64 },
    /// Reroute with no wire into its input, so its type is still undefined.
    RerouteWithoutInput { node: u64 },
    /// A chain of reroutes that loops back on itself.
    RerouteCycle { node: u64 },
    /// Variable node without its name set.
    VariableWithoutName { label: String },
    /// The name points to a variable the script does not declare.
    UnknownVariable { name: String },
    /// Input with more than one wire (only possible in a graph edited by hand).
    DuplicateInput { node: u64, pin: String },
    /// Required input with no wire and no value. `field` is true when the node shows an inline field for it.
    RequiredInputMissing {
        label: String,
        pin: String,
        field: bool,
    },
    /// Property node whose declared type is not in the registry.
    UnknownNodeType { label: String, node_type: String },
    /// Property node whose property is not in its declared type, nor in the ancestors of that type.
    UnknownProperty {
        label: String,
        node_type: String,
        name: String,
    },
    /// Property node whose property is a list of points, which property nodes do not read yet.
    PointsProperty { label: String, name: String },
    /// Property node with a declared type and no embedded property name.
    PropertyWithoutName { label: String },
    /// Type or name of a property node comes from a wire. The compiler needs them as embedded values.
    WiredPropertyPin { label: String, pin: String },
}

/// Name of a cycle class, as it appears in the messages.
fn cycle_class(flow: bool) -> &'static str {
    if flow { "de fluxo" } else { "de dados" }
}

impl Refusal {
    /// What happened, in Portuguese. This is the text the editor shows.
    pub fn message(&self) -> String {
        match self {
            Refusal::MissingNode { node } => format!("nó {node} não existe no grafo"),
            Refusal::MissingPin {
                label,
                pin,
                output: true,
            } => format!("{label} não tem a saída '{pin}'"),
            Refusal::MissingPin {
                label,
                pin,
                output: false,
            } => format!("{label} não tem a entrada '{pin}'"),
            Refusal::UnknownKind { kind } => format!("tipo de nó desconhecido: '{kind}'"),
            Refusal::IncompatibleTypes {
                from,
                ty_from,
                to,
                ty_to,
            } => format!(
                "tipos incompatíveis: {from} ({}) não pode ligar em {to} ({})",
                ty_from.nome(),
                ty_to.nome()
            ),
            Refusal::InputOccupied { node, pin } => {
                format!("a entrada '{pin}' do nó {node} já está conectada")
            }
            Refusal::CycleOnConnect { flow, from, to } => format!(
                "ligação criaria um ciclo {}: o nó {to} já alcança o nó {from}",
                cycle_class(*flow)
            ),
            Refusal::CycleInGraph { flow, node } => {
                format!("ciclo {} no nó {node}", cycle_class(*flow))
            }
            Refusal::RerouteWithoutInput { node } => {
                format!("reroute {node} sem fio de entrada: tipo indefinido")
            }
            Refusal::RerouteCycle { node } => format!("ciclo de reroute no nó {node}"),
            Refusal::VariableWithoutName { label } => {
                format!("{label} precisa do nome da variável embutido")
            }
            Refusal::UnknownVariable { name } => format!("variável '{name}' não existe"),
            Refusal::DuplicateInput { node, pin } => {
                format!("a entrada '{pin}' do nó {node} está ligada mais de uma vez")
            }
            Refusal::RequiredInputMissing { label, pin, .. } => {
                format!("entrada obrigatória '{pin}' de {label} não está conectada")
            }
            Refusal::UnknownNodeType { label, node_type } => {
                format!("{label}: o tipo de nó '{node_type}' não existe")
            }
            Refusal::UnknownProperty {
                label,
                node_type,
                name,
            } => format!("{label}: {node_type} não tem a propriedade '{name}'"),
            Refusal::PointsProperty { label, name } => {
                format!("{label}: '{name}' é uma lista de pontos, que os nós de propriedade ainda não leem")
            }
            Refusal::PropertyWithoutName { label } => {
                format!("{label} precisa do nome da propriedade embutido")
            }
            Refusal::WiredPropertyPin { label, pin } => {
                format!("{label}: '{pin}' precisa ser um valor embutido, não pode vir de fio")
            }
        }
    }

    /// How to fix it, in Portuguese, using the labels the editor shows. Every refusal has one.
    pub fn suggestion(&self) -> &'static str {
        match self {
            Refusal::MissingNode { .. }
            | Refusal::MissingPin { .. }
            | Refusal::UnknownKind { .. } => "Recrie o nó pela busca",
            Refusal::IncompatibleTypes { .. } => {
                "Ligue a um pino da mesma cor e forma, ou solte no vazio para buscar um compatível (fluxo só liga a fluxo)"
            }
            Refusal::InputOccupied { .. } => "Remova o fio atual antes",
            Refusal::CycleOnConnect { flow: true, .. }
            | Refusal::CycleInGraph { flow: true, .. } => {
                "Para repetir, use Enquanto (While) ou Para cada (For Each) em vez de um fio de volta"
            }
            Refusal::CycleOnConnect { flow: false, .. }
            | Refusal::CycleInGraph { flow: false, .. } => {
                "Guarde com Definir variável e leia com Ler variável"
            }
            Refusal::RerouteWithoutInput { .. } => "Ligue a entrada do reroute primeiro",
            Refusal::RerouteCycle { .. } => "Remova um dos fios da cadeia de reroutes",
            Refusal::VariableWithoutName { .. } => "Escreva o nome da variável no campo do nó",
            Refusal::UnknownVariable { .. } => "Crie no painel Variáveis ou escolha outra",
            Refusal::DuplicateInput { .. } => "Remova um dos fios que chegam nela",
            Refusal::RequiredInputMissing { field: true, .. } => {
                "Ligue um fio ou escreva o valor no campo do nó"
            }
            Refusal::RequiredInputMissing { field: false, .. } => "Ligue um fio a essa entrada",
            Refusal::UnknownNodeType { .. } => "Escolha um tipo da lista do nó",
            Refusal::UnknownProperty { .. } => "Escolha uma propriedade da lista do nó",
            Refusal::PointsProperty { .. } => "Escolha uma propriedade que não seja lista de pontos",
            Refusal::PropertyWithoutName { .. } => "Escolha a propriedade no campo do nó",
            Refusal::WiredPropertyPin { .. } => {
                "Escreva o valor no campo do nó, em vez de ligar um fio"
            }
        }
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

/// Par (nó, pino) como `PinRef`.
fn pin_ref((node, pin): (u64, &str)) -> PinRef {
    PinRef {
        node,
        pin: pin.to_string(),
    }
}

/// Definição do tipo do nó na biblioteca, ou erro quando o tipo não existe.
pub(crate) fn def_of(node: &GraphNode) -> Result<&'static NodeDef, Refusal> {
    library::find(&node.kind).ok_or_else(|| Refusal::UnknownKind {
        kind: node.kind.clone(),
    })
}

/// Nodes that read or write a registry property (VIS-021). Their value pin follows the property's type.
pub(crate) fn is_property_node(kind: &str) -> bool {
    matches!(kind, "GetProperty" | "SetProperty")
}

/// What a property node declares (VIS-021): the type of its value pin, and the value that pin uses without a wire.
struct DeclaredProperty {
    ty: PinType,
    default: Option<Value>,
}

/// The property a property node declares. Refuses when the type is not in the registry, when the property is not in
/// that type or its ancestors, when no name is embedded, and when the property is a list of points. A vector
/// component ("position_x") is decimal, and its default is that component of the vector's default.
fn declared_property(node: &GraphNode) -> Result<DeclaredProperty, Refusal> {
    let node_type = texto_do(node, "type");
    let name = texto_do(node, "name");
    let label = node.etiqueta();
    if registry::find(&node_type).is_none() {
        return Err(Refusal::UnknownNodeType { label, node_type });
    }
    if name.is_empty() {
        return Err(Refusal::PropertyWithoutName { label });
    }
    if let Some((vector, index)) = registry::componente(&name) {
        let def = registry::prop_def(&node_type, vector)
            .filter(|d| d.kind == PropKind::Vec2)
            .ok_or(Refusal::UnknownProperty {
                label,
                node_type,
                name,
            })?;
        let default = match Value::from_json(PropKind::Vec2, &def.default) {
            Some(Value::Vec2(pair)) => pair.get(index).copied().unwrap_or(0.0),
            _ => 0.0,
        };
        return Ok(DeclaredProperty {
            ty: PinType::Float,
            default: Some(Value::Float(default)),
        });
    }
    let def = registry::prop_def(&node_type, &name).ok_or_else(|| Refusal::UnknownProperty {
        label: label.clone(),
        node_type: node_type.clone(),
        name: name.clone(),
    })?;
    let ty = PinType::of_prop(def.kind).ok_or(Refusal::PointsProperty { label, name })?;
    Ok(DeclaredProperty {
        ty,
        default: Value::from_json(def.kind, &def.default),
    })
}

/// Type of the value pin of a property node (VIS-021). Without a declared type it is decimal, as before typed
/// properties, so old graphs keep working. With a declared type, it is the type of the property in the registry.
pub(crate) fn property_pin_type(node: &GraphNode) -> Result<PinType, Refusal> {
    if texto_do(node, "type").is_empty() {
        return Ok(PinType::Float);
    }
    declared_property(node).map(|declared| declared.ty)
}

/// Value an input uses when it has no wire and no embedded value (VIS-021). The value pin of a property node uses
/// the default of the declared property, or decimal zero without a declared type. Other pins use the library default.
pub(crate) fn input_default(node: &GraphNode, pin: &PinDef) -> Option<Value> {
    if is_property_node(&node.kind) && pin.name == "value" {
        if texto_do(node, "type").is_empty() {
            return Some(Value::Float(0.0));
        }
        return declared_property(node).ok().and_then(|declared| declared.default);
    }
    pin.default.clone()
}

/// Changing the type or the name of a property node drops its embedded value, which may be of another type (VIS-021).
fn clear_property_value(node: &mut GraphNode, pin: &str) {
    if is_property_node(&node.kind) && (pin == "type" || pin == "name") {
        node.values.remove("value");
    }
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
                w.from.node == from.0
                    && w.from.pin == from.1
                    && w.to.node == to.0
                    && w.to.pin == to.1
            })
            .ok_or_else(|| format!("não há fio de {}.{} para {}.{}", from.0, from.1, to.0, to.1))?;
        self.wires.remove(pos);
        Ok(())
    }

    /// Remove o fio que chega na entrada `to`, se houver. Devolve se removeu alguma coisa.
    pub fn disconnect_input(&mut self, to: (u64, &str)) -> bool {
        let antes = self.wires.len();
        self.wires
            .retain(|w| !(w.to.node == to.0 && w.to.pin == to.1));
        self.wires.len() != antes
    }

    /// Writes the embedded value of a data input; the value must have the pin's type. Changing the type or name
    /// of a property node drops the wires of its value pin that no longer match, and returns them (VIS-021).
    pub fn set_value(&mut self, node: u64, pin: &str, value: Value) -> Result<Vec<Wire>, String> {
        let (ty, etiqueta) = {
            let n = self
                .node(node)
                .ok_or_else(|| format!("nó {node} não existe no grafo"))?;
            (
                self.input_type(n, pin).map_err(|e| e.message())?,
                n.etiqueta(),
            )
        };
        if value.ty() != ty {
            return Err(format!(
                "{etiqueta}: a entrada '{pin}' é {} e o valor é {}",
                ty.nome(),
                value.ty().nome()
            ));
        }
        let changed = match self.nodes.iter_mut().find(|n| n.id == node) {
            Some(n) => {
                let changed = n.values.get(pin) != Some(&value);
                n.values.insert(pin.to_string(), value);
                if changed {
                    clear_property_value(n, pin);
                }
                changed
            }
            None => false,
        };
        if changed && (pin == "type" || pin == "name") {
            return Ok(self.drop_stale_value_wires(node));
        }
        Ok(Vec::new())
    }

    /// Declara uma variável do script. O padrão tem de ser do mesmo tipo da variável.
    pub fn add_variable(
        &mut self,
        name: &str,
        ty: PinType,
        default: Value,
        exported: bool,
    ) -> Result<(), String> {
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

    /// Troca o texto, a posição e o tamanho de uma caixa de comentário (VIS-014).
    pub fn set_comment(
        &mut self,
        id: u64,
        text: &str,
        position: [f64; 2],
        size: [f64; 2],
    ) -> Result<(), String> {
        let c = self
            .comments
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or_else(|| format!("comentário {id} não existe no grafo"))?;
        c.text = text.to_string();
        c.position = position;
        c.size = size;
        Ok(())
    }

    /// Move um nó ou um reroute (VIS-012). Não mexe nos fios: eles seguem os pinos.
    pub fn set_position(&mut self, id: u64, position: [f64; 2]) -> Result<(), String> {
        if let Some(n) = self.nodes.iter_mut().find(|n| n.id == id) {
            n.position = position;
            return Ok(());
        }
        let c = self
            .comments
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or_else(|| format!("nó {id} não existe no grafo"))?;
        c.position = position;
        Ok(())
    }

    /// Tira o valor embutido de uma entrada: ela volta ao padrão da biblioteca. Devolve se havia valor.
    pub fn remove_value(&mut self, node: u64, pin: &str) -> Vec<Wire> {
        let existed = match self.nodes.iter_mut().find(|n| n.id == node) {
            Some(n) => {
                let existed = n.values.remove(pin).is_some();
                if existed {
                    clear_property_value(n, pin);
                }
                existed
            }
            None => false,
        };
        if existed && (pin == "type" || pin == "name") {
            return self.drop_stale_value_wires(node);
        }
        Vec::new()
    }

    /// Removes the wires of a property node's value pin that no longer type-check after its type or name changed
    /// (VIS-021), and returns them. Does nothing for other nodes.
    fn drop_stale_value_wires(&mut self, node: u64) -> Vec<Wire> {
        let mut removed = Vec::new();
        if !self.node(node).is_some_and(|n| is_property_node(&n.kind)) {
            return removed;
        }
        while let Some(i) = self.stale_value_wire(node) {
            removed.push(self.wires.remove(i));
        }
        removed
    }

    /// Index of the first wire that is tied to the value of `node` and no longer type-checks. Tied wires are the
    /// input wire into `value`, the output wires from `value`, and the wires that leave the reroutes it feeds.
    fn stale_value_wire(&self, node: u64) -> Option<usize> {
        // (node, pin) pairs that carry the value of `node`: its `value` output, and the outputs of the reroutes it feeds.
        let mut carriers: HashSet<(u64, String)> = HashSet::from([(node, "value".to_string())]);
        let mut grew = true;
        while grew {
            grew = false;
            for w in &self.wires {
                let feeds_reroute = self.node(w.to.node).is_some_and(|n| n.kind == REROUTE);
                if feeds_reroute && carriers.contains(&(w.from.node, w.from.pin.clone())) {
                    grew |= carriers.insert((w.to.node, "out".to_string()));
                }
            }
        }
        self.wires.iter().position(|w| {
            let into_value = w.to.node == node && w.to.pin == "value";
            let from_value = carriers.contains(&(w.from.node, w.from.pin.clone()));
            (into_value || from_value) && self.endpoint_type(&w.from, &w.to).is_err()
        })
    }

    /// Declara ou troca uma variável (VIS-022). Se o nome já existe, a variável é trocada no mesmo lugar, para a
    /// ordem da lista não mudar. As mesmas regras de `add_variable` valem.
    pub fn set_variable(
        &mut self,
        name: &str,
        ty: PinType,
        default: Value,
        exported: bool,
    ) -> Result<(), String> {
        if self.variable(name).is_none() {
            return self.add_variable(name, ty, default, exported);
        }
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
        if let Some(v) = self.variables.iter_mut().find(|v| v.name == name) {
            *v = Variable {
                name: name.to_string(),
                ty,
                default,
                exported,
            };
        }
        Ok(())
    }

    /// Copia um trecho: os nós e as caixas pedidos, os fios que ficam inteiros dentro da seleção e as variáveis
    /// que os nós usam (VIS-012). Não muda o grafo.
    pub fn copy_fragment(&self, nodes: &[u64], comments: &[u64]) -> Fragment {
        let nos: HashSet<u64> = nodes.iter().copied().collect();
        let caixas: HashSet<u64> = comments.iter().copied().collect();
        let copied: Vec<GraphNode> = self
            .nodes
            .iter()
            .filter(|n| nos.contains(&n.id))
            .cloned()
            .collect();
        let mut variables: Vec<Variable> = Vec::new();
        for n in &copied {
            if n.kind != "GetVariable" && n.kind != "SetVariable" {
                continue;
            }
            if let Some(v) = self.variable(&texto_do(n, "name"))
                && !variables.iter().any(|x| x.name == v.name)
            {
                variables.push(v.clone());
            }
        }
        Fragment {
            nodes: copied,
            wires: self
                .wires
                .iter()
                .filter(|w| nos.contains(&w.from.node) && nos.contains(&w.to.node))
                .cloned()
                .collect(),
            comments: self
                .comments
                .iter()
                .filter(|c| caixas.contains(&c.id))
                .cloned()
                .collect(),
            variables,
        }
    }

    /// Cola um trecho deslocado por (dx, dy) e devolve os ids novos dos nós, na ordem do trecho. Recusa o trecho
    /// inteiro se ele usa um tipo de nó que não existe ou uma variável com outro tipo; nesse caso o grafo não muda.
    /// Variáveis que o trecho usa e o grafo não tem são criadas.
    pub fn paste_fragment(&mut self, f: &Fragment, dx: f64, dy: f64) -> Result<Colados, String> {
        for n in &f.nodes {
            if library::find(&n.kind).is_none() {
                return Err(format!("tipo de nó desconhecido: '{}'", n.kind));
            }
        }
        for v in &f.variables {
            if self.variable(&v.name).is_some_and(|atual| atual.ty != v.ty) {
                return Err(format!(
                    "já existe a variável '{}' com outro tipo",
                    v.name
                ));
            }
        }
        for v in &f.variables {
            if self.variable(&v.name).is_none() {
                self.variables.push(v.clone());
            }
        }
        let mut mapa: BTreeMap<u64, u64> = BTreeMap::new();
        let mut nos = Vec::with_capacity(f.nodes.len());
        let mut caixas = Vec::with_capacity(f.comments.len());
        for n in &f.nodes {
            let id = self.novo_id();
            mapa.insert(n.id, id);
            let mut copia = n.clone();
            copia.id = id;
            copia.position = [n.position[0] + dx, n.position[1] + dy];
            self.nodes.push(copia);
            nos.push(id);
        }
        for c in &f.comments {
            let id = self.novo_id();
            self.comments.push(Comment {
                id,
                text: c.text.clone(),
                position: [c.position[0] + dx, c.position[1] + dy],
                size: c.size,
            });
            caixas.push(id);
        }
        for w in &f.wires {
            if let (Some(&de), Some(&para)) = (mapa.get(&w.from.node), mapa.get(&w.to.node)) {
                self.wires.push(Wire {
                    from: PinRef {
                        node: de,
                        pin: w.from.pin.clone(),
                    },
                    to: PinRef {
                        node: para,
                        pin: w.to.pin.clone(),
                    },
                });
            }
        }
        Ok(Colados {
            nodes: nos,
            comments: caixas,
        })
    }

    /// Colapsa uma seleção de nós de fluxo num nó Função com o nome dado (VIS-015). O corpo da função recebe os nós
    /// selecionados, ligados como estavam. No lugar da seleção fica um Chamar função, ligado ao fluxo de fora.
    /// Só colapsa o que as funções já expressam: no máximo uma entrada e uma saída de fluxo na fronteira, e
    /// nenhum fio de dados cruzando a fronteira (funções ainda não têm pinos, VIS-026). Macros ficam para depois.
    /// Em caso de erro, o grafo fica como estava. Devolve o id do Chamar função.
    pub fn collapse_to_function(&mut self, nodes: &[u64], name: &str) -> Result<u64, String> {
        let antes = self.clone();
        let antigos: Vec<String> = antes.validate().into_iter().map(|e| e.message).collect();
        let res = self.colapsa(nodes, name).and_then(|c| {
            match self.validate().into_iter().find(|e| !antigos.contains(&e.message)) {
                Some(e) => Err(e.message),
                None => Ok(c),
            }
        });
        if res.is_err() {
            *self = antes;
        }
        res
    }

    fn colapsa(&mut self, nodes: &[u64], name: &str) -> Result<u64, String> {
        let nome = name.trim();
        if nome.is_empty() {
            return Err("a função precisa de um nome".into());
        }
        if self
            .nodes
            .iter()
            .any(|n| n.kind == "Function" && texto_do(n, "name") == nome)
        {
            return Err(format!("função '{nome}' repetida"));
        }
        let sel: HashSet<u64> = nodes.iter().copied().collect();
        let dentro: Vec<&GraphNode> = self.nodes.iter().filter(|n| sel.contains(&n.id)).collect();
        if dentro.is_empty() {
            return Err("nenhum nó selecionado".into());
        }
        for n in &dentro {
            let def = def_of(n).map_err(|e| e.message())?;
            if def.event || n.kind == "Function" {
                return Err(format!("{} não pode entrar numa função", n.etiqueta()));
            }
        }
        let mut entradas = Vec::new();
        let mut saidas = Vec::new();
        for w in &self.wires {
            let de_dentro = sel.contains(&w.from.node);
            if de_dentro == sel.contains(&w.to.node) {
                continue;
            }
            if !self.is_flow_wire(w) {
                return Err(format!(
                    "o fio de dados de {}.{} atravessa a seleção: funções ainda não têm pinos de entrada e saída",
                    w.from.node, w.from.pin
                ));
            }
            if de_dentro {
                saidas.push(w.clone());
            } else {
                entradas.push(w.clone());
            }
        }
        if entradas.len() > 1 {
            return Err("a seleção tem mais de uma entrada de fluxo".into());
        }
        if saidas.len() > 1 {
            return Err("a seleção tem mais de uma saída de fluxo".into());
        }

        let n = dentro.len() as f64;
        let centro = [
            dentro.iter().map(|n| n.position[0]).sum::<f64>() / n,
            dentro.iter().map(|n| n.position[1]).sum::<f64>() / n,
        ];
        let topo = dentro
            .iter()
            .map(|n| n.position[1])
            .fold(f64::INFINITY, f64::min);
        let esquerda = dentro
            .iter()
            .map(|n| n.position[0])
            .fold(f64::INFINITY, f64::min);
        let f = self.add_node("Function", [esquerda, topo - 160.0])?;
        self.set_value(f, "name", Value::String(nome.to_string()))?;
        let c = self.add_node("CallFunction", centro)?;
        self.set_value(c, "name", Value::String(nome.to_string()))?;

        self.wires
            .retain(|w| !entradas.contains(w) && !saidas.contains(w));
        let exec = |node: u64| PinRef {
            node,
            pin: "exec".into(),
        };
        if let Some(e) = entradas.first() {
            // Quem chamava a entrada agora chama o Chamar função, e o corpo começa onde a entrada tocava.
            self.wires.push(Wire {
                from: e.from.clone(),
                to: exec(c),
            });
            self.wires.push(Wire {
                from: exec(f),
                to: e.to.clone(),
            });
        }
        if let Some(s) = saidas.first() {
            // Quando a função termina, o fluxo segue para onde a saída ia.
            self.wires.push(Wire {
                from: exec(c),
                to: s.to.clone(),
            });
        }
        Ok(c)
    }

    /// Liga a saída `from` (nó, pino) à entrada `to`. Recusa pino inexistente, tipos diferentes, entrada já
    /// ligada e ciclo. Em caso de erro, o grafo fica como estava.
    pub fn connect(&mut self, from: (u64, &str), to: (u64, &str)) -> Result<(), String> {
        self.can_connect(from, to).map_err(|e| e.message())?;
        self.wires.push(Wire {
            from: pin_ref(from),
            to: pin_ref(to),
        });
        Ok(())
    }

    /// Whether `connect` would accept the link, without changing the graph. Returns the wire type, or the refusal
    /// (VIS-013). The editor shows the refusal and its suggestion when a wire is dropped on a pin.
    pub fn can_connect(&self, from: (u64, &str), to: (u64, &str)) -> Result<PinType, Refusal> {
        let from = pin_ref(from);
        let to = pin_ref(to);
        let ty = self.endpoint_type(&from, &to)?;
        // Cada entrada aceita um fio só. Vale para fluxo e para dados.
        if self.wires.iter().any(|w| w.to == to) {
            return Err(Refusal::InputOccupied {
                node: to.node,
                pin: to.pin,
            });
        }
        // Há ciclo se o nó de destino já alcança o de origem pelos fios da mesma classe.
        let flow = ty == PinType::Flow;
        if self.reaches(to.node, from.node, flow) {
            return Err(Refusal::CycleOnConnect {
                flow,
                from: from.node,
                to: to.node,
            });
        }
        Ok(ty)
    }

    /// Tipo resolvido de todos os pinos do grafo (VIS-013): o que o editor usa para colorir e dar forma aos pinos,
    /// e para destacar as entradas que aceitam um fio arrastado. `ty` vem vazio quando o pino ainda não tem tipo.
    pub fn pins(&self) -> Vec<PinInfo> {
        let mut out = Vec::new();
        for node in &self.nodes {
            let Ok(def) = def_of(node) else { continue };
            for p in &def.inputs {
                out.push(PinInfo {
                    node: node.id,
                    pin: p.name.to_string(),
                    output: false,
                    ty: self.input_type(node, p.name).ok(),
                });
            }
            for p in &def.outputs {
                out.push(PinInfo {
                    node: node.id,
                    pin: p.name.to_string(),
                    output: true,
                    ty: self.output_type(node, p.name).ok(),
                });
            }
        }
        out
    }

    /// Tipo do fio de `from` até `to`, se as duas pontas existem, têm o pino e o tipo bate. A entrada de um
    /// reroute aceita qualquer tipo: o reroute copia o tipo da saída que o alimenta.
    pub(crate) fn endpoint_type(&self, from: &PinRef, to: &PinRef) -> Result<PinType, Refusal> {
        let a = self
            .node(from.node)
            .ok_or(Refusal::MissingNode { node: from.node })?;
        let b = self
            .node(to.node)
            .ok_or(Refusal::MissingNode { node: to.node })?;
        let out = self.output_type(a, &from.pin)?;
        let inp = if b.kind == REROUTE && to.pin == "in" {
            out
        } else {
            self.input_type(b, &to.pin)?
        };
        if out != inp {
            return Err(Refusal::IncompatibleTypes {
                from: format!("{}.{}", a.etiqueta(), from.pin),
                ty_from: out,
                to: format!("{}.{}", b.etiqueta(), to.pin),
                ty_to: inp,
            });
        }
        Ok(out)
    }

    /// Tipo de uma saída. Nós de variável seguem o tipo da variável que nomeiam; reroutes seguem o fio.
    pub(crate) fn output_type(&self, node: &GraphNode, pin: &str) -> Result<PinType, Refusal> {
        let def = def_of(node)?;
        let p = def
            .outputs
            .iter()
            .find(|p| p.name == pin)
            .ok_or_else(|| Refusal::MissingPin {
                label: node.etiqueta(),
                pin: pin.to_string(),
                output: true,
            })?;
        self.resolve(node, p)
    }

    /// Tipo de uma entrada. Nós de variável seguem o tipo da variável que nomeiam; reroutes seguem o fio.
    pub(crate) fn input_type(&self, node: &GraphNode, pin: &str) -> Result<PinType, Refusal> {
        let def = def_of(node)?;
        let p = def
            .inputs
            .iter()
            .find(|p| p.name == pin)
            .ok_or_else(|| Refusal::MissingPin {
                label: node.etiqueta(),
                pin: pin.to_string(),
                output: false,
            })?;
        self.resolve(node, p)
    }

    fn resolve(&self, node: &GraphNode, pin: &PinDef) -> Result<PinType, Refusal> {
        if node.kind == REROUTE {
            return self.tipo_do_reroute(node);
        }
        match pin.ty {
            Some(ty) => Ok(ty),
            None if is_property_node(&node.kind) => property_pin_type(node),
            None => Ok(self.variable_of(node)?.ty),
        }
    }

    /// Tipo que um reroute passa adiante: o da saída que o alimenta, seguindo a cadeia de reroutes. Erro se a
    /// cadeia não tem origem (entrada sem fio) ou se ela dá a volta sobre si mesma.
    fn tipo_do_reroute(&self, node: &GraphNode) -> Result<PinType, Refusal> {
        let mut vistos = HashSet::new();
        let mut atual = node.id;
        loop {
            if !vistos.insert(atual) {
                return Err(Refusal::RerouteCycle { node: atual });
            }
            let w = self
                .wires
                .iter()
                .find(|w| w.to.node == atual && w.to.pin == "in")
                .ok_or(Refusal::RerouteWithoutInput { node: atual })?;
            let origem = self
                .node(w.from.node)
                .ok_or(Refusal::MissingNode { node: w.from.node })?;
            if origem.kind == REROUTE {
                atual = origem.id;
                continue;
            }
            return self.output_type(origem, &w.from.pin);
        }
    }

    /// Variável nomeada pelo nó. O nome é um valor embutido: não pode vir de fio, porque a variável é fixa na
    /// compilação.
    pub(crate) fn variable_of(&self, node: &GraphNode) -> Result<&Variable, Refusal> {
        let name = match node.values.get("name") {
            Some(Value::String(s)) => s.as_str(),
            _ => {
                return Err(Refusal::VariableWithoutName {
                    label: node.etiqueta(),
                });
            }
        };
        self.variable(name).ok_or_else(|| Refusal::UnknownVariable {
            name: name.to_string(),
        })
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
                    suggestion: None,
                });
            }
        }

        for node in &self.nodes {
            let def = match def_of(node) {
                Ok(def) => def,
                Err(refusal) => {
                    erros.push(error_from_refusal(node.id, &refusal));
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
                    Err(refusal) => erros.push(error_from_refusal(node.id, &refusal)),
                }
            }
            // On a property node, an invalid type or name is reported below, once. It is not repeated as a missing value.
            let invalid_property = is_property_node(&node.kind) && property_pin_type(node).is_err();
            for p in &def.inputs {
                let wired = self
                    .wires
                    .iter()
                    .any(|w| w.to.node == node.id && w.to.pin == p.name);
                let invalid_property_value = invalid_property && p.name == "value";
                if !wired
                    && !node.values.contains_key(p.name)
                    && input_default(node, p).is_none()
                    && !invalid_property_value
                {
                    // Value inputs show an inline field on the node; flow and node inputs do not.
                    let field =
                        matches!(p.ty, Some(ty) if !matches!(ty, PinType::Flow | PinType::Node));
                    let refusal = Refusal::RequiredInputMissing {
                        label: node.etiqueta(),
                        pin: p.name.to_string(),
                        field,
                    };
                    erros.push(error_from_refusal(node.id, &refusal));
                }
            }
            if (node.kind == "GetVariable" || node.kind == "SetVariable")
                && let Err(refusal) = self.variable_of(node)
            {
                erros.push(error_from_refusal(node.id, &refusal));
            }
            if is_property_node(&node.kind) {
                // The compiler does not see the scene, so the type must be an embedded value. The name must be one too,
                // but only once a type is declared: a graph from before typed properties may read the name from a wire.
                let typed = !texto_do(node, "type").is_empty();
                let mut wired_pin_refused = false;
                for pin in ["type", "name"] {
                    let wired = self
                        .wires
                        .iter()
                        .any(|w| w.to.node == node.id && w.to.pin == pin);
                    if wired && (pin == "type" || typed) {
                        wired_pin_refused = true;
                        let refusal = Refusal::WiredPropertyPin {
                            label: node.etiqueta(),
                            pin: pin.to_string(),
                        };
                        erros.push(error_from_refusal(node.id, &refusal));
                    }
                }
                if !wired_pin_refused && let Err(refusal) = property_pin_type(node) {
                    erros.push(error_from_refusal(node.id, &refusal));
                }
            }
        }

        let mut ligadas = HashSet::new();
        for w in &self.wires {
            if let Err(refusal) = self.endpoint_type(&w.from, &w.to) {
                erros.push(error_from_refusal(w.to.node, &refusal));
                continue;
            }
            if !ligadas.insert(&w.to) {
                erros.push(error_from_refusal(
                    w.to.node,
                    &Refusal::DuplicateInput {
                        node: w.to.node,
                        pin: w.to.pin.clone(),
                    },
                ));
            }
        }

        let mut nomes = HashSet::new();
        for v in &self.variables {
            if v.name.trim().is_empty() {
                erros.push(CompileError {
                    node: None,
                    message: "nome de variável vazio".into(),
                    suggestion: None,
                });
            } else if !nomes.insert(&v.name) {
                erros.push(CompileError {
                    node: None,
                    message: format!("variável '{}' declarada mais de uma vez", v.name),
                    suggestion: None,
                });
            }
            if v.ty == PinType::Flow {
                erros.push(CompileError {
                    node: None,
                    message: format!("variável '{}' não pode ser do tipo fluxo", v.name),
                    suggestion: None,
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
                    suggestion: None,
                });
            }
        }

        if let Some(id) = self.ciclo(true) {
            let refusal = Refusal::CycleInGraph {
                flow: true,
                node: id,
            };
            erros.push(error_from_refusal(id, &refusal));
        }
        if let Some(id) = self.ciclo(false) {
            let refusal = Refusal::CycleInGraph {
                flow: false,
                node: id,
            };
            erros.push(error_from_refusal(id, &refusal));
        }

        // Funções e estados (VIS-026, VIS-027): nomes únicos, sem vazio, e referências que apontam para algo.
        let mut funcoes = HashSet::new();
        let mut estados = HashSet::new();
        for node in &self.nodes {
            match node.kind.as_str() {
                "Function" => {
                    let nome = texto_do(node, "name");
                    if nome.trim().is_empty() {
                        erros.push(erro(node.id, format!("{} sem nome", node.etiqueta())));
                    } else if !funcoes.insert(nome.clone()) {
                        erros.push(erro(node.id, format!("função '{nome}' repetida")));
                    }
                }
                "State" => {
                    let nome = texto_do(node, "state");
                    if nome.trim().is_empty() {
                        erros.push(erro(node.id, format!("{} sem nome", node.etiqueta())));
                    } else if !estados.insert(nome.clone()) {
                        erros.push(erro(node.id, format!("estado '{nome}' repetido")));
                    }
                }
                _ => {}
            }
        }
        for node in &self.nodes {
            match node.kind.as_str() {
                "CallFunction" => {
                    let nome = texto_do(node, "name");
                    if !funcoes.contains(&nome) {
                        erros.push(erro(node.id, format!("função '{nome}' não existe")));
                    }
                }
                "GoToState" | "StateEnter" | "StateExit" => {
                    let nome = texto_do(node, "state");
                    if !estados.contains(&nome) {
                        erros.push(erro(node.id, format!("estado '{nome}' não existe")));
                    }
                }
                _ => {}
            }
        }
        erros
    }
}

/// Texto embutido no pino do nó, ou vazio quando o pino não tem valor embutido (o padrão da biblioteca é "").
fn texto_do(node: &GraphNode, pin: &str) -> String {
    match node.values.get(pin) {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
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
        let erro = g
            .connect((ev, "exec"), (soma, "entrada_falsa"))
            .unwrap_err();
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
        g.set_value(le, "name", Value::String("nada".into()))
            .unwrap();
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
            from: PinRef {
                node: a,
                pin: "result".into(),
            },
            to: PinRef {
                node: b,
                pin: "a".into(),
            },
        });
        g.wires.push(Wire {
            from: PinRef {
                node: b,
                pin: "result".into(),
            },
            to: PinRef {
                node: a,
                pin: "a".into(),
            },
        });
        let erros = g.validate();
        assert!(
            erros.iter().any(|e| e.message.contains("ciclo de dados")),
            "{erros:?}"
        );
    }

    #[test]
    fn can_connect_diz_o_motivo_sem_mudar_o_grafo() {
        let mut g = Graph::default();
        let cmp = adiciona(&mut g, "Greater");
        let soma = adiciona(&mut g, "Add");
        let erro = g.can_connect((cmp, "result"), (soma, "a")).unwrap_err();
        assert!(erro.message().contains("incompatíveis"), "{erro:?}");
        let mult = adiciona(&mut g, "Multiply");
        assert_eq!(g.can_connect((soma, "result"), (mult, "a")), Ok(PinType::Float));
        assert!(g.wires.is_empty());
    }

    /// Every refusal has a non-empty message and suggestion: that is what the editor shows.
    fn assert_refusal_complete(refusal: &Refusal) {
        assert!(!refusal.message().trim().is_empty(), "{refusal:?}");
        assert!(!refusal.suggestion().trim().is_empty(), "{refusal:?}");
    }

    #[test]
    fn refusal_missing_node_has_message_and_suggestion() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let refusal = g.can_connect((ev, "exec"), (99, "exec")).unwrap_err();
        assert_eq!(refusal, Refusal::MissingNode { node: 99 });
        assert_refusal_complete(&refusal);
    }

    #[test]
    fn refusal_missing_input_pin_has_message_and_suggestion() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let soma = adiciona(&mut g, "Add");
        let refusal = g
            .can_connect((ev, "exec"), (soma, "entrada_falsa"))
            .unwrap_err();
        assert!(
            matches!(refusal, Refusal::MissingPin { output: false, .. }),
            "{refusal:?}"
        );
        assert_refusal_complete(&refusal);
    }

    #[test]
    fn refusal_missing_output_pin_has_message_and_suggestion() {
        let mut g = Graph::default();
        let soma = adiciona(&mut g, "Add");
        let mult = adiciona(&mut g, "Multiply");
        let refusal = g
            .can_connect((soma, "saida_falsa"), (mult, "a"))
            .unwrap_err();
        assert!(
            matches!(refusal, Refusal::MissingPin { output: true, .. }),
            "{refusal:?}"
        );
        assert!(
            refusal.message().contains("não tem a saída 'saida_falsa'"),
            "{refusal:?}"
        );
        assert_refusal_complete(&refusal);
    }

    #[test]
    fn refusal_unknown_kind_has_message_and_suggestion() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        // A node kind the library does not have: it only reaches the graph from a hand-edited file.
        g.nodes.push(GraphNode {
            id: 50,
            kind: "NaoExiste".into(),
            position: [0.0, 0.0],
            values: BTreeMap::new(),
        });
        let refusal = g.can_connect((ev, "exec"), (50, "exec")).unwrap_err();
        assert_eq!(
            refusal,
            Refusal::UnknownKind {
                kind: "NaoExiste".into()
            }
        );
        assert_refusal_complete(&refusal);
    }

    #[test]
    fn refusal_incompatible_types_has_message_and_suggestion() {
        let mut g = Graph::default();
        let cmp = adiciona(&mut g, "Greater");
        let soma = adiciona(&mut g, "Add");
        let refusal = g.can_connect((cmp, "result"), (soma, "a")).unwrap_err();
        assert!(
            matches!(refusal, Refusal::IncompatibleTypes { .. }),
            "{refusal:?}"
        );
        assert_refusal_complete(&refusal);
    }

    #[test]
    fn refusal_input_occupied_has_message_and_suggestion() {
        let mut g = Graph::default();
        let a = adiciona(&mut g, "Add");
        let b = adiciona(&mut g, "Add");
        let m = adiciona(&mut g, "Multiply");
        g.connect((a, "result"), (m, "a")).unwrap();
        let refusal = g.can_connect((b, "result"), (m, "a")).unwrap_err();
        assert_eq!(
            refusal,
            Refusal::InputOccupied {
                node: m,
                pin: "a".into()
            }
        );
        assert_refusal_complete(&refusal);
    }

    #[test]
    fn refusal_cycle_on_connect_has_message_and_suggestion() {
        let mut g = Graph::default();
        let a = adiciona(&mut g, "Sequence");
        let b = adiciona(&mut g, "Sequence");
        g.connect((a, "then_0"), (b, "exec")).unwrap();
        let flow = g.can_connect((b, "then_0"), (a, "exec")).unwrap_err();
        assert!(
            matches!(flow, Refusal::CycleOnConnect { flow: true, .. }),
            "{flow:?}"
        );
        assert_refusal_complete(&flow);
        let soma = adiciona(&mut g, "Add");
        let data = g.can_connect((soma, "result"), (soma, "a")).unwrap_err();
        assert!(
            matches!(data, Refusal::CycleOnConnect { flow: false, .. }),
            "{data:?}"
        );
        assert_refusal_complete(&data);
    }

    #[test]
    fn refusal_cycle_in_graph_flow_has_suggestion() {
        let mut g = Graph::default();
        let a = adiciona(&mut g, "Sequence");
        let b = adiciona(&mut g, "Sequence");
        // Flow cycle built by hand: validation points at the node, with the suggestion.
        g.wires.push(Wire {
            from: pin_ref((a, "then_0")),
            to: pin_ref((b, "exec")),
        });
        g.wires.push(Wire {
            from: pin_ref((b, "then_0")),
            to: pin_ref((a, "exec")),
        });
        let erro = g
            .validate()
            .into_iter()
            .find(|e| e.message.contains("ciclo de fluxo"))
            .expect("a validação acha o ciclo de fluxo");
        assert!(erro.node.is_some());
        assert_eq!(
            erro.suggestion,
            Some(
                "Para repetir, use Enquanto (While) ou Para cada (For Each) em vez de um fio de volta"
            )
        );
    }

    #[test]
    fn refusal_cycle_in_graph_data_has_suggestion() {
        let mut g = Graph::default();
        let a = adiciona(&mut g, "Add");
        let b = adiciona(&mut g, "Add");
        // Data cycle built by hand: validation points at the node, with the suggestion.
        g.wires.push(Wire {
            from: pin_ref((a, "result")),
            to: pin_ref((b, "a")),
        });
        g.wires.push(Wire {
            from: pin_ref((b, "result")),
            to: pin_ref((a, "a")),
        });
        let erro = g
            .validate()
            .into_iter()
            .find(|e| e.message.contains("ciclo de dados"))
            .expect("a validação acha o ciclo de dados");
        assert!(erro.node.is_some());
        assert_eq!(
            erro.suggestion,
            Some("Guarde com Definir variável e leia com Ler variável")
        );
    }

    #[test]
    fn refusal_reroute_without_input_has_message_and_suggestion() {
        let mut g = Graph::default();
        let r = g.add_reroute([0.0, 0.0]);
        let soma = adiciona(&mut g, "Add");
        let refusal = g.can_connect((r, "out"), (soma, "a")).unwrap_err();
        assert_eq!(refusal, Refusal::RerouteWithoutInput { node: r });
        assert_refusal_complete(&refusal);
    }

    #[test]
    fn refusal_reroute_cycle_has_message_and_suggestion() {
        let mut g = Graph::default();
        let r1 = g.add_reroute([0.0, 0.0]);
        let r2 = g.add_reroute([0.0, 0.0]);
        // Two reroutes feeding each other: built by hand, because connect would refuse it.
        g.wires.push(Wire {
            from: pin_ref((r1, "out")),
            to: pin_ref((r2, "in")),
        });
        g.wires.push(Wire {
            from: pin_ref((r2, "out")),
            to: pin_ref((r1, "in")),
        });
        let soma = adiciona(&mut g, "Add");
        let refusal = g.can_connect((r1, "out"), (soma, "a")).unwrap_err();
        assert!(
            matches!(refusal, Refusal::RerouteCycle { .. }),
            "{refusal:?}"
        );
        assert_refusal_complete(&refusal);
    }

    #[test]
    fn refusal_variable_without_name_has_message_and_suggestion() {
        let mut g = Graph::default();
        let le = adiciona(&mut g, "GetVariable");
        let soma = adiciona(&mut g, "Add");
        let refusal = g.can_connect((le, "value"), (soma, "a")).unwrap_err();
        assert!(
            matches!(refusal, Refusal::VariableWithoutName { .. }),
            "{refusal:?}"
        );
        assert_refusal_complete(&refusal);
    }

    #[test]
    fn refusal_unknown_variable_has_message_and_suggestion() {
        let mut g = Graph::default();
        let le = adiciona(&mut g, "GetVariable");
        g.set_value(le, "name", Value::String("nada".into()))
            .unwrap();
        let soma = adiciona(&mut g, "Add");
        let refusal = g.can_connect((le, "value"), (soma, "a")).unwrap_err();
        assert_eq!(
            refusal,
            Refusal::UnknownVariable {
                name: "nada".into()
            }
        );
        assert_refusal_complete(&refusal);
    }

    #[test]
    fn refusal_duplicate_input_has_suggestion() {
        let mut g = Graph::default();
        let a = adiciona(&mut g, "Add");
        let b = adiciona(&mut g, "Add");
        let m = adiciona(&mut g, "Multiply");
        // Two wires into the same input: only possible in a hand-edited graph.
        g.wires.push(Wire {
            from: pin_ref((a, "result")),
            to: pin_ref((m, "a")),
        });
        g.wires.push(Wire {
            from: pin_ref((b, "result")),
            to: pin_ref((m, "a")),
        });
        let erro = g
            .validate()
            .into_iter()
            .find(|e| e.message.contains("mais de uma vez"))
            .expect("a validação acha a entrada dupla");
        assert_eq!(erro.node, Some(m));
        assert!(erro.suggestion.is_some_and(|s| !s.is_empty()));
    }

    #[test]
    fn refusal_required_input_missing_has_suggestion() {
        let mut g = Graph::default();
        // A Sequence with no flow wire into its input: the input is required and has no inline field.
        adiciona(&mut g, "Sequence");
        let erro = g
            .validate()
            .into_iter()
            .find(|e| e.message.contains("entrada obrigatória"))
            .expect("a validação acha a entrada obrigatória");
        assert!(erro.node.is_some());
        assert_eq!(erro.suggestion, Some("Ligue um fio a essa entrada"));
        // A value input shows an inline field, so its suggestion points at the field.
        let com_campo = Refusal::RequiredInputMissing {
            label: "Soma#1".into(),
            pin: "a".into(),
            field: true,
        };
        assert_eq!(
            com_campo.suggestion(),
            "Ligue um fio ou escreva o valor no campo do nó"
        );
        assert_refusal_complete(&com_campo);
    }

    #[test]
    fn refusal_nothing_changes_graph() {
        let mut g = Graph::default();
        let cmp = adiciona(&mut g, "Greater");
        let soma = adiciona(&mut g, "Add");
        let a = adiciona(&mut g, "Add");
        let m = adiciona(&mut g, "Multiply");
        g.connect((a, "result"), (m, "a")).unwrap();
        let antes = g.clone();
        // Three refusals: incompatible types, occupied input, and a missing node.
        assert!(g.connect((cmp, "result"), (soma, "a")).is_err());
        assert!(g.connect((soma, "result"), (m, "a")).is_err());
        assert!(g.connect((a, "result"), (99, "a")).is_err());
        assert!(g.can_connect((cmp, "result"), (soma, "a")).is_err());
        assert_eq!(g, antes);
    }

    #[test]
    fn pins_mostram_tipo_resolvido_de_variavel_e_de_reroute() {
        let mut g = Graph::default();
        g.add_variable("vida", PinType::Int, Value::Int(3), false)
            .unwrap();
        let lê = adiciona(&mut g, "GetVariable");
        g.set_value(lê, "name", Value::String("vida".into())).unwrap();
        let r = g.add_reroute([0.0, 0.0]);
        let pinos = g.pins();
        let tipo = |node: u64, pin: &str| {
            pinos
                .iter()
                .find(|p| p.node == node && p.pin == pin)
                .and_then(|p| p.ty)
        };
        assert_eq!(tipo(lê, "value"), Some(PinType::Int));
        // Sem fio de entrada, o reroute ainda não tem tipo.
        assert_eq!(tipo(r, "out"), None);
        let soma = adiciona(&mut g, "Add");
        g.connect((lê, "value"), (r, "in")).unwrap();
        assert_eq!(g.pins().iter().find(|p| p.node == r && p.pin == "out").unwrap().ty, Some(PinType::Int));
        assert!(g.connect((r, "out"), (soma, "a")).unwrap_err().contains("incompatíveis"));
    }

    #[test]
    fn copia_e_cola_entre_grafos_com_ids_novos() {
        let mut origem = Graph::default();
        origem
            .add_variable("vida", PinType::Float, Value::Float(1.0), false)
            .unwrap();
        let ev = adiciona(&mut origem, "OnReady");
        let sv = adiciona(&mut origem, "SetVariable");
        origem.set_value(sv, "name", Value::String("vida".into())).unwrap();
        let soma = adiciona(&mut origem, "Add");
        origem.connect((ev, "exec"), (sv, "exec")).unwrap();
        origem.connect((soma, "result"), (sv, "value")).unwrap();
        let c = origem.add_comment("trecho", [5.0, 5.0], [80.0, 40.0]);
        let trecho = origem.copy_fragment(&[ev, sv, soma], &[c]);
        // A variável vai junto, porque o SetVariable a usa. Os dois fios ficam dentro da seleção.
        assert_eq!(trecho.variables.len(), 1);
        assert_eq!(trecho.wires.len(), 2);

        let mut destino = Graph::default();
        let antes = adiciona(&mut destino, "Add");
        let novos = destino.paste_fragment(&trecho, 40.0, 40.0).unwrap();
        assert_eq!(novos.nodes.len(), 3);
        assert_eq!(novos.comments.len(), 1);
        assert!(novos.nodes.iter().all(|id| *id > antes));
        assert!(destino.variable("vida").is_some());
        assert_eq!(destino.wires.len(), 2);
        assert_eq!(destino.comments[0].position, [45.0, 45.0]);
        assert!(destino.validate().is_empty(), "{:?}", destino.validate());
        // Colar de novo gera ids novos, sem reaproveitar nenhum.
        let de_novo = destino.paste_fragment(&trecho, 0.0, 0.0).unwrap();
        assert!(de_novo.nodes.iter().all(|id| !novos.nodes.contains(id)));
    }

    #[test]
    fn colar_recusa_variavel_de_outro_tipo_sem_mudar_o_grafo() {
        let mut origem = Graph::default();
        origem
            .add_variable("vida", PinType::Float, Value::Float(1.0), false)
            .unwrap();
        let lê = adiciona(&mut origem, "GetVariable");
        origem.set_value(lê, "name", Value::String("vida".into())).unwrap();
        let trecho = origem.copy_fragment(&[lê], &[]);

        let mut destino = Graph::default();
        destino
            .add_variable("vida", PinType::Bool, Value::Bool(true), false)
            .unwrap();
        let antes = destino.clone();
        assert!(destino.paste_fragment(&trecho, 0.0, 0.0).unwrap_err().contains("outro tipo"));
        assert_eq!(destino, antes);
    }

    #[test]
    fn colapsar_seleção_em_funcao_mantem_o_fluxo_e_compila() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let p1 = adiciona(&mut g, "Print");
        let p2 = adiciona(&mut g, "Print");
        let p3 = adiciona(&mut g, "Print");
        g.connect((ev, "exec"), (p1, "exec")).unwrap();
        g.connect((p1, "exec"), (p2, "exec")).unwrap();
        g.connect((p2, "exec"), (p3, "exec")).unwrap();
        let chamada = g.collapse_to_function(&[p1, p2], "Passos").unwrap();
        // O evento agora chama a função, e a função chega a p3 pelo fim do corpo.
        assert!(g.wires.iter().any(|w| w.from.node == ev && w.to.node == chamada));
        assert!(g.wires.iter().any(|w| w.from.node == chamada && w.to.node == p3));
        assert!(g.validate().is_empty(), "{:?}", g.validate());
        assert!(crate::ir::compile(&g).is_ok());
        // Sem a seleção, nada muda: o nome repetido e o fio de dados são recusados.
        let antes = g.clone();
        assert!(g.collapse_to_function(&[p3], "Passos").unwrap_err().contains("repetida"));
        assert_eq!(g, antes);
    }

    #[test]
    fn colapsar_recusa_fio_de_dados_na_fronteira() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let imp = adiciona(&mut g, "Print");
        let texto = adiciona(&mut g, "Concat");
        g.connect((ev, "exec"), (imp, "exec")).unwrap();
        g.connect((texto, "result"), (imp, "msg")).unwrap();
        // O Concat só carrega dado para o Print de fora: a seleção não pode virar função sem pinos.
        let erro = g.collapse_to_function(&[texto], "Texto").unwrap_err();
        assert!(erro.contains("atravessa"), "{erro}");
        assert_eq!(g.wires.len(), 2);
        assert!(g.nodes.iter().all(|n| n.kind != "Function"));
    }

    #[test]
    fn variavel_trocada_fica_no_mesmo_lugar_e_recusa_padrao_errado() {
        let mut g = Graph::default();
        g.add_variable("a", PinType::Float, Value::Float(1.0), false).unwrap();
        g.add_variable("b", PinType::Float, Value::Float(2.0), false).unwrap();
        g.set_variable("a", PinType::Float, Value::Float(9.0), true).unwrap();
        assert_eq!(g.variables[0].name, "a");
        assert_eq!(g.variables[0].default, Value::Float(9.0));
        assert!(g.variables[0].exported);
        assert!(g.set_variable("a", PinType::Bool, Value::Float(1.0), false).is_err());
        assert!(g.set_variable("c", PinType::Flow, Value::Bool(true), false).is_err());
    }

    #[test]
    fn remover_valor_volta_ao_padrao_e_mover_no_ou_comentario() {
        let mut g = Graph::default();
        let soma = adiciona(&mut g, "Add");
        g.set_value(soma, "a", Value::Float(4.0)).unwrap();
        assert!(g.remove_value(soma, "a").is_empty());
        assert!(!g.node(soma).unwrap().values.contains_key("a"));
        g.set_position(soma, [7.0, 8.0]).unwrap();
        assert_eq!(g.node(soma).unwrap().position, [7.0, 8.0]);
        let c = g.add_comment("oi", [0.0, 0.0], [10.0, 10.0]);
        g.set_comment(c, "tchau", [1.0, 2.0], [30.0, 40.0]).unwrap();
        assert_eq!(g.comments[0].text, "tchau");
        g.set_position(c, [3.0, 4.0]).unwrap();
        assert_eq!(g.comments[0].position, [3.0, 4.0]);
        assert!(g.set_position(99, [0.0, 0.0]).is_err());
    }

    #[test]
    fn evento_de_fisica_compila_com_delta() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnPhysicsProcess");
        let soma = adiciona(&mut g, "Add");
        g.connect((ev, "delta"), (soma, "a")).unwrap();
        g.set_value(soma, "b", Value::Float(0.0)).unwrap();
        // Sem nenhum fio de fluxo, o programa existe e fica vazio: o evento é reconhecido.
        let c = crate::ir::compile(&g).unwrap();
        assert_eq!(c.programs[0].event, "OnPhysicsProcess");
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
        assert!(
            erros.iter().any(|e| e.message.contains("mais de uma vez")),
            "{erros:?}"
        );
        assert!(
            erros.iter().any(|e| e.message.contains("padrão de 'vida'")),
            "{erros:?}"
        );
    }

    /// Property node with its type and name embedded, ready to wire.
    fn property_node(g: &mut Graph, kind: &str, node_type: &str, name: &str) -> u64 {
        let id = adiciona(g, kind);
        g.set_value(id, "type", Value::String(node_type.into())).unwrap();
        g.set_value(id, "name", Value::String(name.into())).unwrap();
        id
    }

    #[test]
    fn property_pin_follows_registry() {
        let mut g = Graph::default();
        let cases = [
            ("modulate", PinType::String),
            ("position", PinType::Vec2),
            ("centered", PinType::Bool),
            ("z_index", PinType::Int),
            ("rotation", PinType::Float),
            ("shape", PinType::String),
        ];
        for (name, expected) in cases {
            let id = property_node(&mut g, "GetProperty", "Sprite2D", name);
            let node = g.node(id).unwrap();
            assert_eq!(g.output_type(node, "value"), Ok(expected), "{name}");
        }
        // No declared type: the node reads decimal, as graphs from before typing did.
        let legacy = adiciona(&mut g, "GetProperty");
        g.set_value(legacy, "name", Value::String("position_x".into()))
            .unwrap();
        assert_eq!(
            g.output_type(g.node(legacy).unwrap(), "value"),
            Ok(PinType::Float)
        );
        // The pin shows with its resolved type for the editor.
        let pin = g.pins().into_iter().find(|p| p.node == legacy && p.pin == "value");
        assert_eq!(pin.and_then(|p| p.ty), Some(PinType::Float));
    }

    #[test]
    fn vector_component_is_decimal_with_the_vector_default() {
        let mut g = Graph::default();
        // position_x and scale_y are decimal components of the vector properties of Sprite2D.
        let read = property_node(&mut g, "GetProperty", "Sprite2D", "position_x");
        let write = property_node(&mut g, "SetProperty", "Sprite2D", "scale_y");
        assert_eq!(g.output_type(g.node(read).unwrap(), "value"), Ok(PinType::Float));
        // The value pin without a wire starts at the component of the vector default: scale is [1, 1].
        let node = g.node(write).unwrap();
        let pin = def_of(node).unwrap().inputs.iter().find(|p| p.name == "value").unwrap();
        assert_eq!(input_default(node, pin), Some(Value::Float(1.0)));
        let errors = g.validate();
        assert!(
            !errors.iter().any(|e| e.message.contains("scale_y") || e.message.contains("position_x")),
            "{errors:?}"
        );
        // A type without the vector property refuses the component, as it refuses any unknown name.
        property_node(&mut g, "GetProperty", "VirtualJoystick", "position_x");
        let errors = g.validate();
        assert!(errors.iter().any(|e| e.message.contains("'position_x'")), "{errors:?}");
    }

    #[test]
    fn validate_refuses_unknown_type_name_and_points() {
        let mut g = Graph::default();
        property_node(&mut g, "GetProperty", "Tipo9", "x");
        property_node(&mut g, "GetProperty", "Sprite2D", "nao_existe");
        property_node(&mut g, "SetProperty", "CollisionPolygon2D", "points");
        let errors = g.validate();
        assert!(
            errors.iter().any(|e| e.message.contains("'Tipo9' não existe")),
            "{errors:?}"
        );
        assert!(
            errors.iter().any(|e| e.message.contains("não tem a propriedade 'nao_existe'")),
            "{errors:?}"
        );
        assert!(
            errors.iter().any(|e| e.message.contains("lista de pontos")),
            "{errors:?}"
        );
        // Only the node's own error shows: an invalid property's value is not reported as "required".
        assert!(
            !errors.iter().any(|e| e.message.contains("obrigatória 'value'")),
            "{errors:?}"
        );
    }

    #[test]
    fn wired_type_and_name_are_refused() {
        let mut g = Graph::default();
        let concat = adiciona(&mut g, "Concat");
        let gp = property_node(&mut g, "GetProperty", "Sprite2D", "modulate");
        g.connect((concat, "result"), (gp, "type")).unwrap();
        g.connect((concat, "result"), (gp, "name")).unwrap();
        let errors = g.validate();
        assert!(
            errors.iter().any(|e| e.message.contains("'type' precisa ser um valor embutido")),
            "{errors:?}"
        );
        assert!(
            errors.iter().any(|e| e.message.contains("'name' precisa ser um valor embutido")),
            "{errors:?}"
        );
    }

    #[test]
    fn changing_type_or_name_clears_embedded_value() {
        let mut g = Graph::default();
        let sp = property_node(&mut g, "SetProperty", "Sprite2D", "modulate");
        g.set_value(sp, "value", Value::String("#ff0000".into())).unwrap();
        // Same name: nothing changes, the value stays.
        g.set_value(sp, "name", Value::String("modulate".into())).unwrap();
        assert!(g.node(sp).unwrap().values.contains_key("value"));
        // Other type: the value goes, because it may be of another type.
        g.set_value(sp, "type", Value::String("Node2D".into())).unwrap();
        assert!(!g.node(sp).unwrap().values.contains_key("value"));
        g.set_value(sp, "value", Value::String("#00ff00".into())).unwrap();
        g.set_value(sp, "name", Value::String("tag".into())).unwrap();
        assert!(!g.node(sp).unwrap().values.contains_key("value"));
        // Removing the type also removes the value: it goes back to decimal with node value.
        g.set_value(sp, "value", Value::String("gato".into())).unwrap();
        g.remove_value(sp, "type");
        assert!(!g.node(sp).unwrap().values.contains_key("value"));
    }

    #[test]
    fn value_without_wire_uses_property_default() {
        let mut g = Graph::default();
        // Colour property with node embedded value: the default comes from the registry, and the input is not required.
        property_node(&mut g, "SetProperty", "Sprite2D", "modulate");
        // No declared type: the default is decimal zero, as before.
        let legacy = adiciona(&mut g, "SetProperty");
        g.set_value(legacy, "name", Value::String("position_x".into()))
            .unwrap();
        let errors = g.validate();
        assert!(!errors.iter().any(|e| e.message.contains("'value'")), "{errors:?}");
        let default_node = g.node(legacy).unwrap();
        let input = def_of(default_node).unwrap().inputs.iter().find(|p| p.name == "value").unwrap();
        assert_eq!(input_default(default_node, input), Some(Value::Float(0.0)));
    }

    #[test]
    fn changing_name_drops_mismatched_value_wire() {
        let mut g = Graph::default();
        let reader = property_node(&mut g, "GetProperty", "Sprite2D", "visible");
        let writer = property_node(&mut g, "SetProperty", "Sprite2D", "visible");
        g.connect((reader, "value"), (writer, "value")).unwrap();
        // Another property of the same type: the wire stays.
        let dropped = g
            .set_value(writer, "name", Value::String("centered".into()))
            .unwrap();
        assert!(dropped.is_empty());
        assert_eq!(g.wires.len(), 1);
        // Vec2 in place of Bool: the wire goes, and it comes back in the response so the editor can warn.
        let dropped = g
            .set_value(writer, "name", Value::String("position".into()))
            .unwrap();
        assert_eq!(dropped.len(), 1);
        assert_eq!(dropped[0].to, PinRef { node: writer, pin: "value".into() });
        assert!(g.wires.is_empty());
    }

    #[test]
    fn changing_read_type_drops_wire_behind_reroute() {
        let mut g = Graph::default();
        let reader = property_node(&mut g, "GetProperty", "Node2D", "position");
        let reroute = g.add_reroute([0.0, 0.0]);
        let writer = property_node(&mut g, "SetProperty", "Node2D", "position");
        g.connect((reader, "value"), (reroute, "in")).unwrap();
        g.connect((reroute, "out"), (writer, "value")).unwrap();
        // Node2D.visible is Bool: the read now gives Bool, and the writer's Vec2 pin node longer accepts it.
        let dropped = g
            .set_value(reader, "name", Value::String("visible".into()))
            .unwrap();
        assert_eq!(dropped.len(), 1);
        assert_eq!(dropped[0].to.node, writer);
        // The wire into the reroute stays: the read still gives a value.
        assert_eq!(g.wires.len(), 1);
        assert_eq!(g.wires[0].to.node, reroute);
    }

    #[test]
    fn removing_type_returns_wires_that_turn_decimal() {
        let mut g = Graph::default();
        let reader = property_node(&mut g, "GetProperty", "Sprite2D", "visible");
        let writer = property_node(&mut g, "SetProperty", "Sprite2D", "visible");
        g.connect((reader, "value"), (writer, "value")).unwrap();
        // No declared type: the value pin becomes decimal, and the read's Bool node longer fits.
        let dropped = g.remove_value(writer, "type");
        assert_eq!(dropped.len(), 1);
        assert!(g.wires.is_empty());
    }
}
