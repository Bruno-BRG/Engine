//! Agendador de uma instância de script (VIS-025, VIS-026, VIS-023, VIS-030, VIS-031, DBG-003). Guarda as tarefas
//! (linhas de execução) de uma instância, avança o tempo com `tick(dt)` e entrega os sinais. Também guarda os
//! pontos de parada, o passo a passo e o rastro para o editor. Cada instância tem o seu agendador: a escolha de
//! instância no depurador é só qual agendador (e qual contexto) o editor mostra.

use crate::ir::Compiled;
use crate::vm::{
    Ambiente, Context, Host, Latente, Parada, Tarefa, TraceEvent, Vm, VmError, avancar, progride,
};
use std::collections::BTreeSet;

/// Onde a execução parou para depuração: o nó da op que vai rodar e o evento do programa.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pausa {
    pub node: u64,
    pub evento: String,
}

/// Tarefas de uma instância, com a pausa de depuração, os sinais pendentes e o rastro.
#[derive(Debug, Default)]
pub struct Scheduler {
    tarefas: Vec<Tarefa>,
    /// Sinais emitidos e ainda não entregues. Entregues no começo do próximo `tick`.
    sinais: Vec<String>,
    pausa: Option<Pausa>,
    /// Índice da tarefa onde o quadro parou por pausa. O resto do quadro continua dali.
    inicio: usize,
    /// Tempo do quadro que parou por pausa, ainda não aplicado aos latentes das tarefas seguintes.
    dt_pendente: f64,
    pontos: BTreeSet<u64>,
    rastrear: bool,
    rastro: Vec<TraceEvent>,
}

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler::default()
    }

    /// Cria uma tarefa para cada programa do evento `evento` ("OnReady", "OnProcess"...). Chame a cada quadro
    /// para os eventos de quadro; os de início, uma vez.
    pub fn iniciar(&mut self, compiled: &Compiled, evento: &str) {
        for (i, programa) in compiled.programs.iter().enumerate() {
            if programa.event == evento {
                self.tarefas.push(Tarefa::nova(i, programa));
            }
        }
    }

    /// Quantidade de tarefas vivas (rodando ou esperando).
    pub fn tarefas(&self) -> usize {
        self.tarefas.len()
    }

    /// Pausa atual, se a execução parou para depuração.
    pub fn pausa(&self) -> Option<&Pausa> {
        self.pausa.as_ref()
    }

    /// Liga ou desliga o ponto de parada do nó. Vale na execução livre (`tick` e `continuar`).
    pub fn definir_ponto_de_parada(&mut self, node: u64, ligado: bool) {
        if ligado {
            self.pontos.insert(node);
        } else {
            self.pontos.remove(&node);
        }
    }

    /// Replaces all breakpoints with the given set (VIS-031).
    pub fn definir_pontos_de_parada(&mut self, pontos: BTreeSet<u64>) {
        self.pontos = pontos;
    }

    pub fn pontos_de_parada(&self) -> &BTreeSet<u64> {
        &self.pontos
    }

    /// Liga ou desliga o rastro (VIS-030). Com o rastro ligado, cada op gera um evento.
    pub fn definir_rastro(&mut self, ligado: bool) {
        self.rastrear = ligado;
    }

    /// Entrega o rastro acumulado e esvazia a fila.
    pub fn tomar_rastro(&mut self) -> Vec<TraceEvent> {
        std::mem::take(&mut self.rastro)
    }

    /// Avança a instância `dt` segundos: entrega os sinais pendentes, progride os latentes e roda as tarefas até
    /// elas pausarem. Enquanto houver pausa de depuração, não faz nada: use `continuar` ou `passo`.
    pub fn tick(
        &mut self,
        compiled: &Compiled,
        ctx: &mut Context,
        host: &mut dyn Host,
        dt: f64,
    ) -> Result<(), VmError> {
        if self.pausa.is_some() {
            return Ok(());
        }
        self.abrir_quadro(ctx, dt);
        self.rodar(compiled, ctx, host, None)
    }

    /// Sai da pausa e roda até a próxima pausa ou até o fim do quadro. Sem pausa, não faz nada.
    pub fn continuar(
        &mut self,
        compiled: &Compiled,
        ctx: &mut Context,
        host: &mut dyn Host,
    ) -> Result<(), VmError> {
        if self.pausa.is_none() {
            return Ok(());
        }
        self.rodar(compiled, ctx, host, None)
    }

    /// Roda uma op e pausa de novo na op seguinte (VIS-031, passo a passo). Pode atravessar tarefas: a pausa cai
    /// na próxima op da instância, qualquer que seja a tarefa.
    /// Without a pause, it does nothing.
    pub fn passo(
        &mut self,
        compiled: &Compiled,
        ctx: &mut Context,
        host: &mut dyn Host,
    ) -> Result<(), VmError> {
        if self.pausa.is_none() {
            return Ok(());
        }
        self.rodar(compiled, ctx, host, Some(1))
    }

    /// Starts a frame with time `dt` and pauses before the first operation of the tasks created with `iniciar`, without
    /// running any. Step-by-step uses it to stop at the start of an event (VIS-031). Does nothing while a pause is active.
    pub fn parar_antes(
        &mut self,
        compiled: &Compiled,
        ctx: &mut Context,
        host: &mut dyn Host,
        dt: f64,
    ) -> Result<(), VmError> {
        if self.pausa.is_some() {
            return Ok(());
        }
        self.abrir_quadro(ctx, dt);
        self.rodar(compiled, ctx, host, Some(0))
    }

    /// Marks the start of a frame with time `dt`: the context gets the `delta`, the pending signals are delivered, and
    /// the tasks run from the start of the queue.
    fn abrir_quadro(&mut self, ctx: &mut Context, dt: f64) {
        ctx.delta = dt;
        self.acordar_sinais();
        self.inicio = 0;
        self.dt_pendente = dt;
    }

    /// Troca o script por uma recompilação (VIS-023, recarga a quente). As variáveis de mesmo nome e mesmo tipo
    /// mantêm o valor; as outras começam no padrão do grafo novo. As tarefas em curso são descartadas, porque as
    /// posições delas no programa antigo deixam de valer. Devolve o contexto para o novo programa.
    pub fn recarregar(&mut self, ctx: &Context, antigo: &Compiled, novo: &Compiled) -> Context {
        self.tarefas.clear();
        self.sinais.clear();
        self.pausa = None;
        self.inicio = 0;
        self.dt_pendente = 0.0;
        recarregar_contexto(ctx, antigo, novo)
    }

    /// Troca o estado ativo: sai o estado antigo (StateExit) e entra o novo (StateEnter). As tarefas dos dois
    /// eventos entram na fila do quadro corrente.
    fn transitar(&mut self, compiled: &Compiled, ctx: &mut Context, novo: String) {
        let antigo = std::mem::replace(&mut ctx.estado, novo.clone());
        self.iniciar_evento(compiled, "StateExit", &antigo);
        self.iniciar_evento(compiled, "StateEnter", &novo);
    }

    fn iniciar_evento(&mut self, compiled: &Compiled, evento: &str, estado: &str) {
        for (i, programa) in compiled.programs.iter().enumerate() {
            if programa.event == evento && programa.estado.as_deref() == Some(estado) {
                self.tarefas.push(Tarefa::nova(i, programa));
            }
        }
    }

    /// Cria uma tarefa para cada nó "State" do estado dado. Chame a cada quadro, com o estado ativo do contexto
    /// (`ctx.estado`), antes do `tick`.
    pub fn iniciar_estado(&mut self, compiled: &Compiled, estado: &str) {
        self.iniciar_evento(compiled, "State", estado);
    }

    fn acordar_sinais(&mut self) {
        let emitidos = std::mem::take(&mut self.sinais);
        for t in &mut self.tarefas {
            if let Some(Latente::Sinal { nome }) = &t.latente
                && emitidos.iter().any(|s| s == nome)
            {
                t.latente = None;
            }
        }
    }

    /// Roda as tarefas a partir de `self.inicio`. Com `passo` = Some(n), roda no máximo n ops antes de pausar.
    fn rodar(
        &mut self,
        compiled: &Compiled,
        ctx: &mut Context,
        host: &mut dyn Host,
        passo: Option<u32>,
    ) -> Result<(), VmError> {
        self.pausa = None;
        let dt = self.dt_pendente;
        let mut passo = passo;
        let mut i = self.inicio;
        while i < self.tarefas.len() {
            // Latente: avança pelo tempo do quadro. Se ainda não terminou, a tarefa segue suspensa.
            if self.tarefas[i].latente.is_some() {
                if self.tarefas[i].nova {
                    // Criada por Spawn neste quadro: começa a contar no próximo.
                    self.tarefas[i].nova = false;
                    i += 1;
                    continue;
                }
                let origem = compiled.programs[self.tarefas[i].programa].node;
                let progresso = match self.tarefas[i].latente.as_mut() {
                    Some(l) if dt > 0.0 => progride(l, dt, host, origem),
                    _ => Ok(false),
                };
                let terminou = match progresso {
                    Ok(t) => t,
                    Err(e) => {
                        self.tarefas.remove(i);
                        self.inicio = 0;
                        self.dt_pendente = 0.0;
                        return Err(e);
                    }
                };
                if !terminou {
                    i += 1;
                    continue;
                }
                self.tarefas[i].latente = None;
            }

            let programa = &compiled.programs[self.tarefas[i].programa];
            let (resultado, novas, sinais, transicoes, restante) = {
                let mut amb = Ambiente {
                    host: &mut *host,
                    ctx: &mut *ctx,
                    pontos: &self.pontos,
                    passo,
                    trace: if self.rastrear {
                        Some(&mut self.rastro)
                    } else {
                        None
                    },
                    novas: Vec::new(),
                    sinais: Vec::new(),
                    transicoes: Vec::new(),
                };
                let r = avancar(&mut self.tarefas[i], programa, &mut amb, Vm::MAX_STEPS);
                (r, amb.novas, amb.sinais, amb.transicoes, amb.passo)
            };
            passo = restante;
            self.tarefas.extend(novas);
            self.sinais.extend(sinais);
            for estado in transicoes {
                self.transitar(compiled, ctx, estado);
            }
            match resultado {
                Err(e) => {
                    self.tarefas.remove(i);
                    self.inicio = 0;
                    self.dt_pendente = 0.0;
                    return Err(e);
                }
                Ok(Parada::Fim) => {
                    self.tarefas.remove(i);
                }
                Ok(Parada::Latente(l)) => {
                    self.tarefas[i].latente = Some(l);
                    i += 1;
                }
                Ok(Parada::Pausa { node }) => {
                    self.pausa = Some(Pausa {
                        node,
                        evento: programa.event.clone(),
                    });
                    self.inicio = i;
                    return Ok(());
                }
            }
        }
        // O quadro acabou: nada mais a retomar.
        self.inicio = 0;
        self.dt_pendente = 0.0;
        Ok(())
    }
}

