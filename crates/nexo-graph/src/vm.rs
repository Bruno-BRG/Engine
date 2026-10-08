//! Máquina virtual da IR (VIS-023). Executa um `Program` contra um `Host` (a ponte para o mundo do jogo) e
//! um `Context` (dono, delta e variáveis da instância). Não conhece a cena nem a física, por isso este crate
//! não depende do nexo-core. Cada op conta como um passo. Passado o orçamento, a execução vira erro em vez de
//! travar o jogo, o que cobre os laços de fluxo infinitos.

use crate::ir::{Op, Program, Reg};
use crate::types::Value;
use std::fmt;

/// O mundo do jogo visto pela VM. A cena implementa este trait.
pub trait Host {
    /// Valor numérico da propriedade `name` do nó `node`. None quando o nó ou a propriedade não existe.
    fn get_property(&mut self, node: u64, name: &str) -> Option<f64>;
    /// Grava a propriedade. False quando o nó ou a propriedade não existe ou não aceita o valor.
    fn set_property(&mut self, node: u64, name: &str, value: f64) -> bool;
    /// True enquanto a ação de entrada `action` está pressionada.
    fn action_pressed(&self, action: &str) -> bool;
    /// Recebe o texto do nó Print.
    fn print(&mut self, msg: &str);
    /// Pede a remoção do nó.
    fn destroy(&mut self, node: u64);
}

/// Estado de uma instância do script entre as execuções.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Id do nó que roda o script. É a saída "self" dos eventos.
    pub owner: u64,
    /// Tempo do quadro, em segundos. É a saída "delta" do OnProcess.
    pub delta: f64,
    /// Valores das variáveis, na ordem de `Compiled::variables`.
    pub vars: Vec<Value>,
}

/// Falha na execução. Guarda o nó de origem da op que falhou.
#[derive(Clone, Debug, PartialEq)]
pub enum VmError {
    /// Passou do orçamento de passos. Quase sempre é um laço de fluxo sem saída.
    BudgetExhausted { limit: u32, node: u64 },
    /// O host não achou a propriedade para ler, ou não aceitou o valor para gravar (`write`).
    PropertyFailed { node: u64, name: String, write: bool },
    /// A IR não bate com o que a VM espera. Programas feitos por `compile` nunca chegam aqui.
    Malformed { node: u64, message: String },
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VmError::BudgetExhausted { limit, node } => {
                write!(f, "orçamento de {limit} passos esgotado no nó {node} (laço infinito?)")
            }
            VmError::PropertyFailed {
                node,
                name,
                write: true,
            } => {
                write!(f, "não foi possível gravar '{name}' no nó {node}")
            }
            VmError::PropertyFailed {
                node,
                name,
                write: false,
            } => {
                write!(f, "não foi possível ler '{name}' do nó {node}")
            }
            VmError::Malformed { node, message } => write!(f, "IR inválida no nó {node}: {message}"),
        }
    }
}

impl std::error::Error for VmError {}

/// A máquina virtual. Não guarda estado: tudo fica no `Context` e no `Host`.
pub struct Vm;

impl Vm {
    /// Orçamento padrão de passos por execução. Cada op conta como um passo.
    pub const MAX_STEPS: u32 = 10_000;

    /// Executa o programa de um evento com o orçamento padrão.
    pub fn run(program: &Program, host: &mut dyn Host, ctx: &mut Context) -> Result<(), VmError> {
        Vm::run_with_budget(program, host, ctx, Vm::MAX_STEPS)
    }

