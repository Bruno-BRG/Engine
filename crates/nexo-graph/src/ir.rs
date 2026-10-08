//! Compilador do grafo para a IR (VIS-023). Cada nó de evento vira um programa linear de ops. Os pinos de
//! dados viram registradores, preenchidos logo antes do nó que os consome. O Branch vira um salto
//! condicional. Os erros de ligação e de entrada obrigatória são reunidos e devolvidos ao editor, cada um
//! com o id do nó onde foi achado.

use crate::graph::{Graph, GraphNode, def_of};
use crate::library;
use crate::types::{Value, Variable};
use crate::vm::Context;
use std::collections::HashSet;
use std::fmt;

/// Índice de um registrador do programa.
pub type Reg = usize;

/// Instrução da IR. Um registrador é escrito antes de ser lido, e cada valor tem o seu.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// `dst` recebe um valor fixo (embutido ou padrão do pino).
    Const { dst: Reg, value: Value },
    /// `dst` recebe o id do nó dono do script (saída "self").
    Owner { dst: Reg },
    /// `dst` recebe o delta do quadro (saída "delta").
    Delta { dst: Reg },
    /// `dst` recebe o valor da variável `var` da instância.
    GetVar { dst: Reg, var: usize },
    /// A variável `var` da instância recebe o valor de `src`.
    SetVar { var: usize, src: Reg },
    /// `dst` = `a` + `b`, em decimal.
    Add { dst: Reg, a: Reg, b: Reg },
    /// `dst` = `a` * `b`, em decimal.
    Mul { dst: Reg, a: Reg, b: Reg },
    /// `dst` = `a` > `b`.
    Greater { dst: Reg, a: Reg, b: Reg },
    /// `dst` = a ação `action` está pressionada.
    ActionPressed { dst: Reg, action: Reg },
    /// `dst` = propriedade `name` do nó `node`.
    GetProp { dst: Reg, node: Reg, name: Reg },
    /// Grava `value` na propriedade `name` do nó `node`.
    SetProp { node: Reg, name: Reg, value: Reg },
    /// Escreve o texto `msg` no console.
    Print { msg: Reg },
    /// Remove o nó `node` da cena.
    Destroy { node: Reg },
    /// Salta para a op de índice `target`.
    Jump { target: usize },
    /// Salta para `target` quando `cond` é falso.
    JumpIfFalse { cond: Reg, target: usize },
}

/// Programa de um nó de evento: as ops que rodam quando o evento dispara.
#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    /// Nome do evento que dispara o programa: "OnReady", "OnProcess"...
    pub event: String,
    /// Id do nó de evento de onde o programa veio.
    pub node: u64,
    pub ops: Vec<Op>,
    /// Nó de origem de cada op, no mesmo índice de `ops`. Serve para os erros e, depois, para a depuração.
    pub sources: Vec<u64>,
    /// Quantidade de registradores que o programa usa.
    pub regs: usize,
}

/// Resultado da compilação: as variáveis do script e um programa por nó de evento.
#[derive(Clone, Debug, PartialEq)]
pub struct Compiled {
    /// Variáveis do script, na ordem em que as ops `GetVar` e `SetVar` as indexam.
    pub variables: Vec<Variable>,
    pub programs: Vec<Program>,
}

impl Compiled {
    /// Contexto de uma instância do script, com as variáveis no valor padrão.
    pub fn new_context(&self, owner: u64) -> Context {
        Context {
            owner,
            delta: 0.0,
            vars: self.variables.iter().map(|v| v.default.clone()).collect(),
        }
    }

    /// Programas de um evento, na ordem do grafo.
    pub fn programs_for<'a>(&'a self, event: &'a str) -> impl Iterator<Item = &'a Program> + 'a {
        self.programs.iter().filter(move |p| p.event == event)
    }
}

/// Erro de validação. `node` é o nó onde o problema foi achado, quando há um.
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

fn err(node: u64, message: impl Into<String>) -> CompileError {
    CompileError {
        node: Some(node),
        message: message.into(),
    }
}

