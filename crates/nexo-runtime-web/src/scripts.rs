//! Scripts visuais na reprodução (VIS-023, CON-004). Cada nó com grafo ganha uma instância: o grafo compilado, um
//! agendador e um contexto. `set_scripts` guarda os grafos (RON) por UID de nó. `play_start` os compila, e cada
//! quadro (`tick`) roda os eventos pelo Host (`host.rs`). Na reprodução, um `set_scripts` recarrega a quente os grafos
//! alterados: as variáveis de mesmo nome e tipo mantêm o valor (`Scheduler::recarregar`).
//!
//! Per-instance debugging (VIS-031): a pause (breakpoint or step) freezes only that instance. The frame events it has
//! not run yet wait in its queue, and it creates no new tasks while paused. `continuar` runs the rest of the frame
//! once; `passo` runs one operation and stops before the next one, even when that one belongs to the next event or
//! frame. The other instances keep running.

use crate::editor::{Play, SceneEditor, js_err};
use crate::host::{Mensagem, Mundo, Ponteiro};
use nexo_core::scene::value_of;
use nexo_core::timer::TimerEvent;
use nexo_core::{Input, ProjectSettings, Scene, Uid};
use nexo_graph::{Compiled, Context, Graph, Host, Pausa, Scheduler, Trace, VmError, compile};
use nexo_physics::{EventKind, PhysicsEvent, STEP};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use wasm_bindgen::prelude::*;

/// A script event that has not started yet: its name, the time it receives, and what triggered it (VIS-002).
#[derive(Clone, Debug, PartialEq)]
struct Evento {
    nome: &'static str,
    dt: f64,
    /// Text the program's filter must match: the input action, the signal's method or the other node's name.
    filter: String,
    /// The other node of the dispatch: the colliding body or the signal's emitter. Zero when there is none.
    other: u64,
}

impl Evento {
    /// Frame event: runs with the frame time and no filter.
    fn frame(name: &'static str, dt: f64) -> Evento {
        Evento { nome: name, dt, filter: String::new(), other: 0 }
    }

    /// Dispatched event (signal, collision, timer or input action). It runs with time zero: the waits that the frame
    /// events already advanced do not move again on it.
    fn dispatch(name: &'static str, filter: &str, other: u64) -> Evento {
        Evento { nome: name, dt: 0.0, filter: filter.to_string(), other }
    }
}

/// Uma instância de script: o grafo de um nó, com o agendador e o contexto dele.
pub(crate) struct Instancia {
    /// Nó dono do script: a saída "self" dos eventos e o nó das mensagens do console.
    pub(crate) dono: Uid,
    /// Texto RON que originou o grafo compilado. Quando muda, a instância é recarregada.
    texto: String,
    grafo: Compiled,
    agenda: Scheduler,
    contexto: Context,
    /// Erros de execução já escritos no console. Cada mensagem aparece uma vez, até a próxima recarga.
    erros: BTreeSet<String>,
    /// Frame events that have not started yet. A pause keeps them, and `continuar` or `passo` resume them.
    fila: VecDeque<Evento>,
    /// Signal, collision and timer events delivered to this instance (VIS-002). The next frame runs them first, and
    /// they wait while the instance is paused.
    delivered: Vec<Evento>,
    /// The step reached the end of a frame: the next frame starts paused, before its first operation.
    aguarda_quadro: bool,
}

#[wasm_bindgen]
impl SceneEditor {
    /// Guarda os grafos de script: um objeto JSON {"<uid do nó>": "<texto RON do grafo>"}, que substitui o anterior.
    /// Não compila aqui: um grafo com erro faz `play_start` falhar, com a mensagem do script. Na reprodução, os grafos
    /// alterados são recarregados na hora (recarga a quente); se um deles tem erro, o erro vai ao console e a versão
    /// anterior segue. Dá erro só para JSON ou UID inválidos.
    pub fn set_scripts(&mut self, json: &str) -> Result<(), JsError> {
        let mapa: BTreeMap<String, String> = serde_json::from_str(json).map_err(js_err)?;
        let mut textos = BTreeMap::new();
        for (uid, texto) in mapa {
            textos.insert(uid.parse::<Uid>().map_err(js_err)?, texto);
        }
        self.scripts = textos;
        if let Some(p) = &mut self.play {
            sincronizar(&mut p.instancias, &p.scene, &self.scripts, &mut self.console);
            aplicar_pontos(&mut p.instancias, &self.pontos);
            atualizar_rastro(&mut p.instancias, self.selecionada, self.painel_aberto);
        }
        Ok(())
    }

    /// Mensagens do console desde a última chamada, em JSON: [{level, node, message}] (CON-004). `level` é "info",
    /// "warning" ou "error"; `node` é o UID do nó dono do script. A fila é esvaziada.
    pub fn console_messages(&mut self) -> Result<String, JsError> {
        let mensagens = std::mem::take(&mut self.console);
        serde_json::to_string(&mensagens).map_err(js_err)
    }

