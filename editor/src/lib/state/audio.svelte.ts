// Estado do áudio no editor: o motor WebAudio, o solo e a pré-visualização (AUD-002, AUD-003).
// Os barramentos vêm de projectSettings. Cada mudança de volume, mudo ou efeito é gravada no projeto
// (markSettingsDirty) e vai para o motor. O solo fica só nesta sessão e não entra no arquivo.
import { AudioEngine } from '$lib/audio/engine';
import { PREVIEW_KEY, type AudioCommand, type BusConfig, type BusEffects } from '$lib/audio/types';
import { markSettingsDirty, projectSettings } from '$lib/state/settings.svelte';
import type { Bus } from '$lib/settings';
import { assets } from './assets.svelte';
import { log } from './log.svelte';
import { scene } from './scene.svelte';

/** Faixa do fader do mixer, em dB. Abaixo de -80 é silêncio. */
export const VOLUME_MIN_DB = -80;
export const VOLUME_MAX_DB = 6;

const clampVolume = (db: number) => Math.min(VOLUME_MAX_DB, Math.max(VOLUME_MIN_DB, db));

class AudioState {
  /** Barramentos em solo. Vale só nesta sessão. */
  solo = $state<string[]>([]);
  /** UID do asset que toca na pré-visualização, ou null. */
  previewUid = $state<string | null>(null);

  readonly engine = new AudioEngine({
    loadBytes: (uid) => this.#bytes(uid),
    onError: (message) => log.error(message),
    onFinished: (key) => {
      if (key === PREVIEW_KEY) {
        this.previewUid = null;
        return;
      }
      // O núcleo guarda quais nós estão tocando: avisa que este acabou, para um Play sem reinício tocar de novo.
      scene.editor?.audio_finished(key);
    },
  });

  /** Barramentos do projeto como o motor os recebe: o solo dos outros canais já aplicado. */
  get configs(): BusConfig[] {
    const soloing = this.solo.length > 0;
    return projectSettings.buses.map((b) => ({
      name: b.name,
      volumeDb: b.volumeDb,
      mute: b.mute,
      silencedBySolo: soloing && !this.solo.includes(b.name),
      effects: b.effects,
    }));
  }

  isSolo(name: string): boolean {
    return this.solo.includes(name);
  }

  /** Volume de um barramento, em dB. Grava no projeto. */
  setVolume(name: string, db: number): void {
    const bus = this.#bus(name);
    if (!bus) return;
    bus.volumeDb = clampVolume(db);
    this.#changed();
  }

  toggleMute(name: string): void {
    const bus = this.#bus(name);
    if (!bus) return;
    bus.mute = !bus.mute;
    this.#changed();
  }

  /** Solo: os outros canais ficam mudos enquanto algum estiver em solo. Não grava no projeto. */
  toggleSolo(name: string): void {
    this.solo = this.solo.includes(name) ? this.solo.filter((n) => n !== name) : [...this.solo, name];
    this.#sync();
  }

  /** Edita os efeitos de um barramento. `edit` altera o objeto no lugar. Grava no projeto. */
  editEffects(name: string, edit: (fx: BusEffects) => void): void {
    const bus = this.#bus(name);
    if (!bus) return;
    edit(bus.effects);
    this.#changed();
  }

  /** Garante o contexto de áudio, com os barramentos atuais. Chamar a partir de um clique. */
  ready(): boolean {
    this.#sync();
    return this.engine.ensureContext() !== null;
  }

  /** Início da reprodução: prepara o motor com os barramentos do projeto. */
  beginPlay(): void {
    this.#sync();
    this.engine.beginSession();
  }

  /** Comandos de um quadro da reprodução. */
  consume(commands: AudioCommand[]): void {
    if (commands.length > 0) this.engine.consume(commands);
  }

  /** Fim da reprodução: todo som para. A pré-visualização também para. */
  endPlay(): void {
    this.engine.endSession();
    this.previewUid = null;
  }

  setPaused(paused: boolean): void {
    this.engine.setPaused(paused);
  }

  /** Liga ou desliga a pré-visualização de um asset de áudio. */
  togglePreview(uid: string): void {
    if (this.previewUid === uid) {
      this.engine.stopPreview();
      this.previewUid = null;
      return;
    }
    if (!this.ready()) return;
    this.engine.stopPreview();
    this.previewUid = uid;
    this.engine.preview(uid);
  }

  #bus(name: string): Bus | undefined {
    return projectSettings.buses.find((b) => b.name === name);
  }

  #changed(): void {
    markSettingsDirty();
    this.#sync();
  }

  #sync(): void {
    this.engine.applyBuses(this.configs);
  }

  #bytes(uid: string): Promise<ArrayBuffer> {
    const ref = assets.byUid(uid);
    if (!ref) return Promise.reject(new Error('o arquivo não está mais no projeto'));
    return assets.readBytes(ref.rel);
  }
}

export const audio = new AudioState();

// Acompanha o projeto: trocar de projeto esquece o áudio decodificado, e um arquivo alterado é lido de novo.
assets.subscribe((ev) => {
  if (ev.type === 'project') {
    audio.engine.stopAll();
    audio.engine.clearBuffers();
    audio.previewUid = null;
    return;
  }
  for (const rel of ev.paths) {
    const ref = assets.byRel(rel);
    if (ref?.kind === 'audio') audio.engine.forget(ref.uid);
  }
});
