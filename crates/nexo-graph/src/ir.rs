//! Compilador do grafo para a IR (VIS-023, VIS-025). Cada nó de evento vira um programa linear de ops. Os pinos
//! de dados viram registradores, preenchidos logo antes do nó que os consome. Branch, While e ForEach viram
//! saltos. Nós latentes (Wait, Tween, Move To, Await Signal, Timer) viram ops que a VM suspende e retoma. Os
//! erros são reunidos pela validação do grafo e devolvidos ao editor, cada um com o id do nó onde foi achado.

use crate::graph::{Graph, GraphNode, REROUTE, def_of};
use crate::library;
use crate::types::{LogLevel, Value, Variable};
use crate::vm::Context;
use std::collections::BTreeMap;

pub use crate::graph::CompileError;

/// Índice de um registrador do programa.
pub type Reg = usize;

/// Comando de áudio na IR (AUD-004). Os valores vêm de registradores, como os de qualquer op. Os campos seguem o
/// contrato do núcleo: o nó é um registrador de nó, o stream é o texto com o UID do asset, e os volumes são em dB.
#[derive(Clone, Debug, PartialEq)]
pub enum AudioOp {
    Play {
        node: Reg,
        stream: Reg,
        volume_db: Reg,
        pitch: Reg,
        bus: Reg,
        looping: Reg,
        from_start: Reg,
    },
    Stop {
        node: Reg,
    },
    Fade {
        node: Reg,
        to_db: Reg,
        seconds: Reg,
    },
    SetBus {
        bus: Reg,
        volume_db: Reg,
    },
}