    /// Posição do ponteiro na reprodução: x e y em pixels do canvas, e a mesma posição no mundo, como o viewport a
    /// desenha. Os scripts leem em "Posição do mouse (mundo)" e "(tela)".
    pub fn pointer_event(&mut self, x: f64, y: f64, world_x: f64, world_y: f64) {
        self.pointer = Ponteiro { tela: [x, y], mundo: [world_x, world_y] };
    }
}

/// Compila os scripts de uma reprodução, na ordem da árvore. Se algum grafo tem erro, devolve todos, uma linha por
/// script, com o nome do nó. Sem erro, devolve as instâncias com o evento "Ao ficar pronto" agendado.
pub(crate) fn iniciar(cena: &Scene, textos: &BTreeMap<Uid, String>) -> Result<Vec<Instancia>, String> {
    let mut instancias = Vec::new();
    let mut erros = Vec::new();
    for uid in cena.pre_order() {
        let Some(texto) = textos.get(&uid) else {
            continue;
        };
        match Instancia::novo(cena, uid, texto) {
            Ok(inst) => instancias.push(inst),
            Err(erro) => erros.push(format!("Script de '{}': {erro}", nome_de(cena, uid))),
        }
    }
    if erros.is_empty() { Ok(instancias) } else { Err(erros.join("\n")) }
}

/// Alinha as instâncias da reprodução com os grafos recebidos: remove as de nós sem script ou sem nó, recarrega as
/// de grafos alterados e cria as dos scripts novos. Erros de compilação vão ao console, e a instância anterior segue.
pub(crate) fn sincronizar(
    instancias: &mut Vec<Instancia>,
    cena: &Scene,
    textos: &BTreeMap<Uid, String>,
    console: &mut Vec<Mensagem>,
) {
    instancias.retain(|i| textos.contains_key(&i.dono) && cena.node(i.dono).is_some());
    for uid in cena.pre_order() {
        let Some(texto) = textos.get(&uid) else {
            continue;
        };
        match instancias.iter_mut().find(|i| i.dono == uid) {
            Some(inst) if inst.texto == *texto => {}
            Some(inst) => match compilar(texto) {
                Ok(novo) => inst.recarregar(novo, texto),
                Err(erro) => {
                    // O texto novo fica registrado: o mesmo erro não volta ao console a cada recarga.
                    inst.texto = texto.clone();
                    console.push(erro_no_console(cena, uid, format!("recarga falhou, segue a versão anterior: {erro}")));
                }
            },
            None => match Instancia::novo(cena, uid, texto) {
                Ok(inst) => instancias.push(inst),
                Err(erro) => console.push(erro_no_console(cena, uid, erro)),
            },
        }
    }
}

/// Gives each instance the breakpoints of its own UID (VIS-031). Called when the breakpoints or the instances change,
/// not on every frame.
pub(crate) fn aplicar_pontos(instancias: &mut [Instancia], pontos: &BTreeMap<Uid, BTreeSet<u64>>) {
    for inst in instancias {
        inst.agenda.definir_pontos_de_parada(pontos.get(&inst.dono).cloned().unwrap_or_default());
    }
}

/// Whether an instance records its trace (VIS-030, DBG-003). The trace costs a few set and map operations per
/// operation, so it is on only for the instance the debugger follows, and only when there is a reason to look: the
/// panel is open, the instance is paused, or it has a breakpoint.
pub(crate) fn grava_rastro(escolhida: bool, painel_aberto: bool, pausada: bool, tem_ponto: bool) -> bool {
    escolhida && (painel_aberto || pausada || tem_ponto)
}

/// Turns the trace of each instance on or off, as `grava_rastro` decides. It runs every frame: it only sets flags,
/// and it clears the trace of an instance whose trace turns off, so nothing is kept without a reader.
pub(crate) fn atualizar_rastro(instancias: &mut [Instancia], escolhida: Option<Uid>, painel_aberto: bool) {
    for inst in instancias {
        let ligado = grava_rastro(
            escolhida == Some(inst.dono),
            painel_aberto,
            inst.pausada(),
            !inst.agenda.pontos_de_parada().is_empty(),
        );
        inst.agenda.definir_rastro(ligado);
        if !ligado {
            inst.tomar_rastro();
        }
    }
}

/// Um quadro de todos os scripts da reprodução. Por instância: primeiro um evento "OnPhysicsProcess" por passo de
/// física (com o passo fixo), depois o "OnProcess" do quadro, com o estado ativo. Erro de execução vai ao console uma
/// vez por mensagem; a instância segue.
/// A paused instance gets no new frame.
pub(crate) fn tick(
    p: &mut Play,
    passos: usize,
    ajustes: &ProjectSettings,
    entrada: &Input,
    ponteiro: Ponteiro,
    console: &mut Vec<Mensagem>,
    dt: f64,
) {
    p.instancias.retain(|i| p.scene.node(i.dono).is_some());
    for (owner, event) in deliveries(p) {
        if let Some(inst) = p.instancias.iter_mut().find(|i| i.dono == owner) {
            inst.delivered.push(event);
        }
    }
    for inst in &mut p.instancias {
        let resultado = {
            let mut mundo = Mundo {
                cena: &mut p.scene,
                fisica: &mut p.physics,
                audio: &mut p.audio,
                entrada,
                ajustes,
                ponteiro,
                console: &mut *console,
                dono: inst.dono,
            };
            inst.quadro_novo(&mut mundo, passos, dt)
        };
        registrar(inst, &p.scene, console, resultado);
    }
}