/// Compila o grafo: um programa por nó de evento. Devolve todos os erros de validação que achar.
pub fn compile(graph: &Graph) -> Result<Compiled, Vec<CompileError>> {
    let mut errors = Vec::new();
    check_graph(graph, &mut errors);
    let mut programs = Vec::new();
    for event in graph.nodes.iter().filter(|n| is_event(n)) {
        match compile_event(graph, event) {
            Ok(p) => programs.push(p),
            Err(e) => errors.push(e),
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Compiled {
        variables: graph.variables.clone(),
        programs,
    })
}

fn is_event(node: &GraphNode) -> bool {
    library::find(&node.kind).is_some_and(|d| d.event)
}

/// Checagens que valem para o grafo inteiro, mesmo nos nós que nenhum evento alcança.
fn check_graph(graph: &Graph, errors: &mut Vec<CompileError>) {
    let mut ids = HashSet::new();
    for node in &graph.nodes {
        if !ids.insert(node.id) {
            errors.push(err(node.id, format!("id de nó repetido: {}", node.id)));
        }
        if let Err(msg) = def_of(node) {
            errors.push(err(node.id, msg));
        }
    }
    // Fios montados à mão (por exemplo, num arquivo editado) não passaram pelo `connect`.
    let mut ligadas = HashSet::new();
    for w in &graph.wires {
        if let Err(msg) = graph.endpoint_type(&w.from, &w.to) {
            errors.push(err(w.to.node, msg));
            continue;
        }
        if !ligadas.insert(&w.to) {
            errors.push(err(
                w.to.node,
                format!(
                    "a entrada '{}' do nó {} está ligada mais de uma vez",
                    w.to.pin, w.to.node
                ),
            ));
        }
    }
}

fn compile_event(graph: &Graph, event: &GraphNode) -> Result<Program, CompileError> {
    let mut e = Emitter {
        graph,
        ops: Vec::new(),
        sources: Vec::new(),
        regs: 0,
        flow_stack: Vec::new(),
        value_stack: Vec::new(),
    };
    e.flow(event.id, "exec")?;
    Ok(Program {
        event: event.kind.clone(),
        node: event.id,
        ops: e.ops,
        sources: e.sources,
        regs: e.regs,
    })
}

/// Estado da compilação de um evento.
struct Emitter<'g> {
    graph: &'g Graph,
    ops: Vec<Op>,
    sources: Vec<u64>,
    regs: usize,
    /// Nós de fluxo em compilação. Um nó que aparece de novo aqui forma um laço de fluxo.
    flow_stack: Vec<u64>,
    /// Nós de dados em avaliação. Um nó que aparece de novo aqui forma um ciclo de dados.
    value_stack: Vec<u64>,
}

impl<'g> Emitter<'g> {
    /// Acrescenta uma op e devolve o índice dela.
    fn emit(&mut self, op: Op, source: u64) -> usize {
        self.ops.push(op);
        self.sources.push(source);
        self.ops.len() - 1
    }

    fn reg(&mut self) -> Reg {
        self.regs += 1;
        self.regs - 1
    }

    /// Ajusta o alvo de um salto já emitido.
    fn patch(&mut self, at: usize, target: usize) {
        if let Op::Jump { target: t } | Op::JumpIfFalse { target: t, .. } = &mut self.ops[at] {
            *t = target;
        }
    }

    /// Emite o que vem depois do pino de fluxo `pin` do nó `from`. Vários fios saindo do mesmo pino rodam
    /// na ordem em que foram criados.
    fn flow(&mut self, from: u64, pin: &str) -> Result<(), CompileError> {
        let graph = self.graph;
        for w in graph.wires.iter().filter(|w| w.from.node == from && w.from.pin == pin) {
            self.exec(w.to.node)?;
        }
        Ok(())
    }

    /// Emite a op de um nó de fluxo e segue pelas saídas dele.
    fn exec(&mut self, id: u64) -> Result<(), CompileError> {
        if self.flow_stack.contains(&id) {
            return Err(err(id, format!("ciclo de fluxo no nó {id}")));
        }
        let graph = self.graph;
        let node = graph
            .node(id)
            .ok_or_else(|| err(id, format!("nó {id} não existe no grafo")))?;
        self.flow_stack.push(id);
        let result = self.exec_node(node);
        self.flow_stack.pop();
        result
    }