    /// Como `run`, com outro orçamento de passos.
    pub fn run_with_budget(
        program: &Program,
        host: &mut dyn Host,
        ctx: &mut Context,
        limit: u32,
    ) -> Result<(), VmError> {
        let mut regs = vec![Value::Bool(false); program.regs];
        let mut pc = 0usize;
        let mut steps = 0u32;
        while let Some(op) = program.ops.get(pc) {
            // Nó de origem da op, para o erro dizer de onde veio.
            let origem = program.sources.get(pc).copied().unwrap_or(program.node);
            steps = steps.saturating_add(1);
            if steps > limit {
                return Err(VmError::BudgetExhausted { limit, node: origem });
            }
            pc += 1;
            match op {
                Op::Const { dst, value } => set(&mut regs, *dst, value.clone(), origem)?,
                Op::Owner { dst } => set(&mut regs, *dst, Value::Node(ctx.owner), origem)?,
                Op::Delta { dst } => set(&mut regs, *dst, Value::Float(ctx.delta), origem)?,
                Op::GetVar { dst, var } => {
                    let v = ctx
                        .vars
                        .get(*var)
                        .cloned()
                        .ok_or_else(|| malformed(origem, format!("variável {var} não existe")))?;
                    set(&mut regs, *dst, v, origem)?;
                }
                Op::SetVar { var, src } => {
                    let v = slot(&regs, *src, origem)?.clone();
                    let destino = ctx
                        .vars
                        .get_mut(*var)
                        .ok_or_else(|| malformed(origem, format!("variável {var} não existe")))?;
                    *destino = v;
                }
                Op::Add { dst, a, b } => {
                    let r = float(&regs, *a, origem)? + float(&regs, *b, origem)?;
                    set(&mut regs, *dst, Value::Float(r), origem)?;
                }
                Op::Mul { dst, a, b } => {
                    let r = float(&regs, *a, origem)? * float(&regs, *b, origem)?;
                    set(&mut regs, *dst, Value::Float(r), origem)?;
                }
                Op::Greater { dst, a, b } => {
                    let r = float(&regs, *a, origem)? > float(&regs, *b, origem)?;
                    set(&mut regs, *dst, Value::Bool(r), origem)?;
                }
                Op::ActionPressed { dst, action } => {
                    let pressed = host.action_pressed(text(&regs, *action, origem)?);
                    set(&mut regs, *dst, Value::Bool(pressed), origem)?;
                }
                Op::GetProp { dst, node, name } => {
                    let alvo = node_id(&regs, *node, origem)?;
                    let nome = text(&regs, *name, origem)?.to_string();
                    let v = host.get_property(alvo, &nome).ok_or_else(|| VmError::PropertyFailed {
                        node: origem,
                        name: nome.clone(),
                        write: false,
                    })?;
                    set(&mut regs, *dst, Value::Float(v), origem)?;
                }
                Op::SetProp { node, name, value } => {
                    let alvo = node_id(&regs, *node, origem)?;
                    let nome = text(&regs, *name, origem)?.to_string();
                    let v = float(&regs, *value, origem)?;
                    if !host.set_property(alvo, &nome, v) {
                        return Err(VmError::PropertyFailed {
                            node: origem,
                            name: nome,
                            write: true,
                        });
                    }
                }
                Op::Print { msg } => host.print(text(&regs, *msg, origem)?),
                Op::Destroy { node } => host.destroy(node_id(&regs, *node, origem)?),
                Op::Jump { target } => pc = *target,
                Op::JumpIfFalse { cond, target } => {
                    if !boolean(&regs, *cond, origem)? {
                        pc = *target;
                    }
                }
            }
        }
        Ok(())
    }
}

fn malformed(node: u64, message: impl Into<String>) -> VmError {
    VmError::Malformed {
        node,
        message: message.into(),
    }
}

/// Lê um registrador. Erro se o índice está fora do programa.
fn slot(regs: &[Value], i: Reg, node: u64) -> Result<&Value, VmError> {
    regs.get(i)
        .ok_or_else(|| malformed(node, format!("registrador {i} fora do programa")))
}

fn float(regs: &[Value], i: Reg, node: u64) -> Result<f64, VmError> {
    match slot(regs, i, node)? {
        Value::Float(f) => Ok(*f),
        other => Err(malformed(
            node,
            format!("registrador {i} deveria ser decimal, é {}", other.ty().nome()),
        )),
    }
}

fn boolean(regs: &[Value], i: Reg, node: u64) -> Result<bool, VmError> {
    match slot(regs, i, node)? {
        Value::Bool(b) => Ok(*b),
        other => Err(malformed(
            node,
            format!("registrador {i} deveria ser booleano, é {}", other.ty().nome()),
        )),
    }
}

fn text(regs: &[Value], i: Reg, node: u64) -> Result<&str, VmError> {
    match slot(regs, i, node)? {
        Value::String(s) => Ok(s.as_str()),
        other => Err(malformed(
            node,
            format!("registrador {i} deveria ser texto, é {}", other.ty().nome()),
        )),
    }
}

fn node_id(regs: &[Value], i: Reg, node: u64) -> Result<u64, VmError> {
    match slot(regs, i, node)? {
        Value::Node(id) => Ok(*id),
        other => Err(malformed(
            node,
            format!("registrador {i} deveria ser nó, é {}", other.ty().nome()),
        )),
    }
}

