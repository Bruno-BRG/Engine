//! Máquina virtual da IR (VIS-023, VIS-025). Executa um `Program` contra um `Host` (a ponte para o mundo do jogo)
//! e um `Context` (dono, delta e variáveis da instância). `Vm::run` roda um programa sem pausas. Os programas com
//! pontos latentes (Wait, Tween, Move To, Await Signal, Timer) rodam pelo `Scheduler` (ver `scheduler.rs`), que
//! usa as mesmas ops. Cada op conta como um passo. Passado o orçamento, a execução vira erro em vez de travar o
//! jogo, o que cobre os laços de fluxo infinitos.

use crate::ir::{AudioOp, Op, Program, Reg};
use crate::types::{LogLevel, Value};
use nexo_core::{AudioCommand, Uid};
use std::collections::BTreeSet;
use std::fmt;

/// O mundo do jogo visto pela VM. A cena implementa este trait. Cada método abaixo é um contrato: o que
/// significa o retorno e o que o host deve fazer está escrito em cada um.
pub trait Host {
    /// Lê a propriedade numérica `name` do nó `node`. `None` quando o nó ou a propriedade não existe.
    fn get_property(&mut self, node: u64, name: &str) -> Option<f64>;
    /// Grava a propriedade numérica. `false` quando o nó ou a propriedade não existe ou não aceita o valor.
    fn set_property(&mut self, node: u64, name: &str, value: f64) -> bool;
    /// Entrada: `true` enquanto a ação `action` está pressionada (qualquer quadro).
    fn action_pressed(&self, action: &str) -> bool;
    /// Entrada: `true` só no quadro em que a ação passou de solta para pressionada.
    fn action_just_pressed(&self, action: &str) -> bool;
    /// Entrada: `true` só no quadro em que a ação passou de pressionada para solta.
    fn action_released(&self, action: &str) -> bool;
    /// Entrada: -1 se a ação `negative` está pressionada, 1 se `positive`, 0 com nenhuma ou com as duas.
    fn axis(&self, negative: &str, positive: &str) -> f64;
    /// Entrada: vetor das quatro ações, com comprimento no máximo 1 (diagonal não é mais rápida).
    fn vector(&self, left: &str, right: &str, up: &str, down: &str) -> [f64; 2];
    /// Posição do ponteiro no mundo do jogo (coordenadas do mundo, depois da câmera).
    fn mouse_world(&self) -> [f64; 2];
    /// Posição do ponteiro na tela, em pixels da janela.
    fn mouse_screen(&self) -> [f64; 2];
    /// Escreve uma mensagem no console do editor, no nível dado (Print, Warn ou Error).
    fn log(&mut self, level: LogLevel, message: &str);
    /// Acha um nó da cena pelo nome. `None` quando não existe. A VM entrega a referência vazia (id 0).
    fn find_node(&self, name: &str) -> Option<u64>;
    /// Cria uma cópia da cena `scene` na posição dada e devolve o id do nó criado. `None` quando a cena não
    /// existe ou não pode ser instanciada; a VM para com erro.
    fn instance_scene(&mut self, scene: &str, position: [f64; 2]) -> Option<u64>;
    /// Remove o nó da cena. Nó inexistente é ignorado.
    fn destroy(&mut self, node: u64);
    /// Move o corpo `node` com a velocidade dada, parando nas paredes (como `move_and_slide` da Godot). Devolve
    /// se ele está no chão depois do movimento. `None` quando o nó não é um corpo que se move; a VM para com erro.
    fn move_and_slide(&mut self, node: u64, velocity: [f64; 2]) -> Option<bool>;
    /// Executa um comando de áudio do núcleo (`nexo_core::AudioCommand`: tocar, parar, fade, volume de barramento).
    /// Nó ou barramento desconhecido é responsabilidade do host: ele deve registrar o problema, e a VM segue.
    fn audio(&mut self, command: AudioCommand);
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
    /// Estado ativo (VIS-027). Vazio quando o grafo não tem estados.
    pub estado: String,
}

/// Evento de rastro (VIS-030, DBG-003). O editor usa para pulsar os fios e mostrar valores no hover.
#[derive(Clone, Debug, PartialEq)]
pub enum TraceEvent {
    /// Uma op do nó `node` rodou, no evento `evento`.
    Executou { evento: String, node: u64 },
    /// A op do nó `node` produziu `valor` no seu registrador de saída. A saída em si é associada pelo editor.
    Valor { node: u64, valor: Value },
}

