// Reprodução no editor (play-in-editor). O comportamento vive no Rust: o editor cria uma cópia
// da cena, avança o relógio, a física e os scripts, e recebe as teclas. Aqui ficam o relógio de
// quadros, o retrato para o Depurador, a entrada do teclado, os eventos de física e as mensagens
// dos scripts para o Console, e a entrega do áudio ao motor.
import { scene, type NodeView } from './scene.svelte';
import { log } from './log.svelte';
import { audio } from './audio.svelte';
import { logic } from '$lib/logic/store.svelte';
import { logicDebug, type InstanciaDebug } from '$lib/logic/debug.svelte';
import type { Trace } from '$lib/logic/model';
import type { AudioCommand } from '$lib/audio/types';
import { pollGamepad, resetGamepad, syncMouse } from '$lib/runtime/devices';
import { settingsUi } from '$lib/state/settings.svelte';

/** Evento de física como o Rust o envia (PHY-005). `node` recebe o evento; `other` causou. */
export interface PhysicsEvent {
  kind: 'body_entered' | 'body_exited' | 'area_entered' | 'area_exited' | 'collision_started' | 'collision_ended';
  node: string;
  other: string;
}

/** Mensagem do console que um script escreveu (CON-004), como o Rust a envia. `node` é o nó dono do script. */
interface ConsoleMessage {
  level: 'info' | 'warning' | 'error';
  node: string;
  message: string;
}

/** Teclas com ação padrão do navegador (rolar a página): bloqueadas durante a reprodução. */
const SCROLL_KEYS = new Set(['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Space']);

/** Seconds between two reads of the debugger (state and trace): 10 Hz, not every frame (VIS-030). */
const DEBUG_READ_S = 0.1;

class PlayRuntime {
  /** Nós da cópia de reprodução, atualizados algumas vezes por segundo para o Depurador. */
  live = $state.raw<NodeView[]>([]);
  #sinceLive = 0;
  #sinceDebug = 0;
  /** Last debugger state text read from the play session, so an unchanged state is not parsed or assigned again. */
  #debugState: string | null = null;
  /** Breakpoints as last sent to the play session, so unchanged ones are not sent again. */
  #pontosEnviados: string | null = null;

  constructor() {
    // A breakpoint set or removed takes effect at once: the play session gets them now, even without frames.
    logicDebug.onPontosMudaram = () => this.#sincronizarPontos();
  }