/// Grava um valor num registrador.
fn set(regs: &mut [Value], i: Reg, value: Value, node: u64) -> Result<(), VmError> {
    match regs.get_mut(i) {
        Some(slot) => {
            *slot = value;
            Ok(())
        }
        None => Err(malformed(node, format!("registrador {i} fora do programa"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Graph;
    use crate::ir::{Op, Program, compile};
    use crate::types::{PinType, Value};
    use std::collections::HashMap;

    /// Host de teste: propriedades num mapa, ações pressionadas e o registro das chamadas.
    #[derive(Default)]
    struct Mock {
        props: HashMap<(u64, String), f64>,
        pressionadas: Vec<String>,
        impressos: Vec<String>,
        destruidos: Vec<u64>,
    }

    impl Host for Mock {
        fn get_property(&mut self, node: u64, name: &str) -> Option<f64> {
            self.props.get(&(node, name.to_string())).copied()
        }

        fn set_property(&mut self, node: u64, name: &str, value: f64) -> bool {
            match self.props.get_mut(&(node, name.to_string())) {
                Some(v) => {
                    *v = value;
                    true
                }
                None => false,
            }
        }

        fn action_pressed(&self, action: &str) -> bool {
            self.pressionadas.iter().any(|a| a == action)
        }

        fn print(&mut self, msg: &str) {
            self.impressos.push(msg.to_string());
        }

        fn destroy(&mut self, node: u64) {
            self.destruidos.push(node);
        }
    }

    fn adiciona(g: &mut Graph, kind: &str) -> u64 {
        g.add_node(kind, [0.0, 0.0]).unwrap()
    }

    fn texto(g: &mut Graph, id: u64, pin: &str, valor: &str) {
        g.set_value(id, pin, Value::String(valor.into())).unwrap();
    }

    fn pos_x(host: &Mock) -> f64 {
        host.props[&(7, "position_x".to_string())]
    }

    /// Exemplo do enunciado: no OnProcess, se "ui_right" está pressionada, a posição X do próprio nó
    /// sobe delta * 100. Equivale a SetProperty(node, "position_x", GetProperty(node, "position_x") + delta * 100).
    fn grafo_de_movimento() -> Graph {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnProcess");
        let acao = adiciona(&mut g, "ActionPressed");
        texto(&mut g, acao, "action", "ui_right");
        let br = adiciona(&mut g, "Branch");
        let sp = adiciona(&mut g, "SetProperty");
        texto(&mut g, sp, "name", "position_x");
        let gp = adiciona(&mut g, "GetProperty");
        texto(&mut g, gp, "name", "position_x");
        let mul = adiciona(&mut g, "Multiply");
        g.set_value(mul, "b", Value::Float(100.0)).unwrap();
        let soma = adiciona(&mut g, "Add");

        g.connect((ev, "exec"), (br, "exec")).unwrap();
        g.connect((acao, "pressed"), (br, "condition")).unwrap();
        g.connect((br, "true"), (sp, "exec")).unwrap();
        g.connect((ev, "self"), (sp, "node")).unwrap();
        g.connect((ev, "self"), (gp, "node")).unwrap();
        g.connect((gp, "value"), (soma, "a")).unwrap();
        g.connect((ev, "delta"), (mul, "a")).unwrap();
        g.connect((mul, "result"), (soma, "b")).unwrap();
        g.connect((soma, "result"), (sp, "value")).unwrap();
        g
    }

    #[test]
    fn movimento_so_acontece_com_a_acao_pressionada() {
        let g = grafo_de_movimento();
        let compiled = compile(&g).unwrap();
        let program = compiled.programs_for("OnProcess").next().unwrap();
        let mut host = Mock::default();
        host.props.insert((7, "position_x".to_string()), 10.0);
        let mut ctx = compiled.new_context(7);
        ctx.delta = 0.5;

        // Sem a ação pressionada, o Branch pula o SetProperty.
        Vm::run(program, &mut host, &mut ctx).unwrap();
        assert_eq!(pos_x(&host), 10.0);

        host.pressionadas.push("ui_right".into());
        Vm::run(program, &mut host, &mut ctx).unwrap();
        // 10 + 0.5 * 100
        assert_eq!(pos_x(&host), 60.0);
    }

    #[test]
    fn branch_escolhe_o_caminho_certo() {
        for (cond, esperado) in [(true, "sim"), (false, "nao")] {
            let mut g = Graph::default();
            let ev = adiciona(&mut g, "OnReady");
            let br = adiciona(&mut g, "Branch");
            g.set_value(br, "condition", Value::Bool(cond)).unwrap();
            let sim = adiciona(&mut g, "Print");
            texto(&mut g, sim, "msg", "sim");
            let nao = adiciona(&mut g, "Print");
            texto(&mut g, nao, "msg", "nao");
            g.connect((ev, "exec"), (br, "exec")).unwrap();
            g.connect((br, "true"), (sim, "exec")).unwrap();
            g.connect((br, "false"), (nao, "exec")).unwrap();

            let compiled = compile(&g).unwrap();
            let program = compiled.programs_for("OnReady").next().unwrap();
            let mut host = Mock::default();
            Vm::run(program, &mut host, &mut compiled.new_context(1)).unwrap();
            assert_eq!(host.impressos, [esperado]);
        }
    }

    #[test]
    fn sequencia_roda_passos_em_ordem_e_destroy_chega_ao_host() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let seq = adiciona(&mut g, "Sequence");
        let p = adiciona(&mut g, "Print");
        texto(&mut g, p, "msg", "primeiro");
        let d = adiciona(&mut g, "Destroy");
        g.connect((ev, "exec"), (seq, "exec")).unwrap();
        g.connect((seq, "then_0"), (p, "exec")).unwrap();
        g.connect((seq, "then_1"), (d, "exec")).unwrap();
        g.connect((ev, "self"), (d, "node")).unwrap();

        let compiled = compile(&g).unwrap();
        let program = compiled.programs_for("OnReady").next().unwrap();
        let mut host = Mock::default();
        Vm::run(program, &mut host, &mut compiled.new_context(7)).unwrap();
        assert_eq!(host.impressos, ["primeiro"]);
        assert_eq!(host.destruidos, [7]);
    }

    #[test]
    fn variavel_persiste_entre_execucoes() {
        let mut g = Graph::default();
        g.add_variable("contador", PinType::Float, Value::Float(0.0), true)
            .unwrap();
        let ev = adiciona(&mut g, "OnProcess");
        let le = adiciona(&mut g, "GetVariable");
        texto(&mut g, le, "name", "contador");
        let soma = adiciona(&mut g, "Add");
        g.set_value(soma, "b", Value::Float(1.0)).unwrap();
        let grava = adiciona(&mut g, "SetVariable");
        texto(&mut g, grava, "name", "contador");
        g.connect((ev, "exec"), (grava, "exec")).unwrap();
        g.connect((le, "value"), (soma, "a")).unwrap();
        g.connect((soma, "result"), (grava, "value")).unwrap();

        let compiled = compile(&g).unwrap();
        let program = compiled.programs_for("OnProcess").next().unwrap();
        let mut ctx = compiled.new_context(1);
        let mut host = Mock::default();
        Vm::run(program, &mut host, &mut ctx).unwrap();
        Vm::run(program, &mut host, &mut ctx).unwrap();
        assert_eq!(ctx.vars[0], Value::Float(2.0));
    }

    #[test]
    fn propriedade_inexistente_vira_erro() {
        let compiled = compile(&grafo_de_movimento()).unwrap();
        let program = compiled.programs_for("OnProcess").next().unwrap();
        // O host não tem nenhuma propriedade: a leitura do GetProperty falha.
        let mut host = Mock::default();
        host.pressionadas.push("ui_right".into());
        let erro = Vm::run(program, &mut host, &mut compiled.new_context(7)).unwrap_err();
        assert!(matches!(erro, VmError::PropertyFailed { write: false, .. }), "{erro:?}");
    }

    #[test]
    fn laco_infinito_esgota_o_orcamento() {
        // Um salto para si mesmo nunca termina. Com o orçamento padrão, vira erro.
        let program = Program {
            event: "OnProcess".into(),
            node: 5,
            ops: vec![Op::Jump { target: 0 }],
            sources: vec![5],
            regs: 0,
        };
        let mut ctx = Context {
            owner: 5,
            delta: 0.0,
            vars: Vec::new(),
        };
        let erro = Vm::run(&program, &mut Mock::default(), &mut ctx).unwrap_err();
        assert_eq!(
            erro,
            VmError::BudgetExhausted {
                limit: Vm::MAX_STEPS,
                node: 5
            }
        );
    }
}