/// Falha na execução. Guarda o nó de origem da op que falhou.
#[derive(Clone, Debug, PartialEq)]
pub enum VmError {
    /// Passou do orçamento de passos. Quase sempre é um laço de fluxo sem saída.
    BudgetExhausted { limit: u32, node: u64 },
    /// O host não achou a propriedade para ler, ou não aceitou o valor para gravar (`write`).
    PropertyFailed {
        node: u64,
        name: String,
        write: bool,
    },
    /// O host recusou uma ação que a VM precisa do retorno (instanciar cena, mover corpo).
    HostFailed { node: u64, action: String },
    /// Divisão por zero no nó Divide.
    DivisionByZero { node: u64 },
    /// `Vm::run` não sustenta pausas. O programa tem um ponto latente e precisa do `Scheduler`.
    Suspended { node: u64 },
    /// A IR não bate com o que a VM espera. Programas feitos por `compile` nunca chegam aqui.
    Malformed { node: u64, message: String },
    /// Um valor que veio de um pino do jogo não é válido (por exemplo, o UID de um som que não é um UID).
    InvalidValue { node: u64, message: String },
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VmError::BudgetExhausted { limit, node } => {
                write!(
                    f,
                    "orçamento de {limit} passos esgotado no nó {node} (laço infinito?)"
                )
            }
            VmError::PropertyFailed {
                node,
                name,
                write: true,
            } => write!(f, "não foi possível gravar '{name}' no nó {node}"),
            VmError::PropertyFailed {
                node,
                name,
                write: false,
            } => write!(f, "não foi possível ler '{name}' do nó {node}"),
            VmError::HostFailed { node, action } => write!(f, "{action} falhou no nó {node}"),
            VmError::DivisionByZero { node } => write!(f, "divisão por zero no nó {node}"),
            VmError::Suspended { node } => {
                write!(
                    f,
                    "o nó {node} pausa o programa: rode pelo Scheduler, não por Vm::run"
                )
            }
            VmError::Malformed { node, message } => {
                write!(f, "IR inválida no nó {node}: {message}")
            }
            VmError::InvalidValue { node, message } => {
                write!(f, "valor inválido no nó {node}: {message}")
            }
        }
    }
}

impl std::error::Error for VmError {}

/// Estado de uma pausa latente. Quando termina, a tarefa segue da op depois da que pausou.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Latente {
    /// Espera `restante` segundos.
    Espera { restante: f64 },
    /// Interpola propriedades do nó `no`: cada alvo é (nome, valor inicial, valor final).
    Tween {
        no: u64,
        alvos: Vec<(String, f64, f64)>,
        duracao: f64,
        decorrido: f64,
    },
    /// Espera o sinal `nome`. Quem acorda é o `Scheduler::emitir`.
    Sinal { nome: String },
}

/// Uma linha de execução de um programa: o ponteiro, os registradores e a pausa, se houver.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Tarefa {
    /// Índice do programa em `Compiled::programs`.
    pub programa: usize,
    pub pc: usize,
    pub regs: Vec<Value>,
    pub latente: Option<Latente>,
    /// Quando parou num ponto de parada, o próximo passo não pode parar de novo no mesmo lugar.
    pub ignora_ponto: bool,
    /// Criada por Spawn neste quadro: o tempo dela só começa a contar no quadro seguinte.
    pub nova: bool,
}

impl Tarefa {
    pub(crate) fn nova(programa: usize, program: &Program) -> Tarefa {
        Tarefa {
            programa,
            pc: 0,
            regs: vec![Value::Bool(false); program.regs],
            latente: None,
            ignora_ponto: false,
            nova: false,
        }
    }
}

/// Por que `avancar` parou.
pub(crate) enum Parada {
    /// O programa acabou.
    Fim,
    /// A tarefa pausou num latente.
    Latente(Latente),
    /// Parou antes de uma op, num ponto de parada ou no passo a passo.
    Pausa { node: u64 },
}

/// O que a tarefa precisa do mundo e o que ela deixa para o agendador.
pub(crate) struct Ambiente<'a> {
    pub host: &'a mut dyn Host,
    pub ctx: &'a mut Context,
    /// Nós com ponto de parada. Só valem na execução livre (`passo` = None).
    pub pontos: &'a BTreeSet<u64>,
    /// Passo a passo: quantas ops ainda podem rodar antes de parar. `None` é execução livre. Quando chega a zero,
    /// a próxima op pausa. Vale para a instância inteira, não para uma tarefa só.
    pub passo: Option<u32>,
    pub trace: Option<&'a mut Vec<TraceEvent>>,
    /// Tarefas criadas por Spawn (temporizadores), para o agendador.
    pub novas: Vec<Tarefa>,
    /// Sinais emitidos por EmitSignal, para o agendador.
    pub sinais: Vec<String>,
    /// Trocas de estado pedidas por GoToState, para o agendador.
    pub transicoes: Vec<String>,
}