  /** Começa a reprodução. Devolve false se o jogo não começou: um script não compila ou não pode ser lido, e o erro vai ao Console. */
  async start(): Promise<boolean> {
    const ed = scene.editor;
    if (!ed) return false;
    try {
      ed.set_scripts(JSON.stringify(await this.#scriptsForPlay()));
      this.#pontosEnviados = null;
      this.#sincronizarPontos();
      // The editor may have been created after the panel opened: the play session gets the panel state it must use.
      ed.debug_panel(logicDebug.painelAberto);
      ed.play_start();
    } catch (e) {
      log.error('O jogo não começou: ' + (e instanceof Error ? e.message : String(e)));
      return false;
    }
    ed.release_keys();
    resetGamepad();
    audio.beginPlay();
    logicDebug.limpar();
    // The new play session follows no instance until the debugger chooses one (it must not keep the old choice).
    this.escolherInstancia(null);
    // Os players com autoplay já estão na fila: começam neste mesmo clique.
    this.#drainAudio();
    this.drainConsole();
    this.#sinceLive = 0;
    this.#sinceDebug = 0;
    this.#debugState = null;
    this.syncLive();
    this.#atualizarDepurador();
    window.addEventListener('keydown', this.#keyDown, true);
    window.addEventListener('keyup', this.#keyUp, true);
    window.addEventListener('blur', this.#blur);
    return true;
  }

  /** Avança a reprodução em `dt` segundos. */
  step(dt: number) {
    const ed = scene.editor;
    if (!ed) return;
    // While a binding capture waits for a gamepad press, the controller does not reach the game.
    pollGamepad(ed, !settingsUi.capture);
    ed.play_tick(dt);
    this.#drainEvents();
    this.#drainAudio();
    this.drainConsole();
    this.#sinceDebug += dt;
    if (this.#sinceDebug >= DEBUG_READ_S) {
      this.#sinceDebug = 0;
      this.#atualizarDepurador();
    }
    this.#sinceLive += dt;
    if (this.#sinceLive > 0.1) {
      this.#sinceLive = 0;
      this.syncLive();
    }
  }

  /** Chooses the instance the debugger follows. Only it records a trace in the play session. */
  escolherInstancia(uid: string | null) {
    logicDebug.select(uid);
    try {
      scene.editor?.debug_select(uid ?? '');
    } catch (e) {
      log.error('Depurador: ' + (e instanceof Error ? e.message : String(e)));
    }
  }

  /**
   * Tells the runtime whether the Depurador panel is open. While it is open, the followed instance records its trace and
   * each frame reads the debugger state (DBG-003). The panel calls it on mount and unmount.
   */
  definirPainelDepurador(aberto: boolean) {
    logicDebug.painelAberto = aberto;
    try {
      scene.editor?.debug_panel(aberto);
    } catch (e) {
      log.error('Depurador: ' + (e instanceof Error ? e.message : String(e)));
    }
    if (aberto) this.#atualizarDepurador();
  }

  /** Continue, or Next operation, for a paused instance (VIS-031). Only that instance changes; the game keeps running. */
  depurar(acao: 'continuar' | 'proxima', uid: string) {
    const ed = scene.editor;
    if (!ed?.is_playing()) return;
    try {
      if (acao === 'continuar') ed.debug_continue(uid);
      else ed.debug_step(uid);
    } catch (e) {
      log.error('Depurador: ' + (e instanceof Error ? e.message : String(e)));
    }
    this.#drainEvents();
    this.#drainAudio();
    this.drainConsole();
    this.#atualizarDepurador();
    this.syncLive();
  }

  /**
   * State of the instances and the trace of the followed one, read only while the debugger is observing. `step` calls
   * it at 10 Hz (DEBUG_READ_S), and the debugger's actions call it at once. The state is parsed only when its text
   * changed.
   */
  #atualizarDepurador() {
    const ed = scene.editor;
    if (!ed || !ed.is_playing()) return;
    if (!logicDebug.observando) {
      // Nothing observes the play session now: the lit highlight must not stay on the canvas.
      logicDebug.clearHighlight();
      return;
    }
    const state = ed.debug_state();
    if (state !== this.#debugState) {
      this.#debugState = state;
      logicDebug.setInstancias(JSON.parse(state) as InstanciaDebug[]);
    }
    // No valid choice (game start, or the instance is gone): the first paused one, or the first.
    if (!logicDebug.instancias.some((i) => i.uid === logicDebug.selecionada)) {
      this.escolherInstancia((logicDebug.instancias.find((i) => i.pausada) ?? logicDebug.instancias[0])?.uid ?? null);
    }
    const uid = logicDebug.selecionada;
    if (uid) {
      // Read with the state. A read with nothing new still ages the highlight (VIS-030).
      const trace = JSON.parse(ed.debug_trace()) as Trace;
      logicDebug.pushTrace(uid, trace, performance.now());
    }
  }

  /**
   * Sends the breakpoints to the play session: each node with a script gets the breakpoints of its script. Sends them
   * only when they change. They hold in the current play session and in the next ones (VIS-031).
   */
  #sincronizarPontos() {
    const ed = scene.editor;
    if (!ed) return;
    const porUid: Record<string, number[]> = {};
    for (const node of scene.data.nodes) {
      const nome = node.props.script;
      if (typeof nome !== 'string' || !nome) continue;
      const ids = logicDebug.breakpointsOf(nome);
      if (ids.length) porUid[node.uid] = ids;
    }
    const json = JSON.stringify(porUid);
    if (json === this.#pontosEnviados) return;
    try {
      ed.set_breakpoints(json);
      this.#pontosEnviados = json;
    } catch (e) {
      log.error('Pontos de parada: ' + (e instanceof Error ? e.message : String(e)));
    }
  }

  /** Botões do mouse e do toque no viewport de jogo. `buttons` é o `PointerEvent.buttons` do evento. */
  pointerButtons(buttons: number) {
    const ed = scene.editor;
    if (ed?.is_playing()) syncMouse(ed, buttons);
  }

  /** Atualiza o Depurador agora, sem esperar o intervalo de 0,1 s (usado pelo Passo a passo). */
  syncLive() {
    const ed = scene.editor;
    if (!ed || !ed.is_playing()) return;
    this.#sinceLive = 0;
    this.live = (JSON.parse(ed.outline() as unknown as string) as { nodes: NodeView[] }).nodes;
  }

  get time() {
    return scene.editor?.play_time() ?? 0;
  }

  stop() {
    window.removeEventListener('keydown', this.#keyDown, true);
    window.removeEventListener('keyup', this.#keyUp, true);
    window.removeEventListener('blur', this.#blur);
    scene.editor?.release_keys();
    resetGamepad();
    scene.editor?.play_stop();
    audio.endPlay();
    this.live = [];
    logicDebug.limpar();
    this.escolherInstancia(null);
  }

  /** Entrega ao motor de áudio os comandos do quadro (contrato em crates/nexo-core/src/audio.rs). */
  #drainAudio() {
    const ed = scene.editor;
    if (!ed) return;
    audio.consume(JSON.parse(ed.audio_commands()) as AudioCommand[]);
  }

  /** Grafos dos scripts anexados aos nós (VIS-023), como `{uid do nó: texto RON}`, para `set_scripts`. */
  async #scriptsForPlay(): Promise<Record<string, string>> {
    const nomePorNo = new Map<string, string>();
    for (const node of scene.data.nodes) {
      const nome = node.props.script;
      if (typeof nome === 'string' && nome) nomePorNo.set(node.uid, nome);
    }
    const textos = await logic.textsForPlay([...new Set(nomePorNo.values())]);
    const porNo: Record<string, string> = {};
    for (const [uid, nome] of nomePorNo) {
      const texto = textos.get(nome);
      if (texto !== undefined) porNo[uid] = texto;
    }
    return porNo;
  }

  /**
   * Leva ao Console as mensagens dos scripts (CON-004). Chamado a cada quadro e depois de `set_scripts`, que pode
   * registrar erros de recarga mesmo com o jogo parado.
   */
  drainConsole() {
    const ed = scene.editor;
    if (!ed) return;
    const mensagens = JSON.parse(ed.console_messages()) as ConsoleMessage[];
    for (const m of mensagens) {
      const opts = { nodeUid: m.node };
      if (m.level === 'error') log.error(m.message, opts);
      else if (m.level === 'warning') log.warn(m.message, opts);
      else log.info(m.message, opts);
    }
  }

  /** Leva ao Console as entradas e saídas de áreas. Contatos entre corpos ficam para a lógica visual. */
  #drainEvents() {
    const ed = scene.editor;
    if (!ed) return;
    // Sinais de timeout (SCN-014): a lógica visual os lerá quando for conectada. Por ora a fila só é esvaziada.
    ed.timer_events();
    const events = JSON.parse(ed.physics_events() as unknown as string) as PhysicsEvent[];
    for (const ev of events) {
      const node = scene.byUid.get(ev.node)?.name ?? ev.node;
      const other = scene.byUid.get(ev.other)?.name ?? ev.other;
      if (ev.kind === 'body_entered' || ev.kind === 'area_entered') {
        log.info(`${other} entrou em ${node}`, { nodeUid: ev.node });
      } else if (ev.kind === 'body_exited' || ev.kind === 'area_exited') {
        log.info(`${other} saiu de ${node}`, { nodeUid: ev.node });
      }
    }
  }

  #typing(e: KeyboardEvent) {
    const t = e.target as HTMLElement | null;
    return !!t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.tagName === 'SELECT' || t.isContentEditable);
  }

  #keyDown = (e: KeyboardEvent) => {
    const ed = scene.editor;
    if (!ed || !ed.is_playing() || this.#typing(e)) return;
    if (SCROLL_KEYS.has(e.code)) e.preventDefault();
    ed.key_event(e.code, true);
  };

  #keyUp = (e: KeyboardEvent) => {
    scene.editor?.key_event(e.code, false);
  };

  #blur = () => {
    scene.editor?.release_keys();
    resetGamepad();
  };
}

export const runtime = new PlayRuntime();