/// Takes the physics and timer events of this frame and turns them into script events, each with the instance it
/// concerns (VIS-002). A signal runs the method of each connection of its emitter (SCN-034) on the target. A body
/// that starts a collision also runs its "On Collision" with the other node's name. A timer's timeout runs its own
/// "On Timer". The raw events stay in the play for the editor (`physics_events`, `timer_events`).
fn deliveries(play: &mut Play) -> Vec<(Uid, Evento)> {
    let physics = play.physics.take_events();
    let timers = play.timers.take_events();
    let routed = route_events(&play.scene, &physics, &timers);
    play.editor_physics.extend(physics);
    play.editor_timers.extend(timers);
    routed
}

/// The script events of raw physics and timer events, each with the instance it concerns. Only reads the scene.
fn route_events(scene: &Scene, physics: &[PhysicsEvent], timers: &[TimerEvent]) -> Vec<(Uid, Evento)> {
    let mut routed = Vec::new();
    for ev in physics {
        let signal = match ev.kind {
            EventKind::BodyEntered => "body_entered",
            EventKind::BodyExited => "body_exited",
            EventKind::AreaEntered => "area_entered",
            EventKind::AreaExited => "area_exited",
            EventKind::CollisionStarted => "collision_started",
            EventKind::CollisionEnded => "collision_ended",
        };
        dispatch_signal(scene, ev.node, signal, ev.other.0, &mut routed);
        if ev.kind == EventKind::CollisionStarted {
            let other_name = scene.node(ev.other).map(|n| n.name.clone()).unwrap_or_default();
            routed.push((ev.node, Evento::dispatch("OnCollision", &other_name, ev.other.0)));
        }
    }
    for ev in timers {
        dispatch_signal(scene, ev.node, ev.kind, 0, &mut routed);
        if ev.kind == "timeout" {
            routed.push((ev.node, Evento::dispatch("OnTimer", "", 0)));
        }
    }
    routed
}

/// Adds an "On Signal" event for each connection of `emitter` with the signal `signal` (SCN-034). The target runs the
/// method of the connection, and `other` is the other node of the dispatch (the collider, or zero for timers).
fn dispatch_signal(scene: &Scene, emitter: Uid, signal: &str, other: u64, out: &mut Vec<(Uid, Evento)>) {
    let Some(node) = scene.node(emitter) else {
        return;
    };
    for connection in node.connections.iter().filter(|c| c.signal == signal) {
        out.push((connection.target, Evento::dispatch("OnSignal", &connection.method, other)));
    }
}

/// Input events of the frame (VIS-002): one for each input action that the instance's programs listen to and that was
/// pressed in this frame.
fn input_events(graph: &Compiled, host: &dyn Host) -> Vec<Evento> {
    let actions: BTreeSet<&str> = graph
        .programs
        .iter()
        .filter(|p| p.event == "OnInputAction" && !p.filter.is_empty())
        .map(|p| p.filter.as_str())
        .collect();
    actions
        .into_iter()
        .filter(|action| host.action_just_pressed(action))
        .map(|action| Evento::dispatch("OnInputAction", action, 0))
        .collect()
}

/// Writes an execution error to the console, once per message, and drops the rest of the frame of the instance.
pub(crate) fn registrar(inst: &mut Instancia, cena: &Scene, console: &mut Vec<Mensagem>, resultado: Result<(), VmError>) {
    let Err(erro) = resultado else {
        return;
    };
    inst.fila.clear();
    inst.aguarda_quadro = false;
    let mensagem = erro.to_string();
    if inst.erros.insert(mensagem.clone()) {
        console.push(erro_no_console(cena, inst.dono, mensagem));
    }
}

impl Instancia {
    /// Instância nova: compila o grafo e agenda o evento "Ao ficar pronto", que roda no primeiro quadro.
    /// The node's exported values (VIS-022) are applied to the new context.
    fn novo(cena: &Scene, dono: Uid, texto: &str) -> Result<Instancia, String> {
        let grafo = compilar(texto)?;
        let mut agenda = Scheduler::new();
        agenda.iniciar(&grafo, "OnReady");
        let contexto = context_of_node(cena, dono, &grafo)?;
        Ok(Instancia {
            dono,
            texto: texto.to_string(),
            grafo,
            agenda,
            contexto,
            erros: BTreeSet::new(),
            fila: VecDeque::new(),
            delivered: Vec::new(),
            aguarda_quadro: false,
        })
    }

    /// Recarga a quente: troca o grafo e mantém as variáveis de mesmo nome e tipo (`Scheduler::recarregar`). As
    /// tarefas em curso são descartadas. Um estado que deixou de existir no grafo novo volta ao estado inicial.
    /// The pause and the rest of the frame are dropped too.
    fn recarregar(&mut self, novo: Compiled, texto: &str) {
        self.contexto = self.agenda.recarregar(&self.contexto, &self.grafo, &novo);
        self.grafo = novo;
        self.texto = texto.to_string();
        self.erros.clear();
        self.fila.clear();
        self.delivered.clear();
        self.aguarda_quadro = false;
        let estado = self.contexto.estado.clone();
        if !estado.is_empty() && !self.grafo.programs.iter().any(|p| p.estado.as_deref() == Some(estado.as_str())) {
            self.contexto.estado = self.grafo.estado_inicial.clone().unwrap_or_default();
        }
    }