/// A máquina virtual. Não guarda estado: tudo fica no `Context`, no `Host` e na `Tarefa`.
pub struct Vm;

impl Vm {
    /// Orçamento padrão de passos por execução. Cada op conta como um passo.
    pub const MAX_STEPS: u32 = 10_000;

    /// Executa o programa de um evento com o orçamento padrão. Erro se o programa tem um ponto latente.
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
        let vazio = BTreeSet::new();
        let mut tarefa = Tarefa::nova(0, program);
        let mut amb = Ambiente {
            host,
            ctx,
            pontos: &vazio,
            passo: None,
            trace: None,
            novas: Vec::new(),
            sinais: Vec::new(),
            transicoes: Vec::new(),
        };
        match avancar(&mut tarefa, program, &mut amb, limit)? {
            Parada::Fim => Ok(()),
            Parada::Latente(_) | Parada::Pausa { .. } => {
                Err(VmError::Suspended { node: program.node })
            }
        }
    }
}

/// Roda a tarefa a partir de `t.pc` até o fim, até uma pausa latente ou até um ponto de parada. Conta o
/// orçamento por chamada. Cada iteração executa uma op.
pub(crate) fn avancar(
    t: &mut Tarefa,
    program: &Program,
    amb: &mut Ambiente,
    limite: u32,
) -> Result<Parada, VmError> {
    let mut passos = 0u32;
    let mut executadas = 0u32;
    loop {
        if t.pc >= program.ops.len() {
            return Ok(Parada::Fim);
        }
        let origem = program.sources.get(t.pc).copied().unwrap_or(program.node);
        // Pausa antes da op: por passo esgotado, ou por ponto de parada na execução livre. Ao retomar de um
        // ponto, a primeira op roda sem parar de novo no mesmo lugar.
        let primeira = executadas == 0;
        if !(primeira && t.ignora_ponto) {
            let passo_esgotado = matches!(amb.passo, Some(0));
            let ponto = amb.passo.is_none() && amb.pontos.contains(&origem);
            if passo_esgotado || ponto {
                t.ignora_ponto = true;
                return Ok(Parada::Pausa { node: origem });
            }
        }
        passos += 1;
        if passos > limite {
            return Err(VmError::BudgetExhausted {
                limit: limite,
                node: origem,
            });
        }
        let atual = t.pc;
        t.pc += 1;
        executadas += 1;
        let op = &program.ops[atual];
        if let Some(tr) = amb.trace.as_mut() {
            tr.push(TraceEvent::Executou {
                evento: program.event.clone(),
                node: origem,
            });
        }
        let parada = executa(op, origem, t, amb)?;
        t.ignora_ponto = false;
        if let Some(restam) = amb.passo.as_mut() {
            *restam = restam.saturating_sub(1);
        }
        if let (Some(dst), Some(tr)) = (destino(op), amb.trace.as_mut()) {
            tr.push(TraceEvent::Valor {
                node: origem,
                valor: t.regs[dst].clone(),
            });
        }
        if let Some(p) = parada {
            return Ok(p);
        }
    }
}