    fn exec_node(&mut self, node: &'g GraphNode) -> Result<(), CompileError> {
        let id = node.id;
        match node.kind.as_str() {
            "Print" => {
                let msg = self.input(node, "msg")?;
                self.emit(Op::Print { msg }, id);
                self.flow(id, "exec")
            }
            "Destroy" => {
                let target = self.input(node, "node")?;
                self.emit(Op::Destroy { node: target }, id);
                self.flow(id, "exec")
            }
            "SetProperty" => {
                let target = self.input(node, "node")?;
                let name = self.input(node, "name")?;
                let value = self.input(node, "value")?;
                self.emit(
                    Op::SetProp {
                        node: target,
                        name,
                        value,
                    },
                    id,
                );
                self.flow(id, "exec")
            }
            "SetVariable" => {
                let var = self.var_index(node)?;
                let src = self.input(node, "value")?;
                self.emit(Op::SetVar { var, src }, id);
                self.flow(id, "exec")
            }
            "Branch" => {
                // cond; JumpIfFalse -> falso; <ramo verdadeiro>; Jump -> fim; <ramo falso>; fim:
                let cond = self.input(node, "condition")?;
                let to_false = self.emit(Op::JumpIfFalse { cond, target: 0 }, id);
                self.flow(id, "true")?;
                let to_end = self.emit(Op::Jump { target: 0 }, id);
                let false_start = self.ops.len();
                self.patch(to_false, false_start);
                self.flow(id, "false")?;
                let end = self.ops.len();
                self.patch(to_end, end);
                Ok(())
            }
            "Sequence" => {
                self.flow(id, "then_0")?;
                self.flow(id, "then_1")
            }
            other => Err(err(id, format!("{other} não é um nó de fluxo"))),
        }
    }

    /// Registrador com o valor da entrada `pin`: pelo fio, se houver; senão, pelo valor embutido ou pelo padrão.
    fn input(&mut self, node: &'g GraphNode, pin: &str) -> Result<Reg, CompileError> {
        let graph = self.graph;
        if let Some(w) = graph.wires.iter().find(|w| w.to.node == node.id && w.to.pin == pin) {
            return self.output(w.from.node, &w.from.pin);
        }
        let expected = graph.input_type(node, pin).map_err(|m| err(node.id, m))?;
        let def = def_of(node).map_err(|m| err(node.id, m))?;
        let default = def
            .inputs
            .iter()
            .find(|p| p.name == pin)
            .and_then(|p| p.default.clone());
        let value = node.values.get(pin).cloned().or(default).ok_or_else(|| {
            err(
                node.id,
                format!("entrada obrigatória '{pin}' de {} não está conectada", node.etiqueta()),
            )
        })?;
        if value.ty() != expected {
            return Err(err(
                node.id,
                format!(
                    "valor de '{pin}' em {} é {} mas o pino é {}",
                    node.etiqueta(),
                    value.ty().nome(),
                    expected.nome()
                ),
            ));
        }
        let dst = self.reg();
        self.emit(Op::Const { dst, value }, node.id);
        Ok(dst)
    }

    /// Índice da variável nomeada pelo nó. O nome tem de ser embutido, porque a variável é fixa na compilação.
    fn var_index(&self, node: &GraphNode) -> Result<usize, CompileError> {
        let name = match node.values.get("name") {
            Some(Value::String(s)) => s,
            _ => {
                return Err(err(
                    node.id,
                    format!(
                        "{} precisa do nome da variável embutido (não pode vir de fio)",
                        node.etiqueta()
                    ),
                ));
            }
        };
        self.graph
            .variables
            .iter()
            .position(|v| &v.name == name)
            .ok_or_else(|| err(node.id, format!("variável '{name}' não existe")))
    }

    /// Registrador com o valor da saída `pin` do nó `id`. Cada uso avalia o nó de novo. Isso é seguro: nós de
    /// dados não têm efeito colateral, quem age é o fluxo.
    fn output(&mut self, id: u64, pin: &str) -> Result<Reg, CompileError> {
        if self.value_stack.contains(&id) {
            return Err(err(id, format!("ciclo de dados no nó {id}")));
        }
        let graph = self.graph;
        let node = graph
            .node(id)
            .ok_or_else(|| err(id, format!("nó {id} não existe no grafo")))?;
        self.value_stack.push(id);
        let result = self.output_node(node, pin);
        self.value_stack.pop();
        result
    }

