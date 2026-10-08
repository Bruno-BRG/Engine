// Reprodução no editor (play-in-editor). O comportamento vive no Rust: o editor cria uma cópia
// da cena, avança o relógio e a física, e recebe as teclas. Aqui ficam o relógio de quadros, o
// retrato para o Depurador, a entrada do teclado e os eventos de física para o Console.
import { scene, type NodeView } from './scene.svelte';
import { log } from './log.svelte';
import { audio } from './audio.svelte';
import type { AudioCommand } from '$lib/audio/types';

/** Evento de física como o Rust o envia (PHY-005). `node` recebe o evento; `other` causou. */
export interface PhysicsEvent {
  kind: 'body_entered' | 'body_exited' | 'area_entered' | 'area_exited' | 'collision_started' | 'collision_ended';
  node: string;
  other: string;
}

/** Teclas com ação padrão do navegador (rolar a página): bloqueadas durante a reprodução. */
const SCROLL_KEYS = new Set(['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Space']);

class PlayRuntime {
  /** Nós da cópia de reprodução, atualizados algumas vezes por segundo para o Depurador. */
  live = $state.raw<NodeView[]>([]);
  #sinceLive = 0;

  start() {
    const ed = scene.editor;
    if (!ed) return;
    ed.play_start();
    ed.release_keys();
    audio.beginPlay();
    // Os players com autoplay já estão na fila: começam neste mesmo clique.
    this.#drainAudio();
    this.#sinceLive = 0;
    this.syncLive();
    window.addEventListener('keydown', this.#keyDown, true);
    window.addEventListener('keyup', this.#keyUp, true);
    window.addEventListener('blur', this.#blur);
  }

  /** Avança a reprodução em `dt` segundos. */
  step(dt: number) {
    const ed = scene.editor;
    if (!ed) return;
    ed.play_tick(dt);
    this.#drainEvents();
    this.#drainAudio();
    this.#sinceLive += dt;
    if (this.#sinceLive > 0.1) {
      this.#sinceLive = 0;
      this.syncLive();
    }
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
  };
}

export const runtime = new PlayRuntime();