/// Executa uma op. Devolve `Some` quando a op pausa a tarefa (latente).
fn executa(
    op: &Op,
    origem: u64,
    t: &mut Tarefa,
    amb: &mut Ambiente,
) -> Result<Option<Parada>, VmError> {
    let regs = &mut t.regs;
    match op {
        Op::Const { dst, value } => set(regs, *dst, value.clone(), origem)?,
        Op::Owner { dst } => set(regs, *dst, Value::Node(amb.ctx.owner), origem)?,
        Op::Delta { dst } => set(regs, *dst, Value::Float(amb.ctx.delta), origem)?,
        Op::GetVar { dst, var } => {
            let v = amb
                .ctx
                .vars
                .get(*var)
                .cloned()
                .ok_or_else(|| malformed(origem, format!("variável {var} não existe")))?;
            set(regs, *dst, v, origem)?;
        }
        Op::SetVar { var, src } => {
            let v = slot(regs, *src, origem)?.clone();
            let destino = amb
                .ctx
                .vars
                .get_mut(*var)
                .ok_or_else(|| malformed(origem, format!("variável {var} não existe")))?;
            *destino = v;
        }
        Op::Add { dst, a, b } => {
            let r = float(regs, *a, origem)? + float(regs, *b, origem)?;
            set(regs, *dst, Value::Float(r), origem)?;
        }
        Op::Sub { dst, a, b } => {
            let r = float(regs, *a, origem)? - float(regs, *b, origem)?;
            set(regs, *dst, Value::Float(r), origem)?;
        }
        Op::Mul { dst, a, b } => {
            let r = float(regs, *a, origem)? * float(regs, *b, origem)?;
            set(regs, *dst, Value::Float(r), origem)?;
        }
        Op::Div { dst, a, b } => {
            let divisor = float(regs, *b, origem)?;
            if divisor == 0.0 {
                return Err(VmError::DivisionByZero { node: origem });
            }
            let r = float(regs, *a, origem)? / divisor;
            set(regs, *dst, Value::Float(r), origem)?;
        }
        Op::Greater { dst, a, b } => {
            let r = float(regs, *a, origem)? > float(regs, *b, origem)?;
            set(regs, *dst, Value::Bool(r), origem)?;
        }
        Op::Less { dst, a, b } => {
            let r = float(regs, *a, origem)? < float(regs, *b, origem)?;
            set(regs, *dst, Value::Bool(r), origem)?;
        }
        Op::Equal { dst, a, b } => {
            let r = float(regs, *a, origem)? == float(regs, *b, origem)?;
            set(regs, *dst, Value::Bool(r), origem)?;
        }
        Op::IntAdd { dst, a, b } => {
            let r = int(regs, *a, origem)?.wrapping_add(int(regs, *b, origem)?);
            set(regs, *dst, Value::Int(r), origem)?;
        }
        Op::IntLess { dst, a, b } => {
            let r = int(regs, *a, origem)? < int(regs, *b, origem)?;
            set(regs, *dst, Value::Bool(r), origem)?;
        }
        Op::MakeVec { dst, x, y } => {
            let r = [float(regs, *x, origem)?, float(regs, *y, origem)?];
            set(regs, *dst, Value::Vec2(r), origem)?;
        }
        Op::AddVec { dst, a, b } => {
            let (va, vb) = (vec2(regs, *a, origem)?, vec2(regs, *b, origem)?);
            set(
                regs,
                *dst,
                Value::Vec2([va[0] + vb[0], va[1] + vb[1]]),
                origem,
            )?;
        }
        Op::ScaleVec { dst, vector, scale } => {
            let v = vec2(regs, *vector, origem)?;
            let k = float(regs, *scale, origem)?;
            set(regs, *dst, Value::Vec2([v[0] * k, v[1] * k]), origem)?;
        }
        Op::VecLength { dst, vector } => {
            let v = vec2(regs, *vector, origem)?;
            set(regs, *dst, Value::Float(v[0].hypot(v[1])), origem)?;
        }
        Op::Concat { dst, a, b } => {
            let r = format!("{}{}", text(regs, *a, origem)?, text(regs, *b, origem)?);
            set(regs, *dst, Value::String(r), origem)?;
        }
        Op::Log { level, msg } => {
            let msg = text(regs, *msg, origem)?;
            amb.host.log(*level, msg);
        }
        Op::ActionPressed { dst, action } => {
            let pressed = amb.host.action_pressed(text(regs, *action, origem)?);
            set(regs, *dst, Value::Bool(pressed), origem)?;
        }
        Op::ActionJustPressed { dst, action } => {
            let pressed = amb.host.action_just_pressed(text(regs, *action, origem)?);
            set(regs, *dst, Value::Bool(pressed), origem)?;
        }
        Op::ActionReleased { dst, action } => {
            let released = amb.host.action_released(text(regs, *action, origem)?);
            set(regs, *dst, Value::Bool(released), origem)?;
        }
        Op::GetAxis {
            dst,
            negative,
            positive,
        } => {
            let eixo = amb.host.axis(
                text(regs, *negative, origem)?,
                text(regs, *positive, origem)?,
            );
            set(regs, *dst, Value::Float(eixo), origem)?;
        }
        Op::GetVector {
            dst,
            left,
            right,
            up,
            down,
        } => {
            let v = amb.host.vector(
                text(regs, *left, origem)?,
                text(regs, *right, origem)?,
                text(regs, *up, origem)?,
                text(regs, *down, origem)?,
            );
            set(regs, *dst, Value::Vec2(v), origem)?;
        }
        Op::MouseWorld { dst } => {
            let p = amb.host.mouse_world();
            set(regs, *dst, Value::Vec2(p), origem)?;
        }
        Op::MouseScreen { dst } => {
            let p = amb.host.mouse_screen();
            set(regs, *dst, Value::Vec2(p), origem)?;
        }
        Op::GetProp { dst, node, name } => {
            let alvo = node_id(regs, *node, origem)?;
            let nome = text(regs, *name, origem)?.to_string();
            let v = amb
                .host
                .get_property(alvo, &nome)
                .ok_or_else(|| VmError::PropertyFailed {
                    node: origem,
                    name: nome.clone(),
                    write: false,
                })?;
            set(regs, *dst, Value::Float(v), origem)?;
        }
        Op::SetProp { node, name, value } => {
            let alvo = node_id(regs, *node, origem)?;
            let nome = text(regs, *name, origem)?.to_string();
            let v = float(regs, *value, origem)?;
            if !amb.host.set_property(alvo, &nome, v) {
                return Err(VmError::PropertyFailed {
                    node: origem,
                    name: nome,
                    write: true,
                });
            }
        }
        Op::FindNode { dst, name } => {
            // Sem nó, a referência vazia (id 0) é o resultado. O id 0 nunca é de um nó real.
            let achado = amb.host.find_node(text(regs, *name, origem)?).unwrap_or(0);
            set(regs, *dst, Value::Node(achado), origem)?;
        }
        Op::InstanceScene {
            dst,
            scene,
            position,
        } => {
            let cena = text(regs, *scene, origem)?.to_string();
            let pos = vec2(regs, *position, origem)?;
            let criado =
                amb.host
                    .instance_scene(&cena, pos)
                    .ok_or_else(|| VmError::HostFailed {
                        node: origem,
                        action: format!("instanciar a cena '{cena}'"),
                    })?;
            set(regs, *dst, Value::Node(criado), origem)?;
        }
        Op::Destroy { node } => amb.host.destroy(node_id(regs, *node, origem)?),
        Op::MoveAndSlide {
            dst,
            node,
            velocity,
        } => {
            let alvo = node_id(regs, *node, origem)?;
            let v = vec2(regs, *velocity, origem)?;
            let no_chao = amb
                .host
                .move_and_slide(alvo, v)
                .ok_or_else(|| VmError::HostFailed {
                    node: origem,
                    action: "mover e deslizar".into(),
                })?;
            set(regs, *dst, Value::Bool(no_chao), origem)?;
        }
        Op::Audio(comando) => {
            let cmd = match comando {
                AudioOp::Play {
                    node,
                    stream,
                    volume_db,
                    pitch,
                    bus,
                    looping,
                    from_start,
                } => AudioCommand::Play {
                    node: Uid(node_id(regs, *node, origem)?),
                    stream: uid_de_asset(text(regs, *stream, origem)?, origem)?,
                    volume_db: float(regs, *volume_db, origem)?,
                    pitch: float(regs, *pitch, origem)?,
                    bus: text(regs, *bus, origem)?.to_string(),
                    looping: boolean(regs, *looping, origem)?,
                    from_start: boolean(regs, *from_start, origem)?,
                },
                AudioOp::Stop { node } => AudioCommand::Stop {
                    node: Uid(node_id(regs, *node, origem)?),
                },
                AudioOp::Fade {
                    node,
                    to_db,
                    seconds,
                } => AudioCommand::Fade {
                    node: Uid(node_id(regs, *node, origem)?),
                    to_db: float(regs, *to_db, origem)?,
                    seconds: float(regs, *seconds, origem)?,
                },
                AudioOp::SetBus { bus, volume_db } => AudioCommand::SetBusVolume {
                    bus: text(regs, *bus, origem)?.to_string(),
                    volume_db: float(regs, *volume_db, origem)?,
                },
            };
            amb.host.audio(cmd);
        }
        Op::Jump { target } => t.pc = *target,
        Op::JumpIfFalse { cond, target } => {
            if !boolean(regs, *cond, origem)? {
                t.pc = *target;
            }
        }
        Op::Wait { seconds } => {
            let s = float(regs, *seconds, origem)?;
            if s > 0.0 {
                return Ok(Some(Parada::Latente(Latente::Espera { restante: s })));
            }
        }
        Op::Tween {
            node,
            name,
            to,
            seconds,
        } => {
            let alvo = node_id(regs, *node, origem)?;
            let nome = text(regs, *name, origem)?.to_string();
            let fim = float(regs, *to, origem)?;
            let duracao = float(regs, *seconds, origem)?;
            return inicia_tween(amb.host, origem, alvo, vec![(nome, fim)], duracao);
        }
        Op::MoveTo {
            node,
            target,
            seconds,
        } => {
            let alvo = node_id(regs, *node, origem)?;
            let p = vec2(regs, *target, origem)?;
            let duracao = float(regs, *seconds, origem)?;
            let alvos = vec![
                ("position_x".to_string(), p[0]),
                ("position_y".to_string(), p[1]),
            ];
            return inicia_tween(amb.host, origem, alvo, alvos, duracao);
        }
        Op::AwaitSignal { name } => {
            let nome = text(regs, *name, origem)?.to_string();
            return Ok(Some(Parada::Latente(Latente::Sinal { nome })));
        }
        Op::GoToState { name } => {
            let nome = text(regs, *name, origem)?.to_string();
            amb.transicoes.push(nome);
        }
        Op::EmitSignal { name } => {
            let nome = text(regs, *name, origem)?.to_string();
            amb.sinais.push(nome);
        }
        Op::Spawn { seconds, entry } => {
            let s = float(regs, *seconds, origem)?;
            let mut filha = Tarefa {
                programa: t.programa,
                pc: *entry,
                regs: regs.to_vec(),
                latente: None,
                ignora_ponto: false,
                nova: false,
            };
            if s > 0.0 {
                filha.latente = Some(Latente::Espera { restante: s });
                filha.nova = true;
            }
            amb.novas.push(filha);
        }
    }
    Ok(None)
}

