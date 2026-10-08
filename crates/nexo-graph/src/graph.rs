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

/// Erro de validação ou de compilação. `node` é o nó onde o problema foi achado, quando há um. `suggestion` diz
/// como corrigir, quando o erro é uma recusa de ligação com caminho claro (VIS-013).
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

/// Erro de validação a partir de uma recusa: o texto e a sugestão vêm do motivo.
pub(crate) fn erro_de(node: u64, motivo: &Recusa) -> CompileError {
    CompileError {
        node: Some(node),
        message: motivo.message(),
        suggestion: Some(motivo.suggestion()),
    }
}

/// Por que uma ligação ou um fio é recusado (VIS-013). `message()` diz o que aconteceu e é o texto que o editor
/// mostra; `suggestion()` diz como corrigir. Os textos são os de sempre: `collapse_to_function` e os testes os
/// comparam por substring.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recusa {
    /// O nó não existe no grafo.
    NoAusente { node: u64 },
    /// O nó não tem a saída (`saida`) ou a entrada `pin`. `etiqueta` é o rótulo do nó, por exemplo "Add#3".
    PinoAusente {
        etiqueta: String,
        pin: String,
        saida: bool,
    },
    /// O tipo do nó não está na biblioteca.
    TipoDesconhecido { kind: String },
    /// As duas pontas têm tipos diferentes. `de` e `para` são "nó.pino", com o rótulo do nó.
    TiposIncompativeis {
        de: String,
        ty_de: PinType,
        para: String,
        ty_para: PinType,
    },
    /// A entrada já tem um fio. Vale para fluxo e para dados.
    EntradaOcupada { node: u64, pin: String },
    /// A ligação fecharia um ciclo da classe dada (`fluxo` ou dados). `de` é a saída, `para` a entrada.
    CicloAoLigar { fluxo: bool, de: u64, para: u64 },
    /// A validação achou um ciclo que já está no grafo.
    CicloNoGrafo { fluxo: bool, node: u64 },
    /// Reroute sem fio na entrada: o tipo dele ainda não existe.
    RerouteSemEntrada { node: u64 },
    /// Cadeia de reroutes que volta sobre si mesma.
    CicloReroute { node: u64 },
    /// Nó de variável sem o nome embutido.
    VariavelSemNome { etiqueta: String },
    /// O nome aponta para uma variável que o script não tem.
    VariavelInexistente { name: String },
    /// Entrada com mais de um fio (grafo editado à mão).
    EntradaDupla { node: u64, pin: String },
}

/// Texto da classe de ciclo, como aparece nas mensagens.
fn classe_de_ciclo(fluxo: bool) -> &'static str {
    if fluxo { "de fluxo" } else { "de dados" }
}

impl Recusa {
    /// O que aconteceu, em português. É o texto que o editor mostra.
    pub fn message(&self) -> String {
        match self {
            Recusa::NoAusente { node } => format!("nó {node} não existe no grafo"),
            Recusa::PinoAusente {
                etiqueta,
                pin,
                saida: true,
            } => format!("{etiqueta} não tem a saída '{pin}'"),
            Recusa::PinoAusente {
                etiqueta,
                pin,
                saida: false,
            } => format!("{etiqueta} não tem a entrada '{pin}'"),
            Recusa::TipoDesconhecido { kind } => format!("tipo de nó desconhecido: '{kind}'"),
            Recusa::TiposIncompativeis {
                de,
                ty_de,
                para,
                ty_para,
            } => format!(
                "tipos incompatíveis: {de} ({}) não pode ligar em {para} ({})",
                ty_de.nome(),
                ty_para.nome()
            ),
            Recusa::EntradaOcupada { node, pin } => {
                format!("a entrada '{pin}' do nó {node} já está conectada")
            }
            Recusa::CicloAoLigar { fluxo, de, para } => format!(
                "ligação criaria um ciclo {}: o nó {para} já alcança o nó {de}",
                classe_de_ciclo(*fluxo)
            ),
            Recusa::CicloNoGrafo { fluxo, node } => {
                format!("ciclo {} no nó {node}", classe_de_ciclo(*fluxo))
            }
            Recusa::RerouteSemEntrada { node } => {
                format!("reroute {node} sem fio de entrada: tipo indefinido")
            }
            Recusa::CicloReroute { node } => format!("ciclo de reroute no nó {node}"),
            Recusa::VariavelSemNome { etiqueta } => {
                format!("{etiqueta} precisa do nome da variável embutido")
            }
            Recusa::VariavelInexistente { name } => format!("variável '{name}' não existe"),
            Recusa::EntradaDupla { node, pin } => {
                format!("a entrada '{pin}' do nó {node} está ligada mais de uma vez")
            }
        }
    }