/// Contexto de uma instância depois da recarga: variáveis por nome e tipo, com o padrão do grafo novo para as
/// que mudaram ou são novas.
pub fn recarregar_contexto(ctx: &Context, antigo: &Compiled, novo: &Compiled) -> Context {
    let vars = novo
        .variables
        .iter()
        .map(|nova| {
            antigo
                .variables
                .iter()
                .position(|v| v.name == nova.name && v.ty == nova.ty)
                .and_then(|i| ctx.vars.get(i).cloned())
                .unwrap_or_else(|| nova.default.clone())
        })
        .collect();
    Context {
        owner: ctx.owner,
        delta: ctx.delta,
        vars,
        estado: ctx.estado.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Graph;
    use nexo_core::{AudioCommand, Uid};
    use crate::ir::compile;
    use crate::types::{LogLevel, PinType, Value};
    use crate::vm::tests::{Mock, grafo_de_movimento};

    fn adiciona(g: &mut Graph, kind: &str) -> u64 {
        g.add_node(kind, [0.0, 0.0]).unwrap()
    }

    fn texto(g: &mut Graph, id: u64, pin: &str, valor: &str) {
        g.set_value(id, pin, Value::String(valor.into())).unwrap();
    }

    fn imprime(g: &mut Graph, antes: (u64, &str), msg: &str) -> u64 {
        let p = adiciona(g, "Print");
        texto(g, p, "msg", msg);
        g.connect(antes, (p, "exec")).unwrap();
        p
    }

    /// Script de plataforma: no OnProcess, com "ui_right" pressionada, move o nó X e incrementa a variável
    /// "passos". O movimento vem de `grafo_de_movimento`; aqui acrescentamos o contador.
    fn plataforma() -> Graph {
        let mut g = grafo_de_movimento();
        g.add_variable("passos", PinType::Float, Value::Float(0.0), true)
            .unwrap();
        // O último SetProperty do movimento é o que vem depois do Branch; o contador entra depois dele.
        let sp = g.nodes.iter().find(|n| n.kind == "SetProperty").unwrap().id;
        let le = adiciona(&mut g, "GetVariable");
        texto(&mut g, le, "name", "passos");
        let soma = adiciona(&mut g, "Add");
        g.set_value(soma, "b", Value::Float(1.0)).unwrap();
        let grava = adiciona(&mut g, "SetVariable");
        texto(&mut g, grava, "name", "passos");
        g.connect((sp, "exec"), (grava, "exec")).unwrap();
        g.connect((le, "value"), (soma, "a")).unwrap();
        g.connect((soma, "result"), (grava, "value")).unwrap();
        g
    }

    #[test]
    fn script_de_plataforma_le_acao_move_no_e_incrementa_variavel() {
        let compiled = compile(&plataforma()).unwrap();
        let mut ctx = compiled.new_context(7);
        let mut host = Mock::default();
        host.props.insert((7, "position_x".to_string()), 10.0);
        let mut sched = Scheduler::new();

        // Sem ação: nada se mexe.
        sched.iniciar(&compiled, "OnProcess");
        sched.tick(&compiled, &mut ctx, &mut host, 0.5).unwrap();
        assert_eq!(host.props[&(7, "position_x".to_string())], 10.0);
        assert_eq!(ctx.vars[0], Value::Float(0.0));

        // Com ação: move 0.5 * 100 e incrementa.
        host.pressionadas.push("ui_right".into());
        sched.iniciar(&compiled, "OnProcess");
        sched.tick(&compiled, &mut ctx, &mut host, 0.5).unwrap();
        assert_eq!(host.props[&(7, "position_x".to_string())], 60.0);
        assert_eq!(ctx.vars[0], Value::Float(1.0));
    }

    #[test]
    fn while_repete_ate_a_condicao_falsar() {
        // n = 0; enquanto n < 3: n = n + 1. Ao fim, n vale 3.
        let mut g = Graph::default();
        g.add_variable("n", PinType::Float, Value::Float(0.0), false)
            .unwrap();
        let ev = adiciona(&mut g, "OnReady");
        let w = adiciona(&mut g, "While");
        let menor = adiciona(&mut g, "Less");
        g.set_value(menor, "b", Value::Float(3.0)).unwrap();
        let le = adiciona(&mut g, "GetVariable");
        texto(&mut g, le, "name", "n");
        let soma = adiciona(&mut g, "Add");
        g.set_value(soma, "b", Value::Float(1.0)).unwrap();
        let grava = adiciona(&mut g, "SetVariable");
        texto(&mut g, grava, "name", "n");
        g.connect((ev, "exec"), (w, "exec")).unwrap();
        g.connect((le, "value"), (menor, "a")).unwrap();
        g.connect((menor, "result"), (w, "condition")).unwrap();
        g.connect((w, "body"), (grava, "exec")).unwrap();
        g.connect((le, "value"), (soma, "a")).unwrap();
        g.connect((soma, "result"), (grava, "value")).unwrap();
        let compiled = compile(&g).unwrap();
        let mut ctx = compiled.new_context(1);
        let mut host = Mock::default();
        let mut sched = Scheduler::new();
        sched.iniciar(&compiled, "OnReady");
        sched.tick(&compiled, &mut ctx, &mut host, 0.0).unwrap();
        assert_eq!(ctx.vars[0], Value::Float(3.0));
        assert_eq!(sched.tarefas(), 0);
    }

    #[test]
    fn while_sem_saida_esgota_o_orcamento() {
        // Condição sempre verdadeira: o laço nunca termina e o orçamento para a tarefa com erro.
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let w = adiciona(&mut g, "While");
        g.set_value(w, "condition", Value::Bool(true)).unwrap();
        g.connect((ev, "exec"), (w, "exec")).unwrap();
        let compiled = compile(&g).unwrap();
        let mut sched = Scheduler::new();
        sched.iniciar(&compiled, "OnReady");
        let erro = sched
            .tick(
                &compiled,
                &mut compiled.new_context(1),
                &mut Mock::default(),
                0.0,
            )
            .unwrap_err();
        assert_eq!(
            erro,
            VmError::BudgetExhausted {
                limit: Vm::MAX_STEPS,
                node: w
            }
        );
        assert_eq!(sched.tarefas(), 0, "a tarefa com erro é descartada");
    }

    #[test]
    fn espera_retoma_depois_de_n_ticks() {
        // OnReady: Wait(0.3) e depois Print("depois"). Com ticks de 0.1 s, a espera começa no primeiro tick e
        // termina três ticks depois, quando a mensagem sai.
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let w = adiciona(&mut g, "Wait");
        g.set_value(w, "seconds", Value::Float(0.3)).unwrap();
        g.connect((ev, "exec"), (w, "exec")).unwrap();
        imprime(&mut g, (w, "exec"), "depois");
        let compiled = compile(&g).unwrap();
        let mut ctx = compiled.new_context(1);
        let mut host = Mock::default();
        let mut sched = Scheduler::new();
        sched.iniciar(&compiled, "OnReady");
        for _ in 0..3 {
            sched.tick(&compiled, &mut ctx, &mut host, 0.1).unwrap();
            assert!(host.impressos.is_empty(), "saiu antes da hora");
        }
        sched.tick(&compiled, &mut ctx, &mut host, 0.1).unwrap();
        assert_eq!(host.impressos, [(LogLevel::Info, "depois".to_string())]);
    }

    #[test]
    fn tween_interpola_e_depois_segue_o_fluxo() {
        // Tween de position_x de 10 até 100 em 1 s. Com ticks de 0.5 s: 55 no meio, 100 ao fim.
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let tw = adiciona(&mut g, "Tween");
        g.set_value(tw, "property", Value::String("position_x".into()))
            .unwrap();
        g.set_value(tw, "to", Value::Float(100.0)).unwrap();
        g.set_value(tw, "seconds", Value::Float(1.0)).unwrap();
        g.connect((ev, "exec"), (tw, "exec")).unwrap();
        g.connect((ev, "self"), (tw, "node")).unwrap();
        imprime(&mut g, (tw, "exec"), "fim");
        let compiled = compile(&g).unwrap();
        let mut ctx = compiled.new_context(7);
        let mut host = Mock::default();
        host.props.insert((7, "position_x".to_string()), 10.0);
        let mut sched = Scheduler::new();
        sched.iniciar(&compiled, "OnReady");
        sched.tick(&compiled, &mut ctx, &mut host, 0.5).unwrap();
        sched.tick(&compiled, &mut ctx, &mut host, 0.5).unwrap();
        assert!((host.props[&(7, "position_x".to_string())] - 55.0).abs() < 1e-9);
        assert!(host.impressos.is_empty());
        sched.tick(&compiled, &mut ctx, &mut host, 0.5).unwrap();
        assert_eq!(host.props[&(7, "position_x".to_string())], 100.0);
        assert_eq!(host.impressos.len(), 1);
    }

    #[test]
    fn await_signal_acorda_quando_outro_caminho_emite() {
        // Dois programas de OnReady: o primeiro aguarda "abriu"; o segundo emite e imprime "A".
        let mut g = Graph::default();
        let ev_b = adiciona(&mut g, "OnReady");
        let aguarda = adiciona(&mut g, "AwaitSignal");
        texto(&mut g, aguarda, "name", "abriu");
        g.connect((ev_b, "exec"), (aguarda, "exec")).unwrap();
        imprime(&mut g, (aguarda, "exec"), "B");
        let ev_a = adiciona(&mut g, "OnReady");
        let emite = adiciona(&mut g, "EmitSignal");
        texto(&mut g, emite, "name", "abriu");
        g.connect((ev_a, "exec"), (emite, "exec")).unwrap();
        imprime(&mut g, (emite, "exec"), "A");
        let compiled = compile(&g).unwrap();
        let mut ctx = compiled.new_context(1);
        let mut host = Mock::default();
        let mut sched = Scheduler::new();
        sched.iniciar(&compiled, "OnReady");
        sched.tick(&compiled, &mut ctx, &mut host, 0.0).unwrap();
        assert_eq!(host.impressos, [(LogLevel::Info, "A".to_string())]);
        // O sinal é entregue no começo do próximo quadro.
        sched.tick(&compiled, &mut ctx, &mut host, 0.0).unwrap();
        assert_eq!(
            host.impressos,
            [
                (LogLevel::Info, "A".to_string()),
                (LogLevel::Info, "B".to_string())
            ]
        );
    }

    #[test]
    fn timer_roda_o_tempo_esgotado_depois_dos_segundos() {
        // TimerStart(0.2): imprime "iniciou" na hora e "fim" depois de 0.2 s, com ticks de 0.1 s.
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let tm = adiciona(&mut g, "TimerStart");
        g.set_value(tm, "seconds", Value::Float(0.2)).unwrap();
        g.connect((ev, "exec"), (tm, "exec")).unwrap();
        imprime(&mut g, (tm, "started"), "iniciou");
        imprime(&mut g, (tm, "timeout"), "fim");
        let compiled = compile(&g).unwrap();
        let mut ctx = compiled.new_context(1);
        let mut host = Mock::default();
        let mut sched = Scheduler::new();
        sched.iniciar(&compiled, "OnReady");
        sched.tick(&compiled, &mut ctx, &mut host, 0.1).unwrap();
        assert_eq!(host.impressos, [(LogLevel::Info, "iniciou".to_string())]);
        sched.tick(&compiled, &mut ctx, &mut host, 0.1).unwrap();
        assert_eq!(host.impressos.len(), 1);
        sched.tick(&compiled, &mut ctx, &mut host, 0.1).unwrap();
        assert_eq!(host.impressos.len(), 2);
        assert_eq!(host.impressos[1], (LogLevel::Info, "fim".to_string()));
    }

    #[test]
    fn passo_a_passo_e_ponto_de_parada() {
        // Três prints em sequência: a, b e c. Ponto de parada no b.
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let pa = imprime(&mut g, (ev, "exec"), "a");
        let pb = imprime(&mut g, (pa, "exec"), "b");
        let pc = imprime(&mut g, (pb, "exec"), "c");
        let compiled = compile(&g).unwrap();
        let mut ctx = compiled.new_context(1);
        let mut host = Mock::default();
        let mut sched = Scheduler::new();
        sched.definir_ponto_de_parada(pb, true);
        sched.definir_rastro(true);
        sched.iniciar(&compiled, "OnReady");

        sched.tick(&compiled, &mut ctx, &mut host, 0.0).unwrap();
        assert_eq!(sched.pausa().map(|p| p.node), Some(pb));
        assert_eq!(host.impressos, [(LogLevel::Info, "a".to_string())]);

        // Um passo roda a op de "b" que está na frente (a constante da mensagem) e pausa de novo em "b".
        sched.passo(&compiled, &mut ctx, &mut host).unwrap();
        assert_eq!(sched.pausa().map(|p| p.node), Some(pb));
        assert_eq!(host.impressos.len(), 1);

        // Outro passo roda a escrita de "b" e pausa antes de "c".
        sched.passo(&compiled, &mut ctx, &mut host).unwrap();
        assert_eq!(sched.pausa().map(|p| p.node), Some(pc));
        assert_eq!(host.impressos.len(), 2);

        // Continuar sem pontos de parada até o fim.
        sched.definir_ponto_de_parada(pb, false);
        sched.continuar(&compiled, &mut ctx, &mut host).unwrap();
        assert!(sched.pausa().is_none());
        assert_eq!(host.impressos.len(), 3);
        assert_eq!(host.impressos[2], (LogLevel::Info, "c".to_string()));

        let rastro = sched.tomar_rastro();
        assert!(rastro.contains(&TraceEvent::Executou {
            evento: "OnReady".into(),
            node: pa
        }));
        assert!(rastro.iter().any(|e| matches!(e, TraceEvent::Valor { node, valor } if *node == pa && *valor == Value::String("a".into()))));
    }

    #[test]
    fn continue_and_step_without_pause_run_nothing_and_park_stops_before_first_op() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let pa = imprime(&mut g, (ev, "exec"), "a");
        imprime(&mut g, (pa, "exec"), "b");
        let compiled = compile(&g).unwrap();
        let mut ctx = compiled.new_context(1);
        let mut host = Mock::default();
        let mut sched = Scheduler::new();

        // Without a pause, continue and step run nothing: neither old nor new tasks.
        sched.continuar(&compiled, &mut ctx, &mut host).unwrap();
        sched.passo(&compiled, &mut ctx, &mut host).unwrap();
        assert!(host.impressos.is_empty());

        // Park: no operation runs, and the following steps reach the first Print without going past the second one.
        sched.iniciar(&compiled, "OnReady");
        sched.parar_antes(&compiled, &mut ctx, &mut host, 0.0).unwrap();
        assert!(sched.pausa().is_some());
        assert!(host.impressos.is_empty());
        for _ in 0..10 {
            if !host.impressos.is_empty() {
                break;
            }
            sched.passo(&compiled, &mut ctx, &mut host).unwrap();
        }
        assert_eq!(host.impressos, [(LogLevel::Info, "a".to_string())]);
    }

    #[test]
    fn recarga_a_quente_mantem_variavel_pelo_nome() {
        // v1: vida = 10, a cada quadro soma 1. Depois de dois quadros, vida = 12.
        let mut v1 = Graph::default();
        v1.add_variable("vida", PinType::Float, Value::Float(10.0), true)
            .unwrap();
        let ev = adiciona(&mut v1, "OnProcess");
        let le = adiciona(&mut v1, "GetVariable");
        texto(&mut v1, le, "name", "vida");
        let soma = adiciona(&mut v1, "Add");
        v1.set_value(soma, "b", Value::Float(1.0)).unwrap();
        let grava = adiciona(&mut v1, "SetVariable");
        texto(&mut v1, grava, "name", "vida");
        v1.connect((ev, "exec"), (grava, "exec")).unwrap();
        v1.connect((le, "value"), (soma, "a")).unwrap();
        v1.connect((soma, "result"), (grava, "value")).unwrap();
        let antigo = compile(&v1).unwrap();
        let mut ctx = antigo.new_context(1);
        let mut host = Mock::default();
        let mut sched = Scheduler::new();
        for _ in 0..2 {
            sched.iniciar(&antigo, "OnProcess");
            sched.tick(&antigo, &mut ctx, &mut host, 0.0).unwrap();
        }
        assert_eq!(ctx.vars[0], Value::Float(12.0));

        // v2: a mesma variável, com outro padrão, e uma variável nova. A vida mantém 12; a nova começa no padrão.
        let mut v2 = v1.clone();
        v2.variables[0].default = Value::Float(99.0);
        v2.add_variable("nivel", PinType::Int, Value::Int(5), false)
            .unwrap();
        let novo = compile(&v2).unwrap();
        let mut ctx2 = sched.recarregar(&ctx, &antigo, &novo);
        assert_eq!(ctx2.vars, [Value::Float(12.0), Value::Int(5)]);
        assert_eq!(sched.tarefas(), 0);

        sched.iniciar(&novo, "OnProcess");
        sched.tick(&novo, &mut ctx2, &mut host, 0.0).unwrap();
        assert_eq!(ctx2.vars[0], Value::Float(13.0));
    }

    #[test]
    fn recarga_com_tipo_mudado_volta_ao_padrao() {
        let mut v1 = Graph::default();
        v1.add_variable("x", PinType::Float, Value::Float(4.0), true)
            .unwrap();
        let antigo = compile(&v1).unwrap();
        let ctx = antigo.new_context(1);
        let mut v2 = Graph::default();
        v2.add_variable("x", PinType::Bool, Value::Bool(true), true)
            .unwrap();
        let novo = compile(&v2).unwrap();
        let ctx2 = recarregar_contexto(&ctx, &antigo, &novo);
        assert_eq!(ctx2.vars, [Value::Bool(true)]);
    }

    #[test]
    fn instancias_tem_agendadores_e_variaveis_separadas() {
        let mut g = Graph::default();
        g.add_variable("n", PinType::Float, Value::Float(0.0), true)
            .unwrap();
        let ev = adiciona(&mut g, "OnReady");
        let le = adiciona(&mut g, "GetVariable");
        texto(&mut g, le, "name", "n");
        let soma = adiciona(&mut g, "Add");
        g.set_value(soma, "b", Value::Float(1.0)).unwrap();
        let grava = adiciona(&mut g, "SetVariable");
        texto(&mut g, grava, "name", "n");
        g.connect((ev, "exec"), (grava, "exec")).unwrap();
        g.connect((le, "value"), (soma, "a")).unwrap();
        g.connect((soma, "result"), (grava, "value")).unwrap();
        let compiled = compile(&g).unwrap();
        let mut host = Mock::default();
        let (mut a, mut b) = (compiled.new_context(1), compiled.new_context(2));
        let (mut sa, mut sb) = (Scheduler::new(), Scheduler::new());
        sa.iniciar(&compiled, "OnReady");
        sa.tick(&compiled, &mut a, &mut host, 0.0).unwrap();
        sb.tick(&compiled, &mut b, &mut host, 0.0).unwrap();
        assert_eq!(a.vars[0], Value::Float(1.0));
        assert_eq!(b.vars[0], Value::Float(0.0));
    }

    #[test]
    fn for_each_repete_e_expoe_o_indice() {
        // ForEach(3): imprime "x" três vezes e grava o índice numa variável inteira. Ao fim, o último índice é 2.
        let mut g = Graph::default();
        g.add_variable("ultimo", PinType::Int, Value::Int(-1), false)
            .unwrap();
        let ev = adiciona(&mut g, "OnReady");
        let fe = adiciona(&mut g, "ForEach");
        g.set_value(fe, "count", Value::Int(3)).unwrap();
        g.connect((ev, "exec"), (fe, "exec")).unwrap();
        let p = imprime(&mut g, (fe, "body"), "x");
        let _ = p;
        let grava = adiciona(&mut g, "SetVariable");
        texto(&mut g, grava, "name", "ultimo");
        g.connect((fe, "body"), (grava, "exec")).unwrap();
        g.connect((fe, "index"), (grava, "value")).unwrap();
        let compiled = compile(&g).unwrap();
        let mut ctx = compiled.new_context(1);
        let mut host = Mock::default();
        let mut sched = Scheduler::new();
        sched.iniciar(&compiled, "OnReady");
        sched.tick(&compiled, &mut ctx, &mut host, 0.0).unwrap();
        assert_eq!(host.impressos.len(), 3);
        assert_eq!(ctx.vars[0], Value::Int(2));
    }

    #[test]
    fn audio_chega_ao_host_com_os_valores_lidos() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let pl = adiciona(&mut g, "PlaySound");
        g.set_value(pl, "node", Value::Node(7)).unwrap();
        texto(&mut g, pl, "stream", "00000000000000ab");
        g.set_value(pl, "volume", Value::Float(-6.0)).unwrap();
        g.connect((ev, "exec"), (pl, "exec")).unwrap();
        let compiled = compile(&g).unwrap();
        let mut host = Mock::default();
        let mut sched = Scheduler::new();
        sched.iniciar(&compiled, "OnReady");
        sched
            .tick(&compiled, &mut compiled.new_context(1), &mut host, 0.0)
            .unwrap();
        // Os padrões dos pinos de Tocar: barramento Master, tom 1, sem repetir, reinicia.
        assert_eq!(
            host.audio,
            [AudioCommand::Play {
                node: Uid(7),
                stream: Uid(0xab),
                volume_db: -6.0,
                pitch: 1.0,
                bus: "Master".into(),
                looping: false,
                from_start: true,
            }]
        );
    }

    /// Função "soma": lê `soma.a` e `soma.b` e grava a soma em `soma.resultado`. Quem chama preenche as
    /// entradas antes e lê a saída depois, pelas variáveis de nome combinado.
    fn grafo_com_funcao_soma() -> Graph {
        let mut g = Graph::default();
        for nome in ["soma.a", "soma.b", "soma.resultado", "total"] {
            g.add_variable(nome, PinType::Float, Value::Float(0.0), false)
                .unwrap();
        }
        // Corpo da função.
        let def = adiciona(&mut g, "Function");
        texto(&mut g, def, "name", "soma");
        let la = adiciona(&mut g, "GetVariable");
        texto(&mut g, la, "name", "soma.a");
        let lb = adiciona(&mut g, "GetVariable");
        texto(&mut g, lb, "name", "soma.b");
        let add = adiciona(&mut g, "Add");
        let grava = adiciona(&mut g, "SetVariable");
        texto(&mut g, grava, "name", "soma.resultado");
        let ret = adiciona(&mut g, "Return");
        g.connect((def, "exec"), (grava, "exec")).unwrap();
        g.connect((la, "value"), (add, "a")).unwrap();
        g.connect((lb, "value"), (add, "b")).unwrap();
        g.connect((add, "result"), (grava, "value")).unwrap();
        g.connect((grava, "exec"), (ret, "exec")).unwrap();
        // Quem chama: preenche as entradas, chama e lê o resultado.
        let ev = adiciona(&mut g, "OnReady");
        let pa = adiciona(&mut g, "SetVariable");
        texto(&mut g, pa, "name", "soma.a");
        g.set_value(pa, "value", Value::Float(2.0)).unwrap();
        let pb = adiciona(&mut g, "SetVariable");
        texto(&mut g, pb, "name", "soma.b");
        g.set_value(pb, "value", Value::Float(3.0)).unwrap();
        let chama = adiciona(&mut g, "CallFunction");
        texto(&mut g, chama, "name", "soma");
        let le = adiciona(&mut g, "GetVariable");
        texto(&mut g, le, "name", "soma.resultado");
        let total = adiciona(&mut g, "SetVariable");
        texto(&mut g, total, "name", "total");
        g.connect((ev, "exec"), (pa, "exec")).unwrap();
        g.connect((pa, "exec"), (pb, "exec")).unwrap();
        g.connect((pb, "exec"), (chama, "exec")).unwrap();
        g.connect((chama, "exec"), (total, "exec")).unwrap();
        g.connect((le, "value"), (total, "value")).unwrap();
        g
    }

    #[test]
    fn funcao_chamada_com_retorno() {
        // A função soma 2 + 3; quem chama lê o resultado e grava em "total", depois que a função termina.
        let compiled = compile(&grafo_com_funcao_soma()).unwrap();
        let mut ctx = compiled.new_context(1);
        let mut host = Mock::default();
        let mut sched = Scheduler::new();
        sched.iniciar(&compiled, "OnReady");
        sched.tick(&compiled, &mut ctx, &mut host, 0.0).unwrap();
        let valor = |nome: &str| {
            let i = compiled
                .variables
                .iter()
                .position(|v| v.name == nome)
                .unwrap();
            ctx.vars[i].clone()
        };
        assert_eq!(valor("soma.resultado"), Value::Float(5.0));
        assert_eq!(valor("total"), Value::Float(5.0));
    }

    #[test]
    fn return_interrompe_o_corpo_da_funcao() {
        // Corpo: Sequência; passo 1 é Retornar; passo 2 marcaria "marca". O passo 2 nunca roda.
        let mut g = Graph::default();
        g.add_variable("marca", PinType::Float, Value::Float(0.0), false)
            .unwrap();
        let def = adiciona(&mut g, "Function");
        texto(&mut g, def, "name", "parar");
        let seq = adiciona(&mut g, "Sequence");
        let ret = adiciona(&mut g, "Return");
        let marca = adiciona(&mut g, "SetVariable");
        texto(&mut g, marca, "name", "marca");
        g.set_value(marca, "value", Value::Float(1.0)).unwrap();
        g.connect((def, "exec"), (seq, "exec")).unwrap();
        g.connect((seq, "then_0"), (ret, "exec")).unwrap();
        g.connect((seq, "then_1"), (marca, "exec")).unwrap();
        let ev = adiciona(&mut g, "OnReady");
        let chama = adiciona(&mut g, "CallFunction");
        texto(&mut g, chama, "name", "parar");
        g.connect((ev, "exec"), (chama, "exec")).unwrap();
        let compiled = compile(&g).unwrap();
        let mut ctx = compiled.new_context(1);
        let mut sched = Scheduler::new();
        sched.iniciar(&compiled, "OnReady");
        sched
            .tick(&compiled, &mut ctx, &mut Mock::default(), 0.0)
            .unwrap();
        assert_eq!(ctx.vars[0], Value::Float(0.0));
    }

    #[test]
    fn chamada_recursiva_e_recusada() {
        let mut g = Graph::default();
        let def = adiciona(&mut g, "Function");
        texto(&mut g, def, "name", "loop");
        // A função chama a si mesma, e um OnReady a chama para o embutimento acontecer.
        let chama = adiciona(&mut g, "CallFunction");
        texto(&mut g, chama, "name", "loop");
        g.connect((def, "exec"), (chama, "exec")).unwrap();
        let ev = adiciona(&mut g, "OnReady");
        let de_fora = adiciona(&mut g, "CallFunction");
        texto(&mut g, de_fora, "name", "loop");
        g.connect((ev, "exec"), (de_fora, "exec")).unwrap();
        let erros = compile(&g).unwrap_err();
        assert!(
            erros.iter().any(|e| e.message.contains("recursiva")),
            "{erros:?}"
        );
    }

    #[test]
    fn chamada_de_funcao_inexistente_e_recusada() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let chama = adiciona(&mut g, "CallFunction");
        texto(&mut g, chama, "name", "nada");
        g.connect((ev, "exec"), (chama, "exec")).unwrap();
        let erros = compile(&g).unwrap_err();
        assert!(
            erros.iter().any(|e| e.message.contains("não existe")),
            "{erros:?}"
        );
    }

    /// Três estados: "Parado" espera "ui_accept" e vai para "Andando"; "Andando" só imprime. Entrar e sair
    /// de estado imprime uma mensagem.
    fn grafo_de_estados() -> Graph {
        let mut g = Graph::default();
        let parado = adiciona(&mut g, "State");
        texto(&mut g, parado, "state", "Parado");
        let acao = adiciona(&mut g, "ActionJustPressed");
        texto(&mut g, acao, "action", "ui_accept");
        let br = adiciona(&mut g, "Branch");
        let vai = adiciona(&mut g, "GoToState");
        texto(&mut g, vai, "state", "Andando");
        g.connect((parado, "exec"), (br, "exec")).unwrap();
        g.connect((acao, "pressed"), (br, "condition")).unwrap();
        g.connect((br, "true"), (vai, "exec")).unwrap();
        let andando = adiciona(&mut g, "State");
        texto(&mut g, andando, "state", "Andando");
        imprime(&mut g, (andando, "exec"), "andando");
        let entra = adiciona(&mut g, "StateEnter");
        texto(&mut g, entra, "state", "Andando");
        imprime(&mut g, (entra, "exec"), "entrei");
        let sai = adiciona(&mut g, "StateExit");
        texto(&mut g, sai, "state", "Parado");
        imprime(&mut g, (sai, "exec"), "saí");
        g
    }

    #[test]
    fn maquina_de_estados_troca_de_estado() {
        let compiled = compile(&grafo_de_estados()).unwrap();
        assert_eq!(compiled.estado_inicial.as_deref(), Some("Parado"));
        let mut ctx = compiled.new_context(1);
        let mut host = Mock::default();
        let mut sched = Scheduler::new();

        // Quadro 1: nada apertado, o estado segue "Parado" e nada imprime.
        sched.iniciar_estado(&compiled, &ctx.estado);
        sched.tick(&compiled, &mut ctx, &mut host, 0.0).unwrap();
        assert_eq!(ctx.estado, "Parado");
        assert!(host.impressos.is_empty());

        // Quadro 2: o botão é apertado. A troca sai de "Parado" e entra em "Andando" no mesmo quadro.
        host.pressionadas.push("ui_accept".into());
        sched.iniciar_estado(&compiled, &ctx.estado);
        sched.tick(&compiled, &mut ctx, &mut host, 0.0).unwrap();
        assert_eq!(ctx.estado, "Andando");
        assert_eq!(
            host.impressos,
            [
                (LogLevel::Info, "saí".to_string()),
                (LogLevel::Info, "entrei".to_string())
            ]
        );

        // Quadro 3: o estado "Andando" é o que roda agora.
        sched.iniciar_estado(&compiled, &ctx.estado);
        sched.tick(&compiled, &mut ctx, &mut host, 0.0).unwrap();
        assert_eq!(
            host.impressos.last(),
            Some(&(LogLevel::Info, "andando".to_string()))
        );
    }

    #[test]
    fn estado_inexistente_e_recusado_na_validacao() {
        let mut g = grafo_de_estados();
        let vai = g.nodes.iter().find(|n| n.kind == "GoToState").unwrap().id;
        g.set_value(vai, "state", Value::String("Nada".into()))
            .unwrap();
        let erros = compile(&g).unwrap_err();
        assert!(
            erros
                .iter()
                .any(|e| e.node == Some(vai) && e.message.contains("estado 'Nada' não existe")),
            "{erros:?}"
        );
    }
}
