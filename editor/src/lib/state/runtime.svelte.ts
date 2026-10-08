// Reprodução no editor (play-in-editor). O comportamento vive no Rust: o editor cria uma cópia
// da cena, avança o relógio, a física e os scripts, e recebe as teclas. Aqui ficam o relógio de
// quadros, o retrato para o Depurador, a entrada do teclado, os eventos de física e as mensagens
// dos scripts para o Console, e a entrega do áudio ao motor.
import { scene, type NodeView } from './scene.svelte';
import { log } from './log.svelte';
import { audio } from './audio.svelte';
import type { AudioCommand } from '$lib/audio/types';
import { pollGamepad, resetGamepad, syncMouse } from '$lib/runtime/devices';

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

class PlayRuntime {
  /** Nós da cópia de reprodução, atualizados algumas vezes por segundo para o Depurador. */
  live = $state.raw<NodeView[]>([]);
  #sinceLive = 0;

  /** Começa a reprodução. Devolve false se o jogo não começou: um script não compila, e o erro vai ao Console. */
  start(): boolean {
    const ed = scene.editor;
    if (!ed) return false;
    try {
      ed.play_start();
    } catch (e) {
      log.error('O jogo não começou: ' + (e instanceof Error ? e.message : String(e)));
      return false;
    }
    ed.release_keys();
    resetGamepad();
    audio.beginPlay();
    // Os players com autoplay já estão na fila: começam neste mesmo clique.
    this.#drainAudio();
    this.drainConsole();
    this.#sinceLive = 0;
    this.syncLive();
    window.addEventListener('keydown', this.#keyDown, true);
    window.addEventListener('keyup', this.#keyUp, true);
    window.addEventListener('blur', this.#blur);
    return true;
  }

  /** Avança a reprodução em `dt` segundos. */
  step(dt: number) {
    const ed = scene.editor;
    if (!ed) return;
    pollGamepad(ed);
    ed.play_tick(dt);
    this.#drainEvents();
    this.#drainAudio();
    this.drainConsole();
    this.#sinceLive += dt;
    if (this.#sinceLive > 0.1) {
      this.#sinceLive = 0;
      this.syncLive();
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
  }

  /** Entrega ao motor de áudio os comandos do quadro (contrato em crates/nexo-core/src/audio.rs). */
  #drainAudio() {
    const ed = scene.editor;
    if (!ed) return;
    audio.consume(JSON.parse(ed.audio_commands()) as AudioCommand[]);
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