    /// Paused for debugging: the schedule stopped at an operation, or a step reached the end of a frame. A paused
    /// instance gets no new frames until `continuar` or `passo`.
    pub(crate) fn pausada(&self) -> bool {
        self.agenda.pausa().is_some() || self.aguarda_quadro
    }

    /// Where the execution stopped, if it stopped at an operation (breakpoint or step by step).
    pub(crate) fn pausa(&self) -> Option<&Pausa> {
        self.agenda.pausa()
    }

    /// Decides what the instance does in the frame: if it is paused, it waits; if a step reached the end of a
    /// frame, the new frame starts paused; otherwise, the whole frame runs up to the next pause.
    fn quadro_novo(&mut self, host: &mut dyn Host, passos: usize, dt: f64) -> Result<(), VmError> {
        if self.aguarda_quadro {
            self.aguarda_quadro = false;
            self.fila = self.frame_queue(&*host, passos, dt);
            return self.avancar_fila(host, true);
        }
        if self.pausada() {
            return Ok(());
        }
        self.fila = self.frame_queue(&*host, passos, dt);
        self.avancar_fila(host, false)
    }

    /// The events of the frame, in order: the ones delivered to the instance, the input actions pressed in this frame,
    /// one "OnPhysicsProcess" per fixed physics step, and the "OnProcess" with the frame time (VIS-002).
    fn frame_queue(&mut self, host: &dyn Host, steps: usize, dt: f64) -> VecDeque<Evento> {
        let mut queue: VecDeque<Evento> = self.delivered.drain(..).collect();
        queue.extend(input_events(&self.grafo, host));
        queue.extend((0..steps).map(|_| Evento::frame("OnPhysicsProcess", STEP)));
        queue.push_back(Evento::frame("OnProcess", dt));
        queue
    }

    /// Starts the event (creates its tasks) and runs it up to the next pause or to its end. `parado`: stops before the
    /// first operation instead of running it (step by step).
    fn comecar(&mut self, evento: Evento, host: &mut dyn Host, parado: bool) -> Result<(), VmError> {
        self.agenda.start_filtered(&self.grafo, evento.nome, &evento.filter, evento.other);
        if evento.nome == "OnProcess" {
            self.agenda.iniciar_estado(&self.grafo, &self.contexto.estado);
        }
        if parado {
            self.agenda.parar_antes(&self.grafo, &mut self.contexto, host, evento.dt)
        } else {
            self.agenda.tick(&self.grafo, &mut self.contexto, host, evento.dt)
        }
    }

    /// Starts the queued events one at a time, until a pause or until the queue is empty. With `parado`, the first op
    /// of each event pauses, and the next event starts only after its operation has run.
    fn avancar_fila(&mut self, host: &mut dyn Host, parado: bool) -> Result<(), VmError> {
        while self.agenda.pausa().is_none() {
            let Some(evento) = self.fila.pop_front() else {
                break;
            };
            self.comecar(evento, host, parado)?;
        }
        Ok(())
    }

    /// Continue (VIS-031): leaves the pause, runs the rest of the frame once, and the following frames up to the next pause.
    pub(crate) fn continuar(&mut self, host: &mut dyn Host) -> Result<(), VmError> {
        if !self.pausada() {
            return Ok(());
        }
        self.aguarda_quadro = false;
        self.agenda.continuar(&self.grafo, &mut self.contexto, host)?;
        self.avancar_fila(host, false)
    }

    /// Next operation (VIS-031): runs one op and pauses before the next one, even in the next event.
    /// If the frame ends, the next frame starts paused. Without a pause, it does nothing.
    pub(crate) fn passo(&mut self, host: &mut dyn Host) -> Result<(), VmError> {
        if self.aguarda_quadro || self.agenda.pausa().is_none() {
            return Ok(());
        }
        self.agenda.passo(&self.grafo, &mut self.contexto, host)?;
        if self.agenda.pausa().is_none() {
            self.avancar_fila(host, true)?;
            if self.agenda.pausa().is_none() {
                self.aguarda_quadro = true;
            }
        }
        Ok(())
    }

    /// Takes the trace of the instance since the last call (VIS-030, DBG-003). Empty if the trace is off.
    pub(crate) fn tomar_rastro(&mut self) -> Trace {
        self.agenda.tomar_rastro()
    }
}