/// Começa uma interpolação: lê os valores iniciais e, se a duração é positiva, pausa a tarefa. Com duração zero
/// grava o valor final na hora e segue.
fn inicia_tween(
    host: &mut dyn Host,
    origem: u64,
    no: u64,
    alvos: Vec<(String, f64)>,
    duracao: f64,
) -> Result<Option<Parada>, VmError> {
    let mut prontos = Vec::with_capacity(alvos.len());
    for (nome, fim) in alvos {
        let inicio = host
            .get_property(no, &nome)
            .ok_or_else(|| VmError::PropertyFailed {
                node: origem,
                name: nome.clone(),
                write: false,
            })?;
        prontos.push((nome, inicio, fim));
    }
    if duracao <= 0.0 {
        for (nome, _, fim) in &prontos {
            if !host.set_property(no, nome, *fim) {
                return Err(VmError::PropertyFailed {
                    node: origem,
                    name: nome.clone(),
                    write: true,
                });
            }
        }
        return Ok(None);
    }
    Ok(Some(Parada::Latente(Latente::Tween {
        no,
        alvos: prontos,
        duracao,
        decorrido: 0.0,
    })))
}

/// Avança um latente em `dt` segundos. Devolve `true` quando ele termina e a tarefa pode seguir.
pub(crate) fn progride(
    l: &mut Latente,
    dt: f64,
    host: &mut dyn Host,
    origem: u64,
) -> Result<bool, VmError> {
    match l {
        Latente::Espera { restante } => {
            *restante -= dt;
            Ok(*restante <= 1e-9)
        }
        // Acorda pelo sinal, não pelo tempo.
        Latente::Sinal { .. } => Ok(false),
        Latente::Tween {
            no,
            alvos,
            duracao,
            decorrido,
        } => {
            *decorrido += dt;
            let t = (*decorrido / *duracao).min(1.0);
            for (nome, inicio, fim) in alvos.iter() {
                let v = *inicio + (*fim - *inicio) * t;
                if !host.set_property(*no, nome, v) {
                    return Err(VmError::PropertyFailed {
                        node: origem,
                        name: nome.clone(),
                        write: true,
                    });
                }
            }
            Ok(t >= 1.0)
        }
    }
}