    /// Como corrigir. Toda recusa de ligação tem uma sugestão: o editor a mostra junto do motivo.
    pub fn suggestion(&self) -> &'static str {
        match self {
            Recusa::NoAusente { .. }
            | Recusa::PinoAusente { .. }
            | Recusa::TipoDesconhecido { .. } => "Recrie o nó pela busca",
            Recusa::TiposIncompativeis { .. } => {
                "Ligue a um pino da mesma cor e forma, ou solte no vazio para buscar um compatível (fluxo só liga a fluxo)"
            }
            Recusa::EntradaOcupada { .. } => "Remova o fio atual antes",
            Recusa::CicloAoLigar { fluxo: true, .. } | Recusa::CicloNoGrafo { fluxo: true, .. } => {
                "Use While ou ForEach para repetir"
            }
            Recusa::CicloAoLigar { fluxo: false, .. }
            | Recusa::CicloNoGrafo { fluxo: false, .. } => {
                "Guarde com SetVariable e leia com GetVariable"
            }
            Recusa::RerouteSemEntrada { .. } => "Ligue a entrada do reroute primeiro",
            Recusa::CicloReroute { .. } => "Remova um dos fios da cadeia de reroutes",
            Recusa::VariavelSemNome { .. } => "Escreva o nome da variável no campo do nó",
            Recusa::VariavelInexistente { .. } => "Crie no painel Variáveis ou escolha outra",
            Recusa::EntradaDupla { .. } => "Remova um dos fios que chegam nela",
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
pub(crate) fn def_of(node: &GraphNode) -> Result<&'static NodeDef, Recusa> {
    library::find(&node.kind).ok_or_else(|| Recusa::TipoDesconhecido {
        kind: node.kind.clone(),
    })
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

    /// Grava o valor embutido de uma entrada de dados. O tipo do valor tem de ser o do pino.
    pub fn set_value(&mut self, node: u64, pin: &str, value: Value) -> Result<(), String> {
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
        if let Some(n) = self.nodes.iter_mut().find(|n| n.id == node) {
            n.values.insert(pin.to_string(), value);
        }
        Ok(())
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
    pub fn remove_value(&mut self, node: u64, pin: &str) -> bool {
        self.nodes
            .iter_mut()
            .find(|n| n.id == node)
            .is_some_and(|n| n.values.remove(pin).is_some())
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

    /// Diz se `connect` aceitaria a ligação, sem mudar o grafo. Devolve o tipo do fio, ou o motivo da recusa
    /// (VIS-013): o editor mostra o motivo e a sugestão ao soltar o fio e ao passar sobre a entrada.
    pub fn can_connect(&self, from: (u64, &str), to: (u64, &str)) -> Result<PinType, Recusa> {
        let from = pin_ref(from);
        let to = pin_ref(to);
        let ty = self.endpoint_type(&from, &to)?;
        // Cada entrada aceita um fio só. Vale para fluxo e para dados.
        if self.wires.iter().any(|w| w.to == to) {
            return Err(Recusa::EntradaOcupada {
                node: to.node,
                pin: to.pin,
            });
        }
        // Há ciclo se o nó de destino já alcança o de origem pelos fios da mesma classe.
        let flow = ty == PinType::Flow;
        if self.reaches(to.node, from.node, flow) {
            return Err(Recusa::CicloAoLigar {
                fluxo: flow,
                de: from.node,
                para: to.node,
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
    pub(crate) fn endpoint_type(&self, from: &PinRef, to: &PinRef) -> Result<PinType, Recusa> {
        let a = self
            .node(from.node)
            .ok_or(Recusa::NoAusente { node: from.node })?;
        let b = self
            .node(to.node)
            .ok_or(Recusa::NoAusente { node: to.node })?;
        let out = self.output_type(a, &from.pin)?;
        let inp = if b.kind == REROUTE && to.pin == "in" {
            out
        } else {
            self.input_type(b, &to.pin)?
        };
        if out != inp {
            return Err(Recusa::TiposIncompativeis {
                de: format!("{}.{}", a.etiqueta(), from.pin),
                ty_de: out,
                para: format!("{}.{}", b.etiqueta(), to.pin),
                ty_para: inp,
            });
        }
        Ok(out)
    }

    /// Tipo de uma saída. Nós de variável seguem o tipo da variável que nomeiam; reroutes seguem o fio.
    pub(crate) fn output_type(&self, node: &GraphNode, pin: &str) -> Result<PinType, Recusa> {
        let def = def_of(node)?;
        let p = def
            .outputs
            .iter()
            .find(|p| p.name == pin)
            .ok_or_else(|| Recusa::PinoAusente {
                etiqueta: node.etiqueta(),
                pin: pin.to_string(),
                saida: true,
            })?;
        self.resolve(node, p)
    }

    /// Tipo de uma entrada. Nós de variável seguem o tipo da variável que nomeiam; reroutes seguem o fio.
    pub(crate) fn input_type(&self, node: &GraphNode, pin: &str) -> Result<PinType, Recusa> {
        let def = def_of(node)?;
        let p = def
            .inputs
            .iter()
            .find(|p| p.name == pin)
            .ok_or_else(|| Recusa::PinoAusente {
                etiqueta: node.etiqueta(),
                pin: pin.to_string(),
                saida: false,
            })?;
        self.resolve(node, p)
    }

    fn resolve(&self, node: &GraphNode, pin: &PinDef) -> Result<PinType, Recusa> {
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
    fn tipo_do_reroute(&self, node: &GraphNode) -> Result<PinType, Recusa> {
        let mut vistos = HashSet::new();
        let mut atual = node.id;
        loop {
            if !vistos.insert(atual) {
                return Err(Recusa::CicloReroute { node: atual });
            }
            let w = self
                .wires
                .iter()
                .find(|w| w.to.node == atual && w.to.pin == "in")
                .ok_or(Recusa::RerouteSemEntrada { node: atual })?;
            let origem = self
                .node(w.from.node)
                .ok_or(Recusa::NoAusente { node: w.from.node })?;
            if origem.kind == REROUTE {
                atual = origem.id;
                continue;
            }
            return self.output_type(origem, &w.from.pin);
        }
    }

    /// Variável nomeada pelo nó. O nome é um valor embutido: não pode vir de fio, porque a variável é fixa na
    /// compilação.
    pub(crate) fn variable_of(&self, node: &GraphNode) -> Result<&Variable, Recusa> {
        let name = match node.values.get("name") {
            Some(Value::String(s)) => s.as_str(),
            _ => {
                return Err(Recusa::VariavelSemNome {
                    etiqueta: node.etiqueta(),
                });
            }
        };
        self.variable(name)
            .ok_or_else(|| Recusa::VariavelInexistente {
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
                Err(motivo) => {
                    erros.push(erro_de(node.id, &motivo));
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
                    Err(motivo) => erros.push(erro_de(node.id, &motivo)),
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
            if (node.kind == "GetVariable" || node.kind == "SetVariable")
                && let Err(motivo) = self.variable_of(node)
            {
                erros.push(erro_de(node.id, &motivo));
            }
        }

        let mut ligadas = HashSet::new();
        for w in &self.wires {
            if let Err(motivo) = self.endpoint_type(&w.from, &w.to) {
                erros.push(erro_de(w.to.node, &motivo));
                continue;
            }
            if !ligadas.insert(&w.to) {
                erros.push(erro_de(
                    w.to.node,
                    &Recusa::EntradaDupla {
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
            let motivo = Recusa::CicloNoGrafo {
                fluxo: true,
                node: id,
            };
            erros.push(erro_de(id, &motivo));
        }
        if let Some(id) = self.ciclo(false) {
            let motivo = Recusa::CicloNoGrafo {
                fluxo: false,
                node: id,
            };
            erros.push(erro_de(id, &motivo));
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

    /// Toda recusa traz motivo e sugestão não vazios: é o que o editor mostra.
    fn motivo_completo(motivo: &Recusa) {
        assert!(!motivo.message().trim().is_empty(), "{motivo:?}");
        assert!(!motivo.suggestion().trim().is_empty(), "{motivo:?}");
    }

    #[test]
    fn recusa_no_ausente_tem_motivo_e_sugestao() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let motivo = g.can_connect((ev, "exec"), (99, "exec")).unwrap_err();
        assert_eq!(motivo, Recusa::NoAusente { node: 99 });
        motivo_completo(&motivo);
    }

    #[test]
    fn recusa_pino_ausente_tem_motivo_e_sugestao() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let soma = adiciona(&mut g, "Add");
        let motivo = g
            .can_connect((ev, "exec"), (soma, "entrada_falsa"))
            .unwrap_err();
        assert!(
            matches!(motivo, Recusa::PinoAusente { saida: false, .. }),
            "{motivo:?}"
        );
        motivo_completo(&motivo);
    }

    #[test]
    fn recusa_tipo_desconhecido_tem_motivo_e_sugestao() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        // Um nó de tipo que a biblioteca não tem: só chega ao grafo por um arquivo editado à mão.
        g.nodes.push(GraphNode {
            id: 50,
            kind: "NaoExiste".into(),
            position: [0.0, 0.0],
            values: BTreeMap::new(),
        });
        let motivo = g.can_connect((ev, "exec"), (50, "exec")).unwrap_err();
        assert_eq!(
            motivo,
            Recusa::TipoDesconhecido {
                kind: "NaoExiste".into()
            }
        );
        motivo_completo(&motivo);
    }

    #[test]
    fn recusa_tipos_incompativeis_tem_motivo_e_sugestao() {
        let mut g = Graph::default();
        let cmp = adiciona(&mut g, "Greater");
        let soma = adiciona(&mut g, "Add");
        let motivo = g.can_connect((cmp, "result"), (soma, "a")).unwrap_err();
        assert!(
            matches!(motivo, Recusa::TiposIncompativeis { .. }),
            "{motivo:?}"
        );
        motivo_completo(&motivo);
    }

    #[test]
    fn recusa_entrada_ocupada_tem_motivo_e_sugestao() {
        let mut g = Graph::default();
        let a = adiciona(&mut g, "Add");
        let b = adiciona(&mut g, "Add");
        let m = adiciona(&mut g, "Multiply");
        g.connect((a, "result"), (m, "a")).unwrap();
        let motivo = g.can_connect((b, "result"), (m, "a")).unwrap_err();
        assert_eq!(
            motivo,
            Recusa::EntradaOcupada {
                node: m,
                pin: "a".into()
            }
        );
        motivo_completo(&motivo);
    }

    #[test]
    fn recusa_ciclo_ao_ligar_tem_motivo_e_sugestao() {
        let mut g = Graph::default();
        let a = adiciona(&mut g, "Sequence");
        let b = adiciona(&mut g, "Sequence");
        g.connect((a, "then_0"), (b, "exec")).unwrap();
        let fluxo = g.can_connect((b, "then_0"), (a, "exec")).unwrap_err();
        assert!(
            matches!(fluxo, Recusa::CicloAoLigar { fluxo: true, .. }),
            "{fluxo:?}"
        );
        motivo_completo(&fluxo);
        let soma = adiciona(&mut g, "Add");
        let dados = g.can_connect((soma, "result"), (soma, "a")).unwrap_err();
        assert!(
            matches!(dados, Recusa::CicloAoLigar { fluxo: false, .. }),
            "{dados:?}"
        );
        motivo_completo(&dados);
    }

    #[test]
    fn recusa_ciclo_no_grafo_tem_sugestao() {
        let mut g = Graph::default();
        let a = adiciona(&mut g, "Add");
        let b = adiciona(&mut g, "Add");
        // Ciclo de dados montado à mão: a validação aponta o nó, com a sugestão.
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
            .expect("a validação acha o ciclo");
        assert!(erro.node.is_some());
        assert_eq!(
            erro.suggestion,
            Some("Guarde com SetVariable e leia com GetVariable")
        );
    }

    #[test]
    fn recusa_reroute_sem_entrada_tem_motivo_e_sugestao() {
        let mut g = Graph::default();
        let r = g.add_reroute([0.0, 0.0]);
        let soma = adiciona(&mut g, "Add");
        let motivo = g.can_connect((r, "out"), (soma, "a")).unwrap_err();
        assert_eq!(motivo, Recusa::RerouteSemEntrada { node: r });
        motivo_completo(&motivo);
    }

    #[test]
    fn recusa_ciclo_de_reroute_tem_motivo_e_sugestao() {
        let mut g = Graph::default();
        let r1 = g.add_reroute([0.0, 0.0]);
        let r2 = g.add_reroute([0.0, 0.0]);
        // Dois reroutes que se alimentam: montado à mão, porque connect já recusaria.
        g.wires.push(Wire {
            from: pin_ref((r1, "out")),
            to: pin_ref((r2, "in")),
        });
        g.wires.push(Wire {
            from: pin_ref((r2, "out")),
            to: pin_ref((r1, "in")),
        });
        let soma = adiciona(&mut g, "Add");
        let motivo = g.can_connect((r1, "out"), (soma, "a")).unwrap_err();
        assert!(matches!(motivo, Recusa::CicloReroute { .. }), "{motivo:?}");
        motivo_completo(&motivo);
    }

    #[test]
    fn recusa_variavel_sem_nome_tem_motivo_e_sugestao() {
        let mut g = Graph::default();
        let le = adiciona(&mut g, "GetVariable");
        let soma = adiciona(&mut g, "Add");
        let motivo = g.can_connect((le, "value"), (soma, "a")).unwrap_err();
        assert!(
            matches!(motivo, Recusa::VariavelSemNome { .. }),
            "{motivo:?}"
        );
        motivo_completo(&motivo);
    }

    #[test]
    fn recusa_variavel_inexistente_tem_motivo_e_sugestao() {
        let mut g = Graph::default();
        let le = adiciona(&mut g, "GetVariable");
        g.set_value(le, "name", Value::String("nada".into()))
            .unwrap();
        let soma = adiciona(&mut g, "Add");
        let motivo = g.can_connect((le, "value"), (soma, "a")).unwrap_err();
        assert_eq!(
            motivo,
            Recusa::VariavelInexistente {
                name: "nada".into()
            }
        );
        motivo_completo(&motivo);
    }

    #[test]
    fn recusa_entrada_dupla_tem_sugestao() {
        let mut g = Graph::default();
        let a = adiciona(&mut g, "Add");
        let b = adiciona(&mut g, "Add");
        let m = adiciona(&mut g, "Multiply");
        // Duas saídas na mesma entrada: só existe em grafo editado à mão.
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
    fn recusa_nao_muda_o_grafo() {
        let mut g = Graph::default();
        let cmp = adiciona(&mut g, "Greater");
        let soma = adiciona(&mut g, "Add");
        let a = adiciona(&mut g, "Add");
        let m = adiciona(&mut g, "Multiply");
        g.connect((a, "result"), (m, "a")).unwrap();
        let antes = g.clone();
        // Três recusas: tipos, entrada ocupada e nó que não existe.
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
        assert!(g.remove_value(soma, "a"));
        assert!(!g.remove_value(soma, "a"));
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
}
