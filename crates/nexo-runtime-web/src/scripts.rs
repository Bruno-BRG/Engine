//! Scripts visuais na reprodução (VIS-023, CON-004, VIS-031). Cada nó com grafo ganha uma instância: o grafo compilado,
//! um agendador, um contexto e a fila dos eventos do quadro. `set_scripts` guarda os grafos (RON) por UID de nó.
//! `play_start` os compila, e cada quadro (`tick`) roda os eventos pelo Host (`host.rs`). Na reprodução, um
//! `set_scripts` recarrega a quente os grafos alterados: as variáveis de mesmo nome e tipo mantêm o valor
//! (`Scheduler::recarregar`).
//!
//! Depuração por instância (VIS-031): uma pausa (ponto de parada, ou passo a passo) congela só essa instância. Os
//! eventos do quadro que ela ainda não rodou ficam na fila dela, sem tarefas novas enquanto ela estiver parada. Com
//! `continuar`, o resto do quadro roda uma vez; com `passo`, roda uma operação e para antes da seguinte, mesmo que
//! ela esteja no evento seguinte ou no quadro seguinte. As outras instâncias seguem.

use crate::editor::{Play, SceneEditor, js_err};
use crate::host::{Mensagem, Mundo, Ponteiro};
use nexo_core::{Input, ProjectSettings, Scene, Uid};
use nexo_graph::{Compiled, Context, Graph, Host, Pausa, Scheduler, TraceEvent, VmError, compile};
use nexo_physics::STEP;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use wasm_bindgen::prelude::*;

/// Evento de quadro que ainda não começou: o nome dele e o tempo que recebe.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Evento {
    nome: &'static str,
    dt: f64,
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
    /// Eventos do quadro que ainda não começaram. Uma pausa os guarda, e `continuar` ou `passo` os retomam.
    fila: VecDeque<Evento>,
    /// O passo a passo chegou ao fim de um quadro: o próximo quadro começa parado, antes da primeira operação.
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
            aplicar_depuracao(&mut p.instancias, &self.pontos, self.selecionada);
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
        match Instancia::novo(uid, texto) {
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
            None => match Instancia::novo(uid, texto) {
                Ok(inst) => instancias.push(inst),
                Err(erro) => console.push(erro_no_console(cena, uid, erro)),
            },
        }
    }
}

/// Aplica a depuração às instâncias: cada uma recebe os pontos de parada do próprio UID, e só a instância escolhida
/// grava o rastro (VIS-031, DBG-003). Chamada quando muda a escolha, os pontos, ou quando há instâncias novas.
pub(crate) fn aplicar_depuracao(
    instancias: &mut [Instancia],
    pontos: &BTreeMap<Uid, BTreeSet<u64>>,
    escolhida: Option<Uid>,
) {
    for inst in instancias {
        inst.agenda.definir_pontos_de_parada(pontos.get(&inst.dono).cloned().unwrap_or_default());
        inst.agenda.definir_rastro(escolhida == Some(inst.dono));
    }
}

/// Um quadro de todos os scripts da reprodução. Por instância: um evento "OnPhysicsProcess" por passo de física (com
/// o passo fixo), depois o "OnProcess" do quadro, com o estado ativo. Uma instância parada não recebe quadro novo.
/// Erro de execução vai ao console uma vez por mensagem; a instância segue.
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

/// Os eventos de um quadro, na ordem de execução: um "OnPhysicsProcess" por passo fixo de física, depois o
/// "OnProcess" com o tempo do quadro.
fn quadro(passos: usize, dt: f64) -> VecDeque<Evento> {
    let mut fila: VecDeque<Evento> = (0..passos).map(|_| Evento { nome: "OnPhysicsProcess", dt: STEP }).collect();
    fila.push_back(Evento { nome: "OnProcess", dt });
    fila
}