/// Instrução da IR. Um registrador é escrito antes de ser lido, e cada valor tem o seu.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// `dst` recebe um valor fixo (embutido ou padrão do pino).
    Const {
        dst: Reg,
        value: Value,
    },
    /// `dst` recebe o id do nó dono do script (saída "self").
    Owner {
        dst: Reg,
    },
    /// `dst` recebe o delta do quadro (saída "delta").
    Delta {
        dst: Reg,
    },
    /// `dst` recebe o valor da variável `var` da instância.
    GetVar {
        dst: Reg,
        var: usize,
    },
    /// A variável `var` da instância recebe o valor de `src`.
    SetVar {
        var: usize,
        src: Reg,
    },
    /// Decimais: soma, subtração, produto, quociente (erro se o divisor é zero).
    Add {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    Sub {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    Mul {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    Div {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    /// Comparações de decimais: `dst` = a > b, a < b, a == b.
    Greater {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    Less {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    Equal {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    /// Inteiros: `dst` = a + b, e `dst` = a < b (usado pelo ForEach).
    IntAdd {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    IntLess {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    /// Vetores 2D: monta [x, y]; soma; escala por um decimal; comprimento.
    MakeVec {
        dst: Reg,
        x: Reg,
        y: Reg,
    },
    AddVec {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    ScaleVec {
        dst: Reg,
        vector: Reg,
        scale: Reg,
    },
    VecLength {
        dst: Reg,
        vector: Reg,
    },
    /// Texto: `dst` = a seguido de b.
    Concat {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    /// Escreve `msg` no console, no nível dado (CON-004).
    Log {
        level: LogLevel,
        msg: Reg,
    },
    /// Entrada: ação pressionada, pressionada neste quadro, solta neste quadro.
    ActionPressed {
        dst: Reg,
        action: Reg,
    },
    ActionJustPressed {
        dst: Reg,
        action: Reg,
    },
    ActionReleased {
        dst: Reg,
        action: Reg,
    },
    /// Eixo (-1, 0 ou 1) e vetor das quatro ações.
    GetAxis {
        dst: Reg,
        negative: Reg,
        positive: Reg,
    },
    GetVector {
        dst: Reg,
        left: Reg,
        right: Reg,
        up: Reg,
        down: Reg,
    },
    /// Posição do mouse no mundo e na tela.
    MouseWorld {
        dst: Reg,
    },
    MouseScreen {
        dst: Reg,
    },
    /// `dst` = propriedade `name` do nó `node`.
    GetProp {
        dst: Reg,
        node: Reg,
        name: Reg,
    },
    /// Grava `value` na propriedade `name` do nó `node`.
    SetProp {
        node: Reg,
        name: Reg,
        value: Reg,
    },
    /// `dst` = o nó da cena com o nome dado (0 quando não existe).
    FindNode {
        dst: Reg,
        name: Reg,
    },
    /// `dst` = referência do nó criado ao instanciar a cena no ponto dado.
    InstanceScene {
        dst: Reg,
        scene: Reg,
        position: Reg,
    },
    /// Remove o nó `node` da cena.
    Destroy {
        node: Reg,
    },
    /// `dst` = se o corpo está no chão depois de mover com a velocidade dada.
    MoveAndSlide {
        dst: Reg,
        node: Reg,
        velocity: Reg,
    },
    /// Comando de áudio para o host.
    Audio(AudioOp),
    /// Salta para a op de índice `target`.
    Jump {
        target: usize,
    },
    /// Salta para `target` quando `cond` é falso.
    JumpIfFalse {
        cond: Reg,
        target: usize,
    },
    /// Latente: pausa a tarefa pelos segundos em `seconds`.
    Wait {
        seconds: Reg,
    },
    /// Latente: interpola a propriedade `name` do nó `node` até `to`, em `seconds`.
    Tween {
        node: Reg,
        name: Reg,
        to: Reg,
        seconds: Reg,
    },
    /// Latente: interpola a posição do nó até `target`, em `seconds`.
    MoveTo {
        node: Reg,
        target: Reg,
        seconds: Reg,
    },
    /// Latente: espera o sinal com o nome `name` ser emitido.
    AwaitSignal {
        name: Reg,
    },
    /// Emite o sinal `name`. Acorda as tarefas que aguardam esse nome no próximo quadro.
    EmitSignal {
        name: Reg,
    },
    /// Cria uma tarefa nova que começa em `entry` depois de `seconds`. A tarefa atual segue adiante.
    Spawn {
        seconds: Reg,
        entry: usize,
    },
    /// Pede a troca do estado ativo para o nome em `name`. A VM anota o pedido e o agendador faz a troca
    /// (sai o estado antigo, entra o novo) logo depois da op.
    GoToState {
        name: Reg,
    },
}

/// Programa de um nó de evento: as ops que rodam quando o evento dispara.
#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    /// Nome do evento que dispara o programa: "OnReady", "OnProcess"...
    pub event: String,
    /// Id do nó de evento de onde o programa veio.
    pub node: u64,
    pub ops: Vec<Op>,
    /// Nó de origem de cada op, no mesmo índice de `ops`. Serve para os erros, o rastro e os pontos de parada.
    pub sources: Vec<u64>,
    /// Quantidade de registradores que o programa usa.
    pub regs: usize,
    /// Nome do estado, nos eventos "State", "StateEnter" e "StateExit". Nos outros eventos, vazio.
    pub estado: Option<String>,
}

/// Resultado da compilação: as variáveis do script, um programa por nó de evento e o estado inicial.
#[derive(Clone, Debug, PartialEq)]
pub struct Compiled {
    /// Variáveis do script, na ordem em que as ops `GetVar` e `SetVar` as indexam.
    pub variables: Vec<Variable>,
    pub programs: Vec<Program>,
    /// Estado inicial: o primeiro nó "State" do grafo. `None` quando o grafo não tem estados.
    pub estado_inicial: Option<String>,
}

impl Compiled {
    /// Contexto de uma instância do script, com as variáveis no valor padrão e no estado inicial.
    pub fn new_context(&self, owner: u64) -> Context {
        Context {
            owner,
            delta: 0.0,
            vars: self.variables.iter().map(|v| v.default.clone()).collect(),
            estado: self.estado_inicial.clone().unwrap_or_default(),
        }
    }

    /// Programas de um evento, na ordem do grafo.
    pub fn programs_for<'a>(&'a self, event: &'a str) -> impl Iterator<Item = &'a Program> + 'a {
        self.programs.iter().filter(move |p| p.event == event)
    }
}

fn err(node: u64, message: impl Into<String>) -> CompileError {
    CompileError {
        node: Some(node),
        message: message.into(),
    }
}

/// Compila o grafo: um programa por nó de evento. Primeiro roda `Graph::validate` (o mesmo que o editor usa),
/// e só compila se não houver erro. Devolve todos os erros de validação que achar.
pub fn compile(graph: &Graph) -> Result<Compiled, Vec<CompileError>> {
    let mut errors = graph.validate();
    if !errors.is_empty() {
        return Err(errors);
    }
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
    let estado_inicial = programs
        .iter()
        .find(|p| p.event == "State")
        .and_then(|p| p.estado.clone());
    Ok(Compiled {
        variables: graph.variables.clone(),
        programs,
        estado_inicial,
    })
}

fn is_event(node: &GraphNode) -> bool {
    library::find(&node.kind).is_some_and(|d| d.event)
}

fn compile_event(graph: &Graph, event: &GraphNode) -> Result<Program, CompileError> {
    let mut e = Emitter {
        graph,
        ops: Vec::new(),
        sources: Vec::new(),
        regs: 0,
        flow_stack: Vec::new(),
        value_stack: Vec::new(),
        produzidas: BTreeMap::new(),
        chamadas: Vec::new(),
        retornos: Vec::new(),
    };
    e.flow(event.id, "exec")?;
    // Estados guardam o nome no pino "state" do evento; os demais eventos não têm estado.
    let estado = match event.kind.as_str() {
        "State" | "StateEnter" | "StateExit" => Some(texto_embutido(event, "state")),
        _ => None,
    };
    Ok(Program {
        event: event.kind.clone(),
        node: event.id,
        ops: e.ops,
        sources: e.sources,
        regs: e.regs,
        estado,
    })
}

/// Texto embutido num pino do nó, ou vazio quando o pino não tem valor embutido.
fn texto_embutido(node: &GraphNode, pin: &str) -> String {
    match node.values.get(pin) {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
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
    /// Saídas de dados que um nó de fluxo produz quando roda (índice do ForEach, instância criada, no chão).
    /// Quem lê essas saídas usa o registrador que o nó preencheu, sem refazer a operação.
    produzidas: BTreeMap<(u64, String), Reg>,
    /// Funções em compilação, pelo id do nó "Function". As chamadas são embutidas no lugar (sem pilha de
    /// execução), então uma função que chama a si mesma é recusada.
    chamadas: Vec<u64>,
    /// Saltos de Return à espera do fim do corpo da função que está sendo embutida, uma lista por chamada.
    retornos: Vec<Vec<usize>>,
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
        match &mut self.ops[at] {
            Op::Jump { target: t } | Op::JumpIfFalse { target: t, .. } => *t = target,
            Op::Spawn { entry, .. } => *entry = target,
            _ => {}
        }
    }

    /// Emite o que vem depois do pino de fluxo `pin` do nó `from`. Vários fios saindo do mesmo pino rodam
    /// na ordem em que foram criados.
    fn flow(&mut self, from: u64, pin: &str) -> Result<(), CompileError> {
        let graph = self.graph;
        for w in graph
            .wires
            .iter()
            .filter(|w| w.from.node == from && w.from.pin == pin)
        {
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
            "Print" | "Warn" | "Error" => {
                let level = match node.kind.as_str() {
                    "Warn" => LogLevel::Warning,
                    "Error" => LogLevel::Error,
                    _ => LogLevel::Info,
                };
                let msg = self.input(node, "msg")?;
                self.emit(Op::Log { level, msg }, id);
                self.flow(id, "exec")
            }
            "Destroy" => {
                let target = self.input(node, "node")?;
                self.emit(Op::Destroy { node: target }, id);
                self.flow(id, "exec")
            }
            "SetProperty" => {
                let r = self.entradas(node, &["node", "name", "value"])?;
                self.emit(
                    Op::SetProp {
                        node: r[0],
                        name: r[1],
                        value: r[2],
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
            "ForEach" => {
                // idx = 0; cabeça: se !(idx < count) vai ao fim; corpo; idx += 1; volta à cabeça; fim:
                let count = self.input(node, "count")?;
                let idx = self.reg();
                self.emit(
                    Op::Const {
                        dst: idx,
                        value: Value::Int(0),
                    },
                    id,
                );
                self.produzidas.insert((id, "index".into()), idx);
                let head = self.ops.len();
                let cond = self.reg();
                self.emit(
                    Op::IntLess {
                        dst: cond,
                        a: idx,
                        b: count,
                    },
                    id,
                );
                let to_end = self.emit(Op::JumpIfFalse { cond, target: 0 }, id);
                self.flow(id, "body")?;
                let um = self.reg();
                self.emit(
                    Op::Const {
                        dst: um,
                        value: Value::Int(1),
                    },
                    id,
                );
                self.emit(
                    Op::IntAdd {
                        dst: idx,
                        a: idx,
                        b: um,
                    },
                    id,
                );
                self.emit(Op::Jump { target: head }, id);
                let end = self.ops.len();
                self.patch(to_end, end);
                self.flow(id, "done")
            }
            "While" => {
                // cabeça: condição avaliada a cada volta; se falsa vai ao fim; corpo; volta à cabeça; fim:
                let head = self.ops.len();
                let cond = self.input(node, "condition")?;
                let to_end = self.emit(Op::JumpIfFalse { cond, target: 0 }, id);
                self.flow(id, "body")?;
                self.emit(Op::Jump { target: head }, id);
                let end = self.ops.len();
                self.patch(to_end, end);
                self.flow(id, "done")
            }
            "Wait" => {
                let seconds = self.input(node, "seconds")?;
                self.emit(Op::Wait { seconds }, id);
                self.flow(id, "exec")
            }
            "Tween" => {
                let r = self.entradas(node, &["node", "property", "to", "seconds"])?;
                self.emit(
                    Op::Tween {
                        node: r[0],
                        name: r[1],
                        to: r[2],
                        seconds: r[3],
                    },
                    id,
                );
                self.flow(id, "exec")
            }
            "MoveTo" => {
                let r = self.entradas(node, &["node", "target", "seconds"])?;
                self.emit(
                    Op::MoveTo {
                        node: r[0],
                        target: r[1],
                        seconds: r[2],
                    },
                    id,
                );
                self.flow(id, "exec")
            }
            "AwaitSignal" => {
                let name = self.input(node, "name")?;
                self.emit(Op::AwaitSignal { name }, id);
                self.flow(id, "exec")
            }
            "EmitSignal" => {
                let name = self.input(node, "name")?;
                self.emit(Op::EmitSignal { name }, id);
                self.flow(id, "exec")
            }
            "TimerStart" => {
                // Spawn(segundos, entrada) cria a tarefa do temporizador; a atual segue por "Iniciado".
                let seconds = self.input(node, "seconds")?;
                let spawn = self.emit(Op::Spawn { seconds, entry: 0 }, id);
                self.flow(id, "started")?;
                let to_end = self.emit(Op::Jump { target: 0 }, id);
                let entry = self.ops.len();
                self.patch(spawn, entry);
                self.flow(id, "timeout")?;
                let end = self.ops.len();
                self.patch(to_end, end);
                Ok(())
            }
            "InstanceScene" => {
                let r = self.entradas(node, &["scene", "position"])?;
                let dst = self.reg();
                self.emit(
                    Op::InstanceScene {
                        dst,
                        scene: r[0],
                        position: r[1],
                    },
                    id,
                );
                self.produzidas.insert((id, "instance".into()), dst);
                self.flow(id, "exec")
            }
            "MoveAndSlide" => {
                let r = self.entradas(node, &["node", "velocity"])?;
                let dst = self.reg();
                self.emit(
                    Op::MoveAndSlide {
                        dst,
                        node: r[0],
                        velocity: r[1],
                    },
                    id,
                );
                self.produzidas.insert((id, "on_floor".into()), dst);
                self.flow(id, "exec")
            }
            "PlaySound" => {
                let r = self.entradas(
                    node,
                    &["node", "stream", "volume", "pitch", "bus", "loop", "reiniciar"],
                )?;
                self.emit(
                    Op::Audio(AudioOp::Play {
                        node: r[0],
                        stream: r[1],
                        volume_db: r[2],
                        pitch: r[3],
                        bus: r[4],
                        looping: r[5],
                        from_start: r[6],
                    }),
                    id,
                );
                self.flow(id, "exec")
            }
            "StopSound" => {
                let node_reg = self.input(node, "node")?;
                self.emit(Op::Audio(AudioOp::Stop { node: node_reg }), id);
                self.flow(id, "exec")
            }
            "FadeSound" => {
                let r = self.entradas(node, &["node", "volume", "seconds"])?;
                self.emit(
                    Op::Audio(AudioOp::Fade {
                        node: r[0],
                        to_db: r[1],
                        seconds: r[2],
                    }),
                    id,
                );
                self.flow(id, "exec")
            }
            "SetBusVolume" => {
                let r = self.entradas(node, &["bus", "volume"])?;
                self.emit(
                    Op::Audio(AudioOp::SetBus {
                        bus: r[0],
                        volume_db: r[1],
                    }),
                    id,
                );
                self.flow(id, "exec")
            }
            // O reroute só repassa o fluxo: a execução segue pelos fios que saem dele.
            REROUTE => self.flow(id, "out"),
            "CallFunction" => {
                // O corpo da função é embutido aqui. O que vem depois do Return cai no fim do corpo, e a
                // chamada segue pelo fluxo de saída.
                let graph = self.graph;
                let nome = texto_embutido(node, "name");
                let def = graph
                    .nodes
                    .iter()
                    .find(|n| n.kind == "Function" && texto_embutido(n, "name") == nome)
                    .ok_or_else(|| err(id, format!("função '{nome}' não existe")))?;
                if self.chamadas.contains(&def.id) {
                    return Err(err(id, format!("chamada recursiva da função '{nome}'")));
                }
                self.chamadas.push(def.id);
                self.retornos.push(Vec::new());
                self.flow(def.id, "exec")?;
                let fim = self.ops.len();
                for salto in self.retornos.pop().unwrap_or_default() {
                    self.patch(salto, fim);
                }
                self.chamadas.pop();
                self.flow(id, "exec")
            }
            "Return" => {
                if self.retornos.is_empty() {
                    return Err(err(id, "retorno fora de uma função"));
                }
                let salto = self.emit(Op::Jump { target: 0 }, id);
                if let Some(corpo) = self.retornos.last_mut() {
                    corpo.push(salto);
                }
                Ok(())
            }
            "GoToState" => {
                let name = self.input(node, "state")?;
                self.emit(Op::GoToState { name }, id);
                self.flow(id, "exec")
            }
            other => Err(err(id, format!("{other} não é um nó de fluxo"))),
        }
    }

    /// Registradores das entradas `pins`, na mesma ordem. Cada uma segue a regra de `input`.
    fn entradas(&mut self, node: &'g GraphNode, pins: &[&str]) -> Result<Vec<Reg>, CompileError> {
        let mut regs = Vec::with_capacity(pins.len());
        for pin in pins {
            regs.push(self.input(node, pin)?);
        }
        Ok(regs)
    }

    /// Registrador com o valor da entrada `pin`: pelo fio, se houver; senão, pelo valor embutido ou pelo padrão.
    fn input(&mut self, node: &'g GraphNode, pin: &str) -> Result<Reg, CompileError> {
        let graph = self.graph;
        if let Some(w) = graph
            .wires
            .iter()
            .find(|w| w.to.node == node.id && w.to.pin == pin)
        {
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
                format!(
                    "entrada obrigatória '{pin}' de {} não está conectada",
                    node.etiqueta()
                ),
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
    /// dados não têm efeito colateral, quem age é o fluxo. As saídas que o fluxo produz são lidas do registrador
    /// que o nó preencheu.
    fn output(&mut self, id: u64, pin: &str) -> Result<Reg, CompileError> {
        if self.value_stack.contains(&id) {
            return Err(err(id, format!("ciclo de dados no nó {id}")));
        }
        let graph = self.graph;
        let node = graph
            .node(id)
            .ok_or_else(|| err(id, format!("nó {id} não existe no grafo")))?;
        if let Some(&reg) = self.produzidas.get(&(id, pin.to_string())) {
            return Ok(reg);
        }
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
            ("Add", "result") => self.binario(node, |dst, a, b| Op::Add { dst, a, b }),
            ("Subtract", "result") => self.binario(node, |dst, a, b| Op::Sub { dst, a, b }),
            ("Multiply", "result") => self.binario(node, |dst, a, b| Op::Mul { dst, a, b }),
            ("Divide", "result") => self.binario(node, |dst, a, b| Op::Div { dst, a, b }),
            ("Greater", "result") => self.binario(node, |dst, a, b| Op::Greater { dst, a, b }),
            ("Less", "result") => self.binario(node, |dst, a, b| Op::Less { dst, a, b }),
            ("Equal", "result") => self.binario(node, |dst, a, b| Op::Equal { dst, a, b }),
            ("Concat", "result") => self.binario(node, |dst, a, b| Op::Concat { dst, a, b }),
            ("AddVector2", "result") => self.binario(node, |dst, a, b| Op::AddVec { dst, a, b }),
            ("MakeVector2", "result") => {
                let r = self.entradas(node, &["x", "y"])?;
                let dst = self.reg();
                self.emit(
                    Op::MakeVec {
                        dst,
                        x: r[0],
                        y: r[1],
                    },
                    id,
                );
                Ok(dst)
            }
            ("ScaleVector2", "result") => {
                let r = self.entradas(node, &["vector", "scale"])?;
                let dst = self.reg();
                self.emit(
                    Op::ScaleVec {
                        dst,
                        vector: r[0],
                        scale: r[1],
                    },
                    id,
                );
                Ok(dst)
            }
            ("VectorLength", "result") => {
                let vector = self.input(node, "vector")?;
                let dst = self.reg();
                self.emit(Op::VecLength { dst, vector }, id);
                Ok(dst)
            }
            ("ActionPressed", "pressed") => {
                self.acao(node, |dst, action| Op::ActionPressed { dst, action })
            }
            ("ActionJustPressed", "pressed") => {
                self.acao(node, |dst, action| Op::ActionJustPressed { dst, action })
            }
            ("ActionReleased", "released") => {
                self.acao(node, |dst, action| Op::ActionReleased { dst, action })
            }
            ("GetAxis", "value") => {
                let r = self.entradas(node, &["negative", "positive"])?;
                let dst = self.reg();
                self.emit(
                    Op::GetAxis {
                        dst,
                        negative: r[0],
                        positive: r[1],
                    },
                    id,
                );
                Ok(dst)
            }
            ("GetVector", "value") => {
                let r = self.entradas(node, &["left", "right", "up", "down"])?;
                let dst = self.reg();
                self.emit(
                    Op::GetVector {
                        dst,
                        left: r[0],
                        right: r[1],
                        up: r[2],
                        down: r[3],
                    },
                    id,
                );
                Ok(dst)
            }
            ("MousePositionWorld", "position") => {
                let dst = self.reg();
                self.emit(Op::MouseWorld { dst }, id);
                Ok(dst)
            }
            ("MousePositionScreen", "position") => {
                let dst = self.reg();
                self.emit(Op::MouseScreen { dst }, id);
                Ok(dst)
            }
            ("GetNode", "node") => {
                let name = self.input(node, "name")?;
                let dst = self.reg();
                self.emit(Op::FindNode { dst, name }, id);
                Ok(dst)
            }
            ("GetProperty", "value") => {
                let r = self.entradas(node, &["node", "name"])?;
                let dst = self.reg();
                self.emit(
                    Op::GetProp {
                        dst,
                        node: r[0],
                        name: r[1],
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
            // O valor do reroute é o valor do fio que chega nele.
            (REROUTE, "out") => {
                let graph = self.graph;
                let w = graph
                    .wires
                    .iter()
                    .find(|w| w.to.node == id && w.to.pin == "in")
                    .ok_or_else(|| err(id, format!("{} sem fio de entrada", node.etiqueta())))?;
                self.output(w.from.node, &w.from.pin)
            }
            // Saídas de nós de fluxo: só existem depois que o nó roda, e ficam no mapa `produzidas`.
            ("InstanceScene", "instance") | ("MoveAndSlide", "on_floor") | ("ForEach", "index") => {
                Err(err(
                    id,
                    format!(
                        "a saída '{pin}' de {} é lida antes do nó rodar: ligue o nó ao fluxo antes de usar a saída",
                        node.etiqueta()
                    ),
                ))
            }
            _ => Err(err(
                id,
                format!("{} não tem a saída de valor '{pin}'", node.etiqueta()),
            )),
        }
    }

    /// Op binária de decimais ou vetores: lê as entradas `a` e `b` e grava a saída num registrador novo.
    fn binario(
        &mut self,
        node: &'g GraphNode,
        make: impl FnOnce(Reg, Reg, Reg) -> Op,
    ) -> Result<Reg, CompileError> {
        let r = self.entradas(node, &["a", "b"])?;
        let dst = self.reg();
        self.emit(make(dst, r[0], r[1]), node.id);
        Ok(dst)
    }

    /// Consulta de ação de entrada: lê o nome da ação e grava o resultado num registrador novo.
    fn acao(
        &mut self,
        node: &'g GraphNode,
        make: impl FnOnce(Reg, Reg) -> Op,
    ) -> Result<Reg, CompileError> {
        let action = self.input(node, "action")?;
        let dst = self.reg();
        self.emit(make(dst, action), node.id);
        Ok(dst)
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
        assert!(
            erros.iter().any(|e| e.message.contains("ciclo de fluxo")),
            "{erros:?}"
        );
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

    #[test]
    fn saida_de_no_de_fluxo_lida_antes_de_rodar_e_recusada() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        // O InstanceScene roda só no OnProcess; o SetProperty do OnReady lê a instância antes de ela existir.
        let ep = adiciona(&mut g, "OnProcess");
        let inst = adiciona(&mut g, "InstanceScene");
        g.set_value(inst, "scene", Value::String("bala.scene".into()))
            .unwrap();
        g.connect((ep, "exec"), (inst, "exec")).unwrap();
        let sp = adiciona(&mut g, "SetProperty");
        g.set_value(sp, "name", Value::String("position_x".into()))
            .unwrap();
        g.connect((ev, "exec"), (sp, "exec")).unwrap();
        g.connect((inst, "instance"), (sp, "node")).unwrap();
        let erros = compile(&g).unwrap_err();
        assert!(
            erros
                .iter()
                .any(|e| e.node == Some(inst) && e.message.contains("antes do nó rodar")),
            "{erros:?}"
        );
    }
}