/// Registrador que a op grava, se ela grava um.
fn destino(op: &Op) -> Option<Reg> {
    match op {
        Op::Const { dst, .. }
        | Op::Owner { dst }
        | Op::Delta { dst }
        | Op::GetVar { dst, .. }
        | Op::Add { dst, .. }
        | Op::Sub { dst, .. }
        | Op::Mul { dst, .. }
        | Op::Div { dst, .. }
        | Op::Greater { dst, .. }
        | Op::Less { dst, .. }
        | Op::Equal { dst, .. }
        | Op::IntAdd { dst, .. }
        | Op::IntLess { dst, .. }
        | Op::MakeVec { dst, .. }
        | Op::AddVec { dst, .. }
        | Op::ScaleVec { dst, .. }
        | Op::VecLength { dst, .. }
        | Op::Concat { dst, .. }
        | Op::ActionPressed { dst, .. }
        | Op::ActionJustPressed { dst, .. }
        | Op::ActionReleased { dst, .. }
        | Op::GetAxis { dst, .. }
        | Op::GetVector { dst, .. }
        | Op::MouseWorld { dst }
        | Op::MouseScreen { dst }
        | Op::GetProp { dst, .. }
        | Op::FindNode { dst, .. }
        | Op::InstanceScene { dst, .. }
        | Op::MoveAndSlide { dst, .. } => Some(*dst),
        _ => None,
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
            format!(
                "registrador {i} deveria ser decimal, é {}",
                other.ty().nome()
            ),
        )),
    }
}