/// Registra um erro de execução no console, uma vez por mensagem, e descarta o resto do quadro da instância.
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
    fn novo(dono: Uid, texto: &str) -> Result<Instancia, String> {
        let grafo = compilar(texto)?;
        let mut agenda = Scheduler::new();
        agenda.iniciar(&grafo, "OnReady");
        let contexto = grafo.new_context(dono.0);
        Ok(Instancia {
            dono,
            texto: texto.to_string(),
            grafo,
            agenda,
            contexto,
            erros: BTreeSet::new(),
            fila: VecDeque::new(),
            aguarda_quadro: false,
        })
    }

    /// Recarga a quente: troca o grafo e mantém as variáveis de mesmo nome e tipo (`Scheduler::recarregar`). As
    /// tarefas em curso e a pausa são descartadas, junto com os eventos que faltavam no quadro. Um estado que deixou
    /// de existir no grafo novo volta ao estado inicial.
    fn recarregar(&mut self, novo: Compiled, texto: &str) {
        self.contexto = self.agenda.recarregar(&self.contexto, &self.grafo, &novo);
        self.grafo = novo;
        self.texto = texto.to_string();
        self.erros.clear();
        self.fila.clear();
        self.aguarda_quadro = false;
        let estado = self.contexto.estado.clone();
        if !estado.is_empty() && !self.grafo.programs.iter().any(|p| p.estado.as_deref() == Some(estado.as_str())) {
            self.contexto.estado = self.grafo.estado_inicial.clone().unwrap_or_default();
        }
    }

    /// Pausada para depuração: a agenda parou numa operação, ou o passo chegou ao fim de um quadro. Uma instância
    /// pausada não recebe quadros novos até `continuar` ou `passo`.
    pub(crate) fn pausada(&self) -> bool {
        self.agenda.pausa().is_some() || self.aguarda_quadro
    }

    /// Onde a execução parou, se parou numa operação (ponto de parada ou passo a passo).
    pub(crate) fn pausa(&self) -> Option<&Pausa> {
        self.agenda.pausa()
    }

    /// Escolhe o que a instância faz no quadro: se ela está parada, espera; se o passo a passo chegou ao fim de um
    /// quadro, o quadro novo começa parado; senão, roda o quadro inteiro até a próxima pausa.
    fn quadro_novo(&mut self, host: &mut dyn Host, passos: usize, dt: f64) -> Result<(), VmError> {
        if self.aguarda_quadro {
            self.aguarda_quadro = false;
            self.fila = quadro(passos, dt);
            return self.avancar_fila(host, true);
        }
        if self.pausada() {
            return Ok(());
        }
        self.fila = quadro(passos, dt);
        self.avancar_fila(host, false)
    }

    /// Começa o evento (cria as tarefas dele) e roda até a próxima pausa ou o fim dele. `parado`: para antes da
    /// primeira operação, em vez de rodar (passo a passo).
    fn comecar(&mut self, evento: Evento, host: &mut dyn Host, parado: bool) -> Result<(), VmError> {
        self.agenda.iniciar(&self.grafo, evento.nome);
        if evento.nome == "OnProcess" {
            self.agenda.iniciar_estado(&self.grafo, &self.contexto.estado);
        }
        if parado {
            self.agenda.parar_antes(&self.grafo, &mut self.contexto, host, evento.dt)
        } else {
            self.agenda.tick(&self.grafo, &mut self.contexto, host, evento.dt)
        }
    }

    /// Começa os eventos da fila, um por vez, até uma pausa ou até a fila acabar. Com `parado`, a primeira operação
    /// de cada evento pausa, e o evento seguinte só começa depois que a operação dele roda.
    fn avancar_fila(&mut self, host: &mut dyn Host, parado: bool) -> Result<(), VmError> {
        while self.agenda.pausa().is_none() {
            let Some(evento) = self.fila.pop_front() else {
                break;
            };
            self.comecar(evento, host, parado)?;
        }
        Ok(())
    }

    /// Continuar (VIS-031): sai da pausa, roda o resto do quadro uma vez e os quadros seguintes até a próxima pausa.
    pub(crate) fn continuar(&mut self, host: &mut dyn Host) -> Result<(), VmError> {
        if !self.pausada() {
            return Ok(());
        }
        self.aguarda_quadro = false;
        self.agenda.continuar(&self.grafo, &mut self.contexto, host)?;
        self.avancar_fila(host, false)
    }

    /// Próxima operação (VIS-031): roda uma operação e pausa antes da seguinte, mesmo que ela esteja no evento
    /// seguinte. Se o quadro acaba, o próximo começa parado no quadro seguinte. Sem pausa, não faz nada.
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

    /// Tira o rastro da instância desde a última vez (VIS-030, DBG-003). Vazio se o rastro está desligado.
    pub(crate) fn tomar_rastro(&mut self) -> Vec<TraceEvent> {
        self.agenda.tomar_rastro()
    }
}

