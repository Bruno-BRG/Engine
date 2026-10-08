//! Editor do grafo de lógica no WebAssembly (VIS-010 a VIS-015). O grafo de um script vive aqui: o TypeScript lê o
//! estado com `snapshot`, envia operações e pede desfazer e refazer. As regras estão em `nexo-graph` (testadas lá);
//! este arquivo só traduz para o JavaScript. Os ids de nós e de caixas são números (`f64`): cabem sem perda, e um
//! `u64` viraria BigInt no wasm-bindgen.

use nexo_graph::{Fragment, Graph, PinType, Value, Variable, generate_csharp, library};
use serde::Serialize;
use serde_json::json;
use wasm_bindgen::prelude::*;

/// Passos de desfazer guardados. Cada passo é uma cópia do grafo, então o limite evita crescer sem fim.
const LIMITE_DESFAZER: usize = 200;

/// Grafo de um script, com o histórico de desfazer e refazer.
#[wasm_bindgen]
pub struct GraphEditor {
    graph: Graph,
    passado: Vec<Graph>,
    futuro: Vec<Graph>,
    /// Grupo aberto (arrastar um nó, digitar num campo): as edições do grupo viram um passo só.
    agrupando: bool,
}

fn js_err(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

fn to_json(v: &impl Serialize) -> Result<String, JsError> {
    serde_json::to_string(v).map_err(js_err)
}

/// Variables as JSON. A node-reference default also goes as `default_node`, hex text in the same format as the
/// exported values (ADR 0012): a JSON number above 2^53 loses its low digits when JavaScript reads it.
fn variables_json(variables: &[Variable]) -> Result<serde_json::Value, serde_json::Error> {
    let mut items = Vec::with_capacity(variables.len());
    for variable in variables {
        let mut item = serde_json::to_value(variable)?;
        if let (Value::Node(id), Some(object)) = (&variable.default, item.as_object_mut()) {
            let text = if *id == 0 { String::new() } else { format!("{id:016x}") };
            object.insert("default_node".into(), serde_json::Value::String(text));
        }
        items.push(item);
    }
    Ok(serde_json::Value::Array(items))
}

/// C# of a graph: the class `csharp_class_name(script)` with one method per event. A compile error comes back as
/// the messages of the validation, one per line, and no code is generated (VIS-024).
fn csharp_text(graph: &Graph, script: &str) -> Result<String, String> {
    let compiled = nexo_graph::compile(graph).map_err(|errors| {
        errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    Ok(generate_csharp(
        &compiled,
        &nexo_graph::csharp_class_name(script),
    ))
}

impl GraphEditor {
    /// Aplica uma edição. Antes, guarda a cópia do grafo para desfazer, a não ser que um grupo esteja aberto (aí
    /// o passo já foi guardado ao abrir o grupo). Se a edição falha, o grafo volta ao estado anterior.
    fn edita<T>(&mut self, f: impl FnOnce(&mut Graph) -> Result<T, String>) -> Result<T, JsError> {
        let antes = self.graph.clone();
        match f(&mut self.graph) {
            Ok(v) => {
                if self.graph != antes {
                    if !self.agrupando {
                        self.empilha(antes);
                    }
                    self.futuro.clear();
                }
                Ok(v)
            }
            Err(msg) => {
                self.graph = antes;
                Err(JsError::new(&msg))
            }
        }
    }

    fn empilha(&mut self, grafo: Graph) {
        self.passado.push(grafo);
        if self.passado.len() > LIMITE_DESFAZER {
            self.passado.remove(0);
        }
    }
}

fn ids(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| *x as u64).collect()
}

#[wasm_bindgen]
impl GraphEditor {
    /// Grafo vazio, sem histórico.
    #[wasm_bindgen(constructor)]
    pub fn new() -> GraphEditor {
        GraphEditor {
            graph: Graph::default(),
            passado: Vec::new(),
            futuro: Vec::new(),
            agrupando: false,
        }
    }

    /// Carrega o texto RON de um script (`scripts/<nome>.logic.ron`). Zera o histórico.
    pub fn load_ron(&mut self, text: &str) -> Result<(), JsError> {
        self.graph = Graph::from_ron(text).map_err(js_err)?;
        self.passado.clear();
        self.futuro.clear();
        self.agrupando = false;
        Ok(())
    }

    /// Texto RON do grafo no formato atual. É o que `Graph::from_ron` lê de volta.
    pub fn to_ron(&self) -> Result<String, JsError> {
        self.graph.to_ron().map_err(js_err)
    }

    /// Biblioteca de nós em JSON: kind, categoria, rótulo, descrição, se é evento e os pinos (nome, rótulo, tipo e
    /// valor padrão). Não depende do grafo: é a mesma para todos.
    pub fn node_library() -> Result<String, JsError> {
        to_json(&library::library())
    }

    /// Estado do grafo em JSON: `nodes`, `wires`, `comments`, `variables` e `pins` (tipo resolvido de cada pino).
    pub fn snapshot(&self) -> Result<String, JsError> {
        let variables = variables_json(&self.graph.variables).map_err(js_err)?;
        to_json(&json!({
            "nodes": self.graph.nodes,
            "wires": self.graph.wires,
            "comments": self.graph.comments,
            "variables": variables,
            "pins": self.graph.pins(),
        }))
    }

    /// Validation errors as JSON: [{node, message, suggestion}]. `node` is null when the error is not about a node,
    /// and `suggestion` is null when there is no clear fix (VIS-013).
    pub fn validate(&self) -> Result<String, JsError> {
        let erros: Vec<_> = self
            .graph
            .validate()
            .into_iter()
            .map(|e| json!({ "node": e.node, "message": e.message, "suggestion": e.suggestion }))
            .collect();
        to_json(&erros)
    }

    /// C# code of the script, generated from the IR (VIS-024). If the graph has errors, the exception carries the
    /// validation messages, one per line. The class is named by `csharp_class_name(script)`.
    pub fn to_csharp(&self, script: &str) -> Result<String, JsError> {
        csharp_text(&self.graph, script).map_err(|msg| JsError::new(&msg))
    }

    /// C# class name of a script, as `to_csharp` writes it. The generated file is that name plus `.cs`.
    pub fn csharp_class_name(script: &str) -> String {
        nexo_graph::csharp_class_name(script)
    }

    // ---------- Nós, reroutes e fios ----------

    /// Adiciona um nó do tipo `kind` na posição dada. Devolve o id.
    pub fn add_node(&mut self, kind: &str, x: f64, y: f64) -> Result<f64, JsError> {
        self.edita(|g| g.add_node(kind, [x, y])).map(|id| id as f64)
    }

    /// Adiciona um reroute (VIS-014). Devolve o id.
    pub fn add_reroute(&mut self, x: f64, y: f64) -> Result<f64, JsError> {
        self.edita(|g| Ok(g.add_reroute([x, y]))).map(|id| id as f64)
    }

    pub fn remove_node(&mut self, id: f64) -> Result<(), JsError> {
        self.edita(|g| g.remove_node(id as u64))
    }

    /// Move um nó, um reroute ou uma caixa de comentário.
    pub fn set_position(&mut self, id: f64, x: f64, y: f64) -> Result<(), JsError> {
        self.edita(|g| g.set_position(id as u64, [x, y]))
    }

    /// Liga a saída à entrada. Em caso de recusa, o erro diz o motivo (tipos, entrada ocupada, ciclo).
    pub fn connect(&mut self, from_node: f64, from_pin: &str, to_node: f64, to_pin: &str) -> Result<(), JsError> {
        self.edita(|g| g.connect((from_node as u64, from_pin), (to_node as u64, to_pin)))
    }

    /// Whether the link would be accepted, without changing anything: `undefined` if so. Otherwise returns JSON
    /// {message, suggestion} with the refusal and how to fix it (VIS-013). The editor uses it to highlight the pins
    /// that accept the dragged wire and to show the refusal.
    pub fn check_connection(&self, from_node: f64, from_pin: &str, to_node: f64, to_pin: &str) -> Option<String> {
        let refusal = self
            .graph
            .can_connect((from_node as u64, from_pin), (to_node as u64, to_pin))
            .err()?;
        Some(json!({ "message": refusal.message(), "suggestion": refusal.suggestion() }).to_string())
    }

    pub fn disconnect(&mut self, from_node: f64, from_pin: &str, to_node: f64, to_pin: &str) -> Result<(), JsError> {
        self.edita(|g| g.disconnect((from_node as u64, from_pin), (to_node as u64, to_pin)))
    }

    /// Remove o fio que chega na entrada. Devolve se havia fio.
    pub fn disconnect_input(&mut self, to_node: f64, to_pin: &str) -> Result<bool, JsError> {
        self.edita(|g| Ok(g.disconnect_input((to_node as u64, to_pin))))
    }

    /// Writes the embedded value of an input. `value_json` is the `Value` as JSON, for example {"Float": 2.5}. Returns, as
    /// JSON, the wires the change dropped: changing the type or name of a property node can leave wires without a valid type.
    pub fn set_value(&mut self, node: f64, pin: &str, value_json: &str) -> Result<String, JsError> {
        let value: Value = serde_json::from_str(value_json).map_err(js_err)?;
        let removed = self.edita(|g| g.set_value(node as u64, pin, value))?;
        to_json(&removed)
    }

    /// Removes the embedded value: the input goes back to the library default. Returns, as JSON, the wires the change dropped.
    pub fn remove_value(&mut self, node: f64, pin: &str) -> Result<String, JsError> {
        let removed = self.edita(|g| Ok(g.remove_value(node as u64, pin)))?;
        to_json(&removed)
    }

    // ---------- Variáveis (VIS-022) ----------

    /// Declara ou troca uma variável. `ty_json` é o tipo ("Float", "Bool"...) e `default_json` o valor padrão
    /// no formato de `Value`.
    pub fn set_variable(&mut self, name: &str, ty_json: &str, default_json: &str, exported: bool) -> Result<(), JsError> {
        let ty: PinType = serde_json::from_str(ty_json).map_err(js_err)?;
        let default: Value = serde_json::from_str(default_json).map_err(js_err)?;
        self.edita(|g| g.set_variable(name, ty, default, exported))
    }

    pub fn remove_variable(&mut self, name: &str) -> Result<(), JsError> {
        self.edita(|g| g.remove_variable(name))
    }

    // ---------- Comentários (VIS-014) ----------

    pub fn add_comment(&mut self, text: &str, x: f64, y: f64, w: f64, h: f64) -> Result<f64, JsError> {
        self.edita(|g| Ok(g.add_comment(text, [x, y], [w, h]))).map(|id| id as f64)
    }

    pub fn set_comment(&mut self, id: f64, text: &str, x: f64, y: f64, w: f64, h: f64) -> Result<(), JsError> {
        self.edita(|g| g.set_comment(id as u64, text, [x, y], [w, h]))
    }

    pub fn remove_comment(&mut self, id: f64) -> Result<(), JsError> {
        self.edita(|g| g.remove_comment(id as u64))
    }

    // ---------- Copiar, colar e colapsar (VIS-012, VIS-015) ----------

    /// Trecho da seleção em JSON, para a área de transferência. Não muda o grafo.
    pub fn copy_fragment(&self, nodes: &[f64], comments: &[f64]) -> Result<String, JsError> {
        to_json(&self.graph.copy_fragment(&ids(nodes), &ids(comments)))
    }

    /// Cola um trecho (JSON de `copy_fragment`) deslocado por (dx, dy). Devolve os ids novos, em JSON:
    /// {nodes: [...], comments: [...]}.
    pub fn paste_fragment(&mut self, fragment_json: &str, dx: f64, dy: f64) -> Result<String, JsError> {
        let f: Fragment = serde_json::from_str(fragment_json).map_err(js_err)?;
        let novos = self.edita(|g| g.paste_fragment(&f, dx, dy))?;
        to_json(&novos)
    }

    /// Colapsa a seleção num nó Função com o nome dado. Devolve o id do Chamar função que a substitui.
    pub fn collapse_to_function(&mut self, nodes: &[f64], name: &str) -> Result<f64, JsError> {
        let sel = ids(nodes);
        self.edita(|g| g.collapse_to_function(&sel, name)).map(|id| id as f64)
    }

    // ---------- Desfazer e refazer (VIS-012) ----------

    /// Abre um grupo: as edições até `end_group` viram um passo de desfazer só (arrastar um nó, por exemplo).
    pub fn begin_group(&mut self) {
        if !self.agrupando {
            self.empilha(self.graph.clone());
            self.futuro.clear();
            self.agrupando = true;
        }
    }

    pub fn end_group(&mut self) {
        self.agrupando = false;
    }

    /// Volta um passo. Devolve false quando não há o que desfazer.
    pub fn undo(&mut self) -> bool {
        self.agrupando = false;
        let Some(anterior) = self.passado.pop() else {
            return false;
        };
        let atual = std::mem::replace(&mut self.graph, anterior);
        self.futuro.push(atual);
        true
    }

    /// Refaz um passo. Devolve false quando não há o que refazer.
    pub fn redo(&mut self) -> bool {
        self.agrupando = false;
        let Some(seguinte) = self.futuro.pop() else {
            return false;
        };
        let atual = std::mem::replace(&mut self.graph, seguinte);
        self.passado.push(atual);
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.passado.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.futuro.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexo_core::scene::Uid;

    #[test]
    fn node_default_is_also_sent_as_hex_text() {
        let variables = vec![
            Variable { name: "alvo".into(), ty: PinType::Node, default: Value::Node((1 << 53) + 1), exported: true },
            Variable { name: "vazio".into(), ty: PinType::Node, default: Value::Node(0), exported: false },
            Variable { name: "vida".into(), ty: PinType::Float, default: Value::Float(1.5), exported: true },
        ];
        let items = variables_json(&variables).unwrap();
        assert_eq!(items[0]["default_node"], "0020000000000001");
        assert_eq!(items[1]["default_node"], "");
        assert!(items[2].get("default_node").is_none());
        // The text reads back to the same id. A JSON number above 2^53 would not survive JavaScript.
        let text = items[0]["default_node"].as_str().unwrap();
        assert_eq!(text.parse::<Uid>().unwrap(), Uid((1 << 53) + 1));
    }

    #[test]
    fn csharp_of_script_uses_its_class_name_and_reports_compile_errors() {
        let mut g = Graph::default();
        g.add_node("OnReady", [0.0, 0.0]).unwrap();
        let code = csharp_text(&g, "Script 1").unwrap();
        assert!(
            code.contains("public partial class Script_1 : NexoScript"),
            "{code}"
        );
        assert!(code.contains("public void OnReady_"), "{code}");

        // Variable without a name: validation refuses the graph, and the message reaches the panel without code.
        g.variables.push(Variable {
            name: " ".into(),
            ty: PinType::Float,
            default: Value::Float(0.0),
            exported: false,
        });
        let error = csharp_text(&g, "Script 1").unwrap_err();
        assert!(error.contains("nome de variável vazio"), "{error}");
    }
}