    fn output_node(&mut self, node: &'g GraphNode, pin: &str) -> Result<Reg, CompileError> {
        let id = node.id;
        match (node.kind.as_str(), pin) {
            ("OnReady" | "OnProcess", "self") => {
                let dst = self.reg();
                self.emit(Op::Owner { dst }, id);
                Ok(dst)
            }
            ("OnProcess", "delta") => {
                let dst = self.reg();
                self.emit(Op::Delta { dst }, id);
                Ok(dst)
            }
            ("Add" | "Multiply" | "Greater", "result") => {
                let a = self.input(node, "a")?;
                let b = self.input(node, "b")?;
                let dst = self.reg();
                let op = match node.kind.as_str() {
                    "Add" => Op::Add { dst, a, b },
                    "Multiply" => Op::Mul { dst, a, b },
                    _ => Op::Greater { dst, a, b },
                };
                self.emit(op, id);
                Ok(dst)
            }
            ("ActionPressed", "pressed") => {
                let action = self.input(node, "action")?;
                let dst = self.reg();
                self.emit(Op::ActionPressed { dst, action }, id);
                Ok(dst)
            }
            ("GetProperty", "value") => {
                let target = self.input(node, "node")?;
                let name = self.input(node, "name")?;
                let dst = self.reg();
                self.emit(
                    Op::GetProp {
                        dst,
                        node: target,
                        name,
                    },
                    id,
                );
                Ok(dst)
            }
            ("GetVariable", "value") => {
                let var = self.var_index(node)?;
                let dst = self.reg();
                self.emit(Op::GetVar { dst, var }, id);
                Ok(dst)
            }
            _ => Err(err(id, format!("{} não tem a saída de valor '{pin}'", node.etiqueta()))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{PinRef, Wire};

    fn adiciona(g: &mut Graph, kind: &str) -> u64 {
        g.add_node(kind, [0.0, 0.0]).unwrap()
    }

    #[test]
    fn entrada_obrigatoria_sem_ligacao_gera_erro() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnProcess");
        // SetProperty sem nó nem propriedade: as duas entradas são obrigatórias.
        let sp = adiciona(&mut g, "SetProperty");
        g.connect((ev, "exec"), (sp, "exec")).unwrap();
        let erros = compile(&g).unwrap_err();
        assert!(
            erros
                .iter()
                .any(|e| e.node == Some(sp) && e.message.contains("obrigatória")),
            "{erros:?}"
        );
    }

    #[test]
    fn um_programa_por_no_de_evento() {
        let mut g = Graph::default();
        let a = adiciona(&mut g, "OnReady");
        let b = adiciona(&mut g, "OnProcess");
        let p = adiciona(&mut g, "Print");
        g.connect((a, "exec"), (p, "exec")).unwrap();
        let c = compile(&g).unwrap();
        assert_eq!(c.programs.len(), 2);
        assert_eq!(c.programs[0].event, "OnReady");
        assert_eq!(c.programs[0].node, a);
        assert_eq!(c.programs[1].event, "OnProcess");
        assert_eq!(c.programs[1].node, b);
        // O OnProcess não tem nada ligado no fluxo: programa vazio.
        assert!(c.programs[1].ops.is_empty());
    }

    #[test]
    fn ciclo_de_fluxo_montado_a_mao_e_pego_na_compilacao() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let a = adiciona(&mut g, "Sequence");
        let b = adiciona(&mut g, "Sequence");
        g.connect((ev, "exec"), (a, "exec")).unwrap();
        g.connect((a, "then_0"), (b, "exec")).unwrap();
        // Fio de volta colocado direto na lista, como num arquivo editado à mão.
        g.wires.push(Wire {
            from: PinRef {
                node: b,
                pin: "then_0".into(),
            },
            to: PinRef {
                node: a,
                pin: "exec".into(),
            },
        });
        let erros = compile(&g).unwrap_err();
        assert!(erros.iter().any(|e| e.message.contains("ciclo de fluxo")), "{erros:?}");
    }

    #[test]
    fn branch_sem_caminhos_salta_para_o_fim() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let br = adiciona(&mut g, "Branch");
        g.connect((ev, "exec"), (br, "exec")).unwrap();
        let c = compile(&g).unwrap();
        // Const(condição padrão = falso); JumpIfFalse para o fim; Jump para o fim.
        assert_eq!(
            c.programs[0].ops,
            vec![
                Op::Const {
                    dst: 0,
                    value: Value::Bool(false)
                },
                Op::JumpIfFalse { cond: 0, target: 3 },
                Op::Jump { target: 3 },
            ]
        );
        assert_eq!(c.programs[0].regs, 1);
    }
}