fn int(regs: &[Value], i: Reg, node: u64) -> Result<i64, VmError> {
    match slot(regs, i, node)? {
        Value::Int(n) => Ok(*n),
        other => Err(malformed(
            node,
            format!(
                "registrador {i} deveria ser inteiro, é {}",
                other.ty().nome()
            ),
        )),
    }
}

fn boolean(regs: &[Value], i: Reg, node: u64) -> Result<bool, VmError> {
    match slot(regs, i, node)? {
        Value::Bool(b) => Ok(*b),
        other => Err(malformed(
            node,
            format!(
                "registrador {i} deveria ser booleano, é {}",
                other.ty().nome()
            ),
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

fn vec2(regs: &[Value], i: Reg, node: u64) -> Result<[f64; 2], VmError> {
    match slot(regs, i, node)? {
        Value::Vec2(v) => Ok(*v),
        other => Err(malformed(
            node,
            format!(
                "registrador {i} deveria ser vetor 2D, é {}",
                other.ty().nome()
            ),
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

/// UID de um asset de áudio, como texto de 16 dígitos hexadecimais (o formato do contrato do núcleo). Texto que
/// não é UID é valor inválido: o pino foi preenchido errado no jogo, não a IR.
fn uid_de_asset(s: &str, node: u64) -> Result<Uid, VmError> {
    s.parse::<Uid>().map_err(|_| VmError::InvalidValue {
        node,
        message: format!("'{s}' não é o UID de um asset de áudio"),
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::graph::Graph;
    use crate::ir::{Op, Program, compile};
    use crate::types::{PinType, Value};
    use std::collections::HashMap;

    /// Host de teste: propriedades num mapa, entradas simuladas e o registro das chamadas.
    #[derive(Default)]
    pub(crate) struct Mock {
        pub props: HashMap<(u64, String), f64>,
        pub pressionadas: Vec<String>,
        pub impressos: Vec<(LogLevel, String)>,
        pub destruidos: Vec<u64>,
        pub audio: Vec<AudioCommand>,
        pub nos: HashMap<String, u64>,
        pub mouse: [f64; 2],
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
        fn action_just_pressed(&self, action: &str) -> bool {
            self.action_pressed(action)
        }
        fn action_released(&self, action: &str) -> bool {
            !self.action_pressed(action)
        }
        fn axis(&self, negative: &str, positive: &str) -> f64 {
            match (self.action_pressed(negative), self.action_pressed(positive)) {
                (true, false) => -1.0,
                (false, true) => 1.0,
                _ => 0.0,
            }
        }
        fn vector(&self, left: &str, right: &str, up: &str, down: &str) -> [f64; 2] {
            [self.axis(left, right), self.axis(up, down)]
        }
        fn mouse_world(&self) -> [f64; 2] {
            self.mouse
        }
        fn mouse_screen(&self) -> [f64; 2] {
            self.mouse
        }
        fn log(&mut self, level: LogLevel, message: &str) {
            self.impressos.push((level, message.to_string()));
        }
        fn find_node(&self, name: &str) -> Option<u64> {
            self.nos.get(name).copied()
        }
        fn instance_scene(&mut self, scene: &str, _position: [f64; 2]) -> Option<u64> {
            if scene == "bala.scene" {
                Some(42)
            } else {
                None
            }
        }
        fn destroy(&mut self, node: u64) {
            self.destruidos.push(node);
        }
        fn move_and_slide(&mut self, node: u64, velocity: [f64; 2]) -> Option<bool> {
            let x = self.props.get_mut(&(node, "position_x".to_string()))?;
            *x += velocity[0];
            Some(velocity[1] >= 0.0)
        }
        fn audio(&mut self, command: AudioCommand) {
            self.audio.push(command);
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
    pub(crate) fn grafo_de_movimento() -> Graph {
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
            assert_eq!(host.impressos, [(LogLevel::Info, esperado.to_string())]);
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
        assert_eq!(host.impressos, [(LogLevel::Info, "primeiro".to_string())]);
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
        assert!(
            matches!(erro, VmError::PropertyFailed { write: false, .. }),
            "{erro:?}"
        );
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
            estado: None,
        };
        let mut ctx = Context {
            owner: 5,
            delta: 0.0,
            vars: Vec::new(),
            estado: String::new(),
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

    #[test]
    fn divisao_por_zero_vira_erro() {
        // OnReady -> SetProperty(self, "x", 1 / 0). O divisor zero tem de virar erro, não infinito.
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let dv = adiciona(&mut g, "Divide");
        g.set_value(dv, "a", Value::Float(1.0)).unwrap();
        g.set_value(dv, "b", Value::Float(0.0)).unwrap();
        let sp = adiciona(&mut g, "SetProperty");
        texto(&mut g, sp, "name", "x");
        g.connect((ev, "exec"), (sp, "exec")).unwrap();
        g.connect((ev, "self"), (sp, "node")).unwrap();
        g.connect((dv, "result"), (sp, "value")).unwrap();
        let compiled = compile(&g).unwrap();
        let program = compiled.programs_for("OnReady").next().unwrap();
        let mut host = Mock::default();
        host.props.insert((3, "x".to_string()), 0.0);
        let erro = Vm::run(program, &mut host, &mut compiled.new_context(3)).unwrap_err();
        assert_eq!(erro, VmError::DivisionByZero { node: dv });
    }

    /// OnReady -> Tocar -> Parar -> Fade -> Volume do barramento. O nó e o UID vêm dos pinos; os volumes em dB.
    fn grafo_de_audio(stream: &str) -> Graph {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let pl = adiciona(&mut g, "PlaySound");
        g.set_value(pl, "node", Value::Node(7)).unwrap();
        texto(&mut g, pl, "stream", stream);
        g.set_value(pl, "volume", Value::Float(-6.0)).unwrap();
        let para = adiciona(&mut g, "StopSound");
        g.set_value(para, "node", Value::Node(7)).unwrap();
        let fade = adiciona(&mut g, "FadeSound");
        g.set_value(fade, "node", Value::Node(9)).unwrap();
        g.set_value(fade, "volume", Value::Float(-40.0)).unwrap();
        g.set_value(fade, "seconds", Value::Float(0.5)).unwrap();
        let bus = adiciona(&mut g, "SetBusVolume");
        texto(&mut g, bus, "bus", "Musica");
        g.set_value(bus, "volume", Value::Float(-3.0)).unwrap();
        g.connect((ev, "exec"), (pl, "exec")).unwrap();
        g.connect((pl, "exec"), (para, "exec")).unwrap();
        g.connect((para, "exec"), (fade, "exec")).unwrap();
        g.connect((fade, "exec"), (bus, "exec")).unwrap();
        g
    }

    #[test]
    fn comandos_de_audio_chegam_ao_host_no_contrato_do_nucleo() {
        let compiled = compile(&grafo_de_audio("00000000000000ab")).unwrap();
        let program = compiled.programs_for("OnReady").next().unwrap();
        let mut host = Mock::default();
        Vm::run(program, &mut host, &mut compiled.new_context(1)).unwrap();
        assert_eq!(
            host.audio,
            [
                AudioCommand::Play {
                    node: Uid(7),
                    stream: Uid(0xab),
                    volume_db: -6.0,
                    pitch: 1.0,
                    bus: "Master".into(),
                    looping: false,
                    from_start: true,
                },
                AudioCommand::Stop { node: Uid(7) },
                AudioCommand::Fade {
                    node: Uid(9),
                    to_db: -40.0,
                    seconds: 0.5,
                },
                AudioCommand::SetBusVolume {
                    bus: "Musica".into(),
                    volume_db: -3.0,
                },
            ]
        );
    }

    #[test]
    fn uid_de_asset_invalido_vira_erro_de_valor() {
        let compiled = compile(&grafo_de_audio("tiro.wav")).unwrap();
        let program = compiled.programs_for("OnReady").next().unwrap();
        let erro = Vm::run(program, &mut Mock::default(), &mut compiled.new_context(1)).unwrap_err();
        assert!(
            matches!(erro, VmError::InvalidValue { .. }),
            "{erro:?}"
        );
        assert!(erro.to_string().contains("tiro.wav"), "{erro}");
    }

    #[test]
    fn tocar_sem_asset_e_recusado_na_compilacao() {
        let mut g = grafo_de_audio("00000000000000ab");
        let pl = g.nodes.iter().find(|n| n.kind == "PlaySound").unwrap().id;
        // Sem o valor embutido e sem fio, o pino de UID é obrigatório.
        g.nodes
            .iter_mut()
            .find(|n| n.id == pl)
            .unwrap()
            .values
            .remove("stream");
        let erros = compile(&g).unwrap_err();
        assert!(
            erros
                .iter()
                .any(|e| e.node == Some(pl) && e.message.contains("obrigatória")),
            "{erros:?}"
        );
    }
}