/// Lê e compila um grafo. O erro junta as mensagens de leitura ou de validação, cada uma com o nó onde está.
fn compilar(texto: &str) -> Result<Compiled, String> {
    let grafo = Graph::from_ron(texto)?;
    compile(&grafo).map_err(|erros| erros.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; "))
}

/// Context of a node: its variables start with the exported values stored in `script_vars` (VIS-022). Without
/// the map, or with an empty one, the script's defaults apply.
fn context_of_node(cena: &Scene, dono: Uid, grafo: &Compiled) -> Result<Context, String> {
    let values = cena.node(dono).map(|n| value_of(n, "script_vars")).unwrap_or(serde_json::Value::Null);
    let empty = serde_json::Map::new();
    grafo.new_context_with(dono.0, values.as_object().unwrap_or(&empty))
}

fn nome_de(cena: &Scene, uid: Uid) -> String {
    cena.node(uid).map_or_else(|| uid.to_string(), |n| n.name.clone())
}

fn erro_no_console(cena: &Scene, dono: Uid, texto: String) -> Mensagem {
    Mensagem { level: "error", node: dono, message: format!("Script de '{}': {texto}", nome_de(cena, dono)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexo_core::AudioPlayback;
    use nexo_graph::{PinType, Value};
    use nexo_physics::Physics;
    use serde_json::json;

    /// Test world: an empty scene, with the physics and audio that the prints do not use.
    struct Cenario {
        cena: Scene,
        fisica: Physics,
        audio: AudioPlayback,
        entrada: Input,
        ajustes: ProjectSettings,
        console: Vec<Mensagem>,
    }

    impl Cenario {
        fn novo() -> Cenario {
            let cena = Scene::new("Teste", 1);
            let ajustes = ProjectSettings::default();
            let fisica = Physics::build(&cena, &ajustes);
            Cenario {
                cena,
                fisica,
                audio: AudioPlayback::default(),
                entrada: Input::default(),
                ajustes,
                console: Vec::new(),
            }
        }

        /// Texts printed by the script of `dono`, in order.
        fn textos(&self, dono: Uid) -> Vec<String> {
            self.console.iter().filter(|m| m.node == dono).map(|m| m.message.clone()).collect()
        }
    }

    /// Runs `acao` with the test world of the script `dono`.
    fn com_mundo<T>(c: &mut Cenario, dono: Uid, acao: impl FnOnce(&mut Mundo<'_>) -> T) -> T {
        let mut mundo = Mundo {
            cena: &mut c.cena,
            fisica: &mut c.fisica,
            audio: &mut c.audio,
            entrada: &c.entrada,
            ajustes: &c.ajustes,
            ponteiro: Ponteiro::default(),
            console: &mut c.console,
            dono,
        };
        acao(&mut mundo)
    }

    /// One frame of the instance, with `passos` physics steps.
    fn quadro_de(inst: &mut Instancia, c: &mut Cenario, passos: usize) -> Result<(), VmError> {
        let dono = inst.dono;
        com_mundo(c, dono, |m| inst.quadro_novo(m, passos, 0.1))
    }

    fn adiciona(g: &mut Graph, kind: &str) -> u64 {
        g.add_node(kind, [0.0, 0.0]).unwrap()
    }

    /// Graph with two events: "OnPhysicsProcess" prints "fis" and "OnProcess" prints "proc". Returns the nodes of the
    /// two prints. Each print has two operations (the message constant and the write), so one step does not always
    /// reach the write.
    fn grafo_de_quadro() -> (Graph, u64, u64) {
        let mut g = Graph::default();
        let ev_fis = adiciona(&mut g, "OnPhysicsProcess");
        let fis = adiciona(&mut g, "Print");
        g.set_value(fis, "msg", Value::String("fis".into())).unwrap();
        g.connect((ev_fis, "exec"), (fis, "exec")).unwrap();
        let ev_proc = adiciona(&mut g, "OnProcess");
        let proc = adiciona(&mut g, "Print");
        g.set_value(proc, "msg", Value::String("proc".into())).unwrap();
        g.connect((ev_proc, "exec"), (proc, "exec")).unwrap();
        (g, fis, proc)
    }

    fn instancia(dono: u64, g: &Graph) -> Instancia {
        Instancia::novo(&Scene::new("Teste", 1), Uid(dono), &g.to_ron().unwrap()).unwrap()
    }

    /// Next operation until `pare` holds, with at most ten steps.
    fn passa_ate(a: &mut Instancia, c: &mut Cenario, pare: impl Fn(&Instancia) -> bool) {
        for _ in 0..10 {
            if pare(&*a) {
                return;
            }
            let dono = a.dono;
            com_mundo(c, dono, |m| a.passo(m)).unwrap();
        }
    }

    /// Breakpoint on the "proc" print of instance `a` only.
    fn com_ponto_em(a: &mut Instancia, proc: u64) {
        let pontos = BTreeMap::from([(a.dono, BTreeSet::from([proc]))]);
        aplicar_pontos(std::slice::from_mut(a), &pontos);
    }

    #[test]
    fn ponto_de_parada_pausa_so_a_instancia_que_o_tem() {
        let (g, _, proc) = grafo_de_quadro();
        let mut a = instancia(1, &g);
        let mut b = instancia(2, &g);
        com_ponto_em(&mut a, proc);
        let mut c = Cenario::novo();

        quadro_de(&mut a, &mut c, 0).unwrap();
        quadro_de(&mut b, &mut c, 0).unwrap();

        assert!(a.pausada());
        assert_eq!(a.pausa().map(|p| p.node), Some(proc));
        assert!(c.textos(Uid(1)).is_empty());
        assert!(!b.pausada());
        assert_eq!(c.textos(Uid(2)), ["proc"]);
    }

    #[test]
    fn quadros_pausados_nao_acumulam_tarefas() {
        let (g, _, proc) = grafo_de_quadro();
        let mut a = instancia(1, &g);
        com_ponto_em(&mut a, proc);
        let mut c = Cenario::novo();
        quadro_de(&mut a, &mut c, 1).unwrap();
        assert!(a.pausada());
        let tarefas = a.agenda.tarefas();

        for _ in 0..10 {
            quadro_de(&mut a, &mut c, 1).unwrap();
        }

        assert_eq!(a.agenda.tarefas(), tarefas, "quadros parados criaram tarefas novas");
        assert_eq!(c.textos(Uid(1)), ["fis"], "a instância parada não deveria imprimir");
    }

    #[test]
    fn continuar_roda_o_resto_do_quadro_uma_vez() {
        let (g, _, proc) = grafo_de_quadro();
        let mut a = instancia(1, &g);
        com_ponto_em(&mut a, proc);
        let mut c = Cenario::novo();
        quadro_de(&mut a, &mut c, 1).unwrap();
        for _ in 0..5 {
            quadro_de(&mut a, &mut c, 1).unwrap();
        }

        com_mundo(&mut c, Uid(1), |m| a.continuar(m)).unwrap();
        assert!(!a.pausada());
        assert_eq!(c.textos(Uid(1)), ["fis", "proc"], "o resto do quadro roda uma vez só");

        // After continuing, the next frame runs again up to the breakpoint.
        quadro_de(&mut a, &mut c, 1).unwrap();
        assert_eq!(c.textos(Uid(1)), ["fis", "proc", "fis"]);
        assert_eq!(a.pausa().map(|p| p.node), Some(proc));
    }

    #[test]
    fn passo_cruza_eventos_e_quadros() {
        let (g, fis, proc) = grafo_de_quadro();
        let mut a = instancia(1, &g);
        com_ponto_em(&mut a, proc);
        let mut c = Cenario::novo();
        // Frame 1: "fis" runs and the execution stops at "proc" (breakpoint).
        quadro_de(&mut a, &mut c, 1).unwrap();
        assert_eq!(a.pausa().map(|p| p.node), Some(proc));
        assert_eq!(c.textos(Uid(1)), ["fis"]);

        // Next operation until the end of the frame: "proc" prints and the instance waits for the next frame.
        passa_ate(&mut a, &mut c, |i| i.pausa().is_none());
        assert_eq!(c.textos(Uid(1)), ["fis", "proc"]);
        assert!(a.pausada() && a.pausa().is_none(), "fim de quadro: espera o quadro seguinte");

        // Frame 2 starts paused at the first event, before any of its operations.
        quadro_de(&mut a, &mut c, 1).unwrap();
        assert_eq!(a.pausa().map(|p| p.node), Some(fis));
        assert_eq!(c.textos(Uid(1)), ["fis", "proc"]);

        // Next operation until "proc" of frame 2: "fis" prints and the pause falls at the start of "proc".
        passa_ate(&mut a, &mut c, |i| i.pausa().map(|p| p.node) == Some(proc));
        assert_eq!(c.textos(Uid(1)), ["fis", "proc", "fis"]);
        assert_eq!(a.pausa().map(|p| p.node), Some(proc));
    }

    #[test]
    fn recarga_limpa_a_pausa_e_o_quadro_que_faltava() {
        let (g, _, proc) = grafo_de_quadro();
        let mut a = instancia(1, &g);
        com_ponto_em(&mut a, proc);
        let mut c = Cenario::novo();
        quadro_de(&mut a, &mut c, 1).unwrap();
        assert!(a.pausada());

        a.recarregar(compile(&g).unwrap(), &g.to_ron().unwrap());

        assert!(!a.pausada());
        assert!(a.fila.is_empty());
        assert_eq!(a.agenda.tarefas(), 0);
    }

    #[test]
    fn grava_rastro_needs_the_chosen_instance_and_a_reason() {
        // chosen instance, panel open, paused, breakpoint
        assert!(!grava_rastro(false, true, true, true), "not the chosen instance");
        assert!(!grava_rastro(true, false, false, false), "no panel, no pause, no breakpoint");
        assert!(grava_rastro(true, true, false, false), "panel open");
        assert!(grava_rastro(true, false, true, false), "paused");
        assert!(grava_rastro(true, false, false, true), "has a breakpoint");
    }

    #[test]
    fn trace_stays_off_without_panel_pause_or_breakpoint() {
        let (g, _, _) = grafo_de_quadro();
        let mut todas = vec![instancia(1, &g)];
        atualizar_rastro(&mut todas, Some(Uid(1)), false);
        let mut c = Cenario::novo();

        quadro_de(&mut todas[0], &mut c, 1).unwrap();

        assert!(todas[0].tomar_rastro().is_empty(), "the chosen instance recorded a trace with no reason");
        assert_eq!(c.textos(Uid(1)), ["fis", "proc"], "the instance still runs");
    }

    #[test]
    fn trace_is_on_for_the_chosen_instance_with_a_breakpoint() {
        let (g, _, proc) = grafo_de_quadro();
        let mut a = instancia(1, &g);
        com_ponto_em(&mut a, proc);
        atualizar_rastro(std::slice::from_mut(&mut a), Some(Uid(1)), false);
        let mut c = Cenario::novo();

        quadro_de(&mut a, &mut c, 1).unwrap();

        assert!(!a.tomar_rastro().is_empty());
    }

    #[test]
    fn rastro_so_grava_na_instancia_escolhida() {
        let (g, _, _) = grafo_de_quadro();
        let mut todas = vec![instancia(1, &g), instancia(2, &g)];
        atualizar_rastro(&mut todas, Some(Uid(2)), true);
        let mut c = Cenario::novo();
        for inst in &mut todas {
            quadro_de(inst, &mut c, 0).unwrap();
        }

        assert!(todas[0].tomar_rastro().is_empty());
        assert!(!todas[1].tomar_rastro().is_empty());
    }

    /// Scene with one node whose `script_vars` are `values`, and the script text: an exported Float "vida" (default
    /// 1.0) and a local Int "local" (default 7).
    fn scene_with_node(values: serde_json::Value) -> (Scene, Uid, String) {
        let mut g = Graph::default();
        g.add_variable("vida", PinType::Float, Value::Float(1.0), true).unwrap();
        g.add_variable("local", PinType::Int, Value::Int(7), false).unwrap();
        let text = g.to_ron().unwrap();
        let mut scene = Scene::new_default("Teste", 1);
        let root = scene.roots()[0];
        let node = scene.add_node(Some(root), "Node2D", None).unwrap();
        scene.set_prop(node, "script_vars", values, None).unwrap();
        (scene, node, text)
    }

    #[test]
    fn exported_value_of_the_node_starts_the_instance() {
        let (scene, node, text) = scene_with_node(json!({"vida": {"Float": 4.0}}));
        let inst = Instancia::novo(&scene, node, &text).unwrap();
        // The exported variable takes the node's value; the local one keeps the script's default.
        assert_eq!(inst.contexto.vars, vec![Value::Float(4.0), Value::Int(7)]);
    }

    #[test]
    fn node_without_exported_values_starts_at_the_default() {
        let (scene, node, text) = scene_with_node(json!({}));
        let inst = Instancia::novo(&scene, node, &text).unwrap();
        assert_eq!(inst.contexto.vars, vec![Value::Float(1.0), Value::Int(7)]);
    }

    #[test]
    fn malformed_exported_value_refuses_the_play() {
        let (scene, node, text) = scene_with_node(json!({"vida": {"Float": "quatro"}}));
        let texts = BTreeMap::from([(node, text)]);
        let error = iniciar(&scene, &texts).err().expect("a malformed value must refuse the play");
        assert!(error.contains("Script de 'Node2D'"), "{error}");
        assert!(error.contains("vida"), "{error}");
    }

    #[test]
    fn script_reads_the_node_value_on_each_frame() {
        // OnProcess copies "vida" into "copia": the VM must read the node's value, not the script default (VIS-022).
        let mut g = Graph::default();
        g.add_variable("vida", PinType::Float, Value::Float(1.0), true).unwrap();
        g.add_variable("copia", PinType::Float, Value::Float(0.0), false).unwrap();
        let ev = adiciona(&mut g, "OnProcess");
        let get = adiciona(&mut g, "GetVariable");
        g.set_value(get, "name", Value::String("vida".into())).unwrap();
        let set = adiciona(&mut g, "SetVariable");
        g.set_value(set, "name", Value::String("copia".into())).unwrap();
        g.connect((ev, "exec"), (set, "exec")).unwrap();
        g.connect((get, "value"), (set, "value")).unwrap();
        let text = g.to_ron().unwrap();
        let mut c = Cenario::novo();
        // The test world starts empty: the node is a root.
        let node = c.cena.add_node(None, "Node2D", None).unwrap();
        c.cena.set_prop(node, "script_vars", json!({"vida": {"Float": 4.0}}), None).unwrap();
        let mut inst = Instancia::novo(&c.cena, node, &text).unwrap();
        quadro_de(&mut inst, &mut c, 0).unwrap();
        assert_eq!(inst.contexto.vars[1], Value::Float(4.0));
    }

    /// A play over `scene`, with the physics and timers that `play_start` builds, and no scripts yet.
    fn play_of(scene: Scene) -> Play {
        let physics = Physics::build(&scene, &ProjectSettings::default());
        Play {
            scene,
            time: 0.0,
            physics,
            audio: AudioPlayback::default(),
            timers: nexo_core::timer::Timers::default(),
            instancias: Vec::new(),
            editor_physics: Vec::new(),
            editor_timers: Vec::new(),
        }
    }

    /// Graph with one event `kind` that prints `msg`. The event's filter goes in the pin `pin`, when there is one.
    fn graph_with_filter(kind: &str, pin: &str, filter: &str, msg: &str) -> Graph {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, kind);
        if !pin.is_empty() {
            g.set_value(ev, pin, Value::String(filter.into())).unwrap();
        }
        let pr = adiciona(&mut g, "Print");
        g.set_value(pr, "msg", Value::String(msg.into())).unwrap();
        g.connect((ev, "exec"), (pr, "exec")).unwrap();
        g
    }

    /// Messages printed to the console by the script of `owner`.
    fn texts_of(console: &[Mensagem], owner: Uid) -> Vec<String> {
        console.iter().filter(|m| m.node == owner).map(|m| m.message.clone()).collect()
    }

    #[test]
    fn timeout_runs_the_method_connected_to_the_target() {
        // Timer -> target: the connection calls "on_timeout" on the target's script (SCN-034, VIS-002).
        let mut scene = Scene::new("Teste", 1);
        let timer = scene.add_node(None, "Timer", None).unwrap();
        let target = scene.add_node(None, "Node2D", None).unwrap();
        scene.connect(timer, "timeout", target, "on_timeout").unwrap();
        let text = graph_with_filter("OnSignal", "method", "on_timeout", "esgotou").to_ron().unwrap();
        let mut play = play_of(scene);
        play.instancias.push(Instancia::novo(&play.scene, target, &text).unwrap());
        assert!(play.timers.start(&play.scene, timer));
        let (settings, input, mut console) = (ProjectSettings::default(), Input::default(), Vec::new());
        // The default wait is 1 s: a frame of 1.5 s fires the timeout.
        play.timers.advance(&play.scene, 1.5);
        tick(&mut play, 0, &settings, &input, Ponteiro::default(), &mut console, 1.5);
        assert_eq!(texts_of(&console, target), ["esgotou"]);
        // The raw event stays in the queue for the editor.
        assert_eq!(play.editor_timers.len(), 1);
    }

    #[test]
    fn timer_runs_the_on_timer_of_its_own_script() {
        let mut scene = Scene::new("Teste", 1);
        let timer = scene.add_node(None, "Timer", None).unwrap();
        let text = graph_with_filter("OnTimer", "", "", "tic").to_ron().unwrap();
        let mut play = play_of(scene);
        play.instancias.push(Instancia::novo(&play.scene, timer, &text).unwrap());
        assert!(play.timers.start(&play.scene, timer));
        let (settings, input, mut console) = (ProjectSettings::default(), Input::default(), Vec::new());
        play.timers.advance(&play.scene, 1.5);
        tick(&mut play, 0, &settings, &input, Ponteiro::default(), &mut console, 1.5);
        assert_eq!(texts_of(&console, timer), ["tic"]);
    }

    #[test]
    fn collision_routes_on_collision_with_the_other_name_and_the_connected_method() {
        let mut scene = Scene::new("Teste", 1);
        let body = scene.add_node(None, "RigidBody2D", None).unwrap();
        let wall = scene.add_node(None, "StaticBody2D", None).unwrap();
        scene.rename_node(wall, "Parede").unwrap();
        let target = scene.add_node(None, "Node2D", None).unwrap();
        scene.connect(body, "collision_started", target, "on_hit").unwrap();
        let ev = PhysicsEvent { kind: EventKind::CollisionStarted, node: body, other: wall, point: None, normal: None };
        let routed = route_events(&scene, &[ev], &[]);
        assert!(routed.contains(&(body, Evento::dispatch("OnCollision", "Parede", wall.0))));
        assert!(routed.contains(&(target, Evento::dispatch("OnSignal", "on_hit", wall.0))));
        assert_eq!(routed.len(), 2);
    }

    #[test]
    fn area_signal_routes_only_to_the_connections_of_that_signal() {
        let mut scene = Scene::new("Teste", 1);
        let area = scene.add_node(None, "Area2D", None).unwrap();
        let body = scene.add_node(None, "RigidBody2D", None).unwrap();
        let target = scene.add_node(None, "Node2D", None).unwrap();
        scene.connect(area, "body_entered", target, "on_enter").unwrap();
        scene.connect(area, "body_exited", target, "on_exit").unwrap();
        let ev = PhysicsEvent { kind: EventKind::BodyEntered, node: area, other: body, point: None, normal: None };
        let routed = route_events(&scene, &[ev], &[]);
        // Only the entered connection fires; the exit one stays quiet, and an area has no "On Collision".
        assert_eq!(routed, vec![(target, Evento::dispatch("OnSignal", "on_enter", body.0))]);
    }

    #[test]
    fn pressed_action_runs_the_on_input_action_of_that_name() {
        let text = graph_with_filter("OnInputAction", "action", "jump", "pulou").to_ron().unwrap();
        let mut scene = Scene::new("Teste", 1);
        let node = scene.add_node(None, "Node2D", None).unwrap();
        let mut play = play_of(scene);
        play.instancias.push(Instancia::novo(&play.scene, node, &text).unwrap());
        let mut settings = ProjectSettings::default();
        settings.input.bind("jump", "Space");
        let mut input = Input::default();
        let mut console = Vec::new();
        tick(&mut play, 0, &settings, &input, Ponteiro::default(), &mut console, 0.1);
        assert!(texts_of(&console, node).is_empty());
        input.set_key("Space", true);
        tick(&mut play, 0, &settings, &input, Ponteiro::default(), &mut console, 0.1);
        assert_eq!(texts_of(&console, node), ["pulou"]);
    }
}