/// Lê e compila um grafo. O erro junta as mensagens de leitura ou de validação, cada uma com o nó onde está.
fn compilar(texto: &str) -> Result<Compiled, String> {
    let grafo = Graph::from_ron(texto)?;
    compile(&grafo).map_err(|erros| erros.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; "))
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
    use nexo_graph::Value;
    use nexo_physics::Physics;

    /// Mundo de teste: uma cena vazia, com a física e o áudio que os prints não usam.
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

        /// Textos que o script de `dono` imprimiu, na ordem.
        fn textos(&self, dono: Uid) -> Vec<String> {
            self.console.iter().filter(|m| m.node == dono).map(|m| m.message.clone()).collect()
        }
    }

    /// Roda `acao` com o mundo de teste do script `dono`.
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

    /// Um quadro da instância, com `passos` passos de física.
    fn quadro_de(inst: &mut Instancia, c: &mut Cenario, passos: usize) -> Result<(), VmError> {
        let dono = inst.dono;
        com_mundo(c, dono, |m| inst.quadro_novo(m, passos, 0.1))
    }

    fn adiciona(g: &mut Graph, kind: &str) -> u64 {
        g.add_node(kind, [0.0, 0.0]).unwrap()
    }

    /// Grafo com dois eventos: "OnPhysicsProcess" imprime "fis" e "OnProcess" imprime "proc". Devolve os nós dos
    /// dois prints. Cada print tem duas operações (a constante da mensagem e a escrita), então um passo nem sempre
    /// chega à escrita.
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
        Instancia::novo(Uid(dono), &g.to_ron().unwrap()).unwrap()
    }

    /// Próxima operação até `pare` ser verdadeiro, com no máximo dez passos.
    fn passa_ate(a: &mut Instancia, c: &mut Cenario, pare: impl Fn(&Instancia) -> bool) {
        for _ in 0..10 {
            if pare(&*a) {
                return;
            }
            let dono = a.dono;
            com_mundo(c, dono, |m| a.passo(m)).unwrap();
        }
    }

    /// Ponto de parada no print de "proc" da instância `a` (só ela).
    fn com_ponto_em(a: &mut Instancia, proc: u64) {
        let pontos = BTreeMap::from([(a.dono, BTreeSet::from([proc]))]);
        aplicar_depuracao(std::slice::from_mut(a), &pontos, None);
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

        // Depois de continuar, o quadro seguinte roda de novo até o ponto de parada.
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
        // Quadro 1: o "fis" roda e a execução para no "proc" (ponto de parada).
        quadro_de(&mut a, &mut c, 1).unwrap();
        assert_eq!(a.pausa().map(|p| p.node), Some(proc));
        assert_eq!(c.textos(Uid(1)), ["fis"]);

        // Próxima operação até o fim do quadro: "proc" imprime e a instância espera o quadro seguinte.
        passa_ate(&mut a, &mut c, |i| i.pausa().is_none());
        assert_eq!(c.textos(Uid(1)), ["fis", "proc"]);
        assert!(a.pausada() && a.pausa().is_none(), "fim de quadro: espera o quadro seguinte");

        // Quadro 2 começa parado no primeiro evento, antes de qualquer operação dele.
        quadro_de(&mut a, &mut c, 1).unwrap();
        assert_eq!(a.pausa().map(|p| p.node), Some(fis));
        assert_eq!(c.textos(Uid(1)), ["fis", "proc"]);

        // Próxima operação até o "proc" do quadro 2: o "fis" imprime e a pausa cai no começo do "proc".
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
    fn rastro_so_grava_na_instancia_escolhida() {
        let (g, _, _) = grafo_de_quadro();
        let mut todas = vec![instancia(1, &g), instancia(2, &g)];
        aplicar_depuracao(&mut todas, &BTreeMap::new(), Some(Uid(2)));
        let mut c = Cenario::novo();
        for inst in &mut todas {
            quadro_de(inst, &mut c, 0).unwrap();
        }

        assert!(todas[0].tomar_rastro().is_empty());
        assert!(!todas[1].tomar_rastro().is_empty());
    }
}
