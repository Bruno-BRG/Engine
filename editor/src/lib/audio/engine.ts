// Motor de áudio do editor (WebAudio). Executa, quadro a quadro, os comandos do núcleo (AudioCommand)
// durante a reprodução, e toca as pré-visualizações do Inspector e do Projeto. Cada barramento é uma
// cadeia de ganhos com os quatro efeitos (AUD-003). Este módulo não conhece a interface: quem liga o
// motor aos assets e ao log é o estado em `$lib/state/audio.svelte`.
import type { AudioCommand, BusConfig } from './types';
import { DEFAULT_BUS, PREVIEW_KEY } from './types';
import { dbToGain, makeImpulse } from './dsp';

export interface EngineOptions {
  /** Bytes de um asset de áudio, pelo UID. Cada chamada devolve um ArrayBuffer novo. */
  loadBytes: (uid: string) => Promise<ArrayBuffer>;
  /** Falhas de áudio (arquivo ilegível, motor indisponível) para o Console. */
  onError: (message: string) => void;
  /** Um som acabou: terminou sozinho (sem repetir) ou não pôde tocar. O núcleo precisa saber disso. */
  onFinished?: (key: string) => void;
}

/** Tempo de suavização dos ganhos, em segundos. Evita cliques ao mexer em volume e mudo. */
const SMOOTH_S = 0.02;
/** Frequência do passa-baixas quando o filtro está desligado: acima da audição, sem efeito. */
const OPEN_CUTOFF_HZ = 20000;

/** Cadeia de um barramento: entrada → passa-baixas → compressor → (seco, eco, reverberação) → volume. */
interface Bus {
  input: GainNode;
  lowPass: BiquadFilterNode;
  compressor: DynamicsCompressorNode;
  delay: DelayNode;
  delayFeedback: GainNode;
  delayWet: GainNode;
  convolver: ConvolverNode;
  reverbWet: GainNode;
  /** Soma das partes seca, com eco e com reverberação. */
  mix: GainNode;
  /** Volume e mudo do barramento. Sai para a saída do sistema ou para a entrada do barramento mestre. */
  fader: GainNode;
  /** Para onde o fader está ligado agora. */
  target: AudioNode;
  /** Tempo do impulso da reverberação em uso. -1 enquanto nenhum foi gerado. */
  decay: number;
}

/** Um som tocando (ou prestes a tocar, enquanto o áudio é decodificado). */
interface Voice {
  key: string;
  /** Volume do som com o Fade. */
  gain: GainNode;
  /** Atenuação espacial de um player 2D. Fica entre o volume e o barramento. */
  attenuation: GainNode;
  /** Só existe depois da decodificação. */
  source: AudioBufferSourceNode | null;
}

/** Ajustes que comandos do núcleo fazem durante a reprodução. Somem ao parar. */
interface BusOverride {
  volumeDb?: number;
  mute?: boolean;
}

export class AudioEngine {
  readonly #opts: EngineOptions;
  #ctx: AudioContext | null = null;
  #paused = false;
  /** Barramentos como o projeto os define, com o solo já aplicado. */
  #configs: BusConfig[] = [];
  #overrides = new Map<string, BusOverride>();
  #buses = new Map<string, Bus>();
  #voices = new Map<string, Voice>();
  #buffers = new Map<string, Promise<AudioBuffer>>();

  constructor(opts: EngineOptions) {
    this.#opts = opts;
  }

  /**
   * Cria o contexto de áudio na primeira vez, com os barramentos já aplicados. Precisa de um gesto do
   * usuário (Play ou Ouvir); antes disso o navegador deixa o contexto suspenso. Devolve null se não houver
   * como criar.
   */
  ensureContext(): AudioContext | null {
    if (this.#ctx) {
      if (!this.#paused && this.#ctx.state === 'suspended') void this.#ctx.resume();
      return this.#ctx;
    }
    if (typeof AudioContext === 'undefined') {
      this.#opts.onError('Áudio: este ambiente não tem AudioContext');
      return null;
    }
    try {
      this.#ctx = new AudioContext();
    } catch (e) {
      this.#opts.onError(`Áudio: não foi possível abrir a saída de som (${messageOf(e)})`);
      return null;
    }
    if (this.#paused) void this.#ctx.suspend();
    this.#syncBuses();
    return this.#ctx;
  }

  /**
   * Recebe os barramentos do projeto (volume, mudo, solo e efeitos). Pode ser chamado antes de o contexto
   * existir: o valor fica guardado até ele ser criado. Quando o volume ou o mudo de um barramento muda aqui,
   * o ajuste feito por comando durante a reprodução é descartado.
   */
  applyBuses(configs: BusConfig[]): void {
    for (const next of configs) {
      const prev = this.#configs.find((c) => c.name === next.name);
      if (prev && (prev.volumeDb !== next.volumeDb || prev.mute !== next.mute)) this.#overrides.delete(next.name);
    }
    this.#configs = configs;
    this.#syncBuses();
  }

  /** Início de uma reprodução: zera os ajustes de comandos, retoma o áudio e prepara o contexto. */
  beginSession(): void {
    this.#overrides.clear();
    this.setPaused(false);
    this.ensureContext();
  }

  /** Fim da reprodução: todo som para, os ajustes de comandos somem e o áudio volta a rodar. */
  endSession(): void {
    this.stopAll();
    this.#overrides.clear();
    this.setPaused(false);
  }

  /** Pausa (suspende o contexto, então os sons param no ponto) ou retoma. */
  setPaused(paused: boolean): void {
    this.#paused = paused;
    if (!this.#ctx) return;
    if (paused) void this.#ctx.suspend();
    else void this.#ctx.resume();
  }

  /** Executa os comandos de um quadro, na ordem em que o núcleo os enviou. */
  consume(commands: AudioCommand[]): void {
    for (const c of commands) {
      switch (c.type) {
        case 'play':
          this.#play(c.node, c.stream, c.bus, c.volumeDb, c.pitch, c.looping, c.fromStart);
          break;
        case 'stop':
          this.stop(c.node);
          break;
        case 'fade':
          this.#fade(c.node, c.toDb, c.seconds);
          break;
        case 'set_bus_volume':
          this.#override(c.bus, { volumeDb: c.volumeDb });
          break;
        case 'set_bus_mute':
          this.#override(c.bus, { mute: c.mute });
          break;
        case 'set_attenuation':
          this.#attenuate(c.node, c.db);
          break;
      }
    }
  }

  /** Toca um asset na pré-visualização (Inspector e Projeto), pelo barramento padrão, sem repetir. */
  preview(uid: string): void {
    this.#play(PREVIEW_KEY, uid, DEFAULT_BUS, 0, 1, false, true);
  }

  /** Para a pré-visualização, se estiver tocando. */
  stopPreview(): void {
    this.stop(PREVIEW_KEY);
  }

  /** Para o som de uma chave (UID do nó, ou a pré-visualização). */
  stop(key: string): void {
    const voice = this.#voices.get(key);
    if (voice) this.#kill(voice);
  }

  stopAll(): void {
    for (const voice of [...this.#voices.values()]) this.#kill(voice);
  }

  /** Esquece o áudio decodificado de um asset (o arquivo mudou no disco). */
  forget(uid: string): void {
    this.#buffers.delete(uid);
  }

  /** Esquece todo o áudio decodificado (troca de projeto). */
  clearBuffers(): void {
    this.#buffers.clear();
  }

  // ---------- Vozes ----------

  #play(key: string, stream: string, bus: string, volumeDb: number, pitch: number, looping: boolean, fromStart: boolean) {
    const ctx = this.ensureContext();
    if (!ctx) return;
    const current = this.#voices.get(key);
    // Sem reiniciar, um som que já toca segue como está.
    if (current && !fromStart) return;
    if (current) this.#kill(current);

    const out: AudioNode = this.#busNode(bus) ?? ctx.destination;
    const gain = ctx.createGain();
    gain.gain.value = dbToGain(volumeDb);
    const attenuation = ctx.createGain();
    gain.connect(attenuation);
    attenuation.connect(out);
    const voice: Voice = { key, gain, attenuation, source: null };
    this.#voices.set(key, voice);

    this.#buffer(ctx, stream).then(
      (buffer) => {
        // Parado ou trocado enquanto decodificava: não começa.
        if (this.#voices.get(key) !== voice) return;
        const source = ctx.createBufferSource();
        source.buffer = buffer;
        source.loop = looping;
        source.playbackRate.value = pitch;
        source.connect(gain);
        source.onended = () => this.#ended(key, voice);
        source.start();
        voice.source = source;
      },
      (e: unknown) => {
        if (this.#voices.get(key) === voice) {
          this.#kill(voice);
          this.#opts.onFinished?.(key);
        }
        this.#opts.onError(`Áudio: não foi possível tocar “${stream}” (${messageOf(e)})`);
      },
    );
  }

  #kill(voice: Voice) {
    if (this.#voices.get(voice.key) === voice) this.#voices.delete(voice.key);
    if (voice.source) {
      try {
        voice.source.stop();
      } catch {
        // Já parou ou nunca começou: nada a fazer.
      }
      voice.source.disconnect();
      voice.source = null;
    }
    voice.gain.disconnect();
    voice.attenuation.disconnect();
  }

  #ended(key: string, voice: Voice) {
    // Um som parado pelo código já saiu do mapa antes, então só o fim natural chega aqui.
    if (this.#voices.get(key) !== voice) return;
    this.#kill(voice);
    this.#opts.onFinished?.(key);
  }

  #fade(key: string, toDb: number, seconds: number) {
    const voice = this.#voices.get(key);
    const ctx = this.#ctx;
    if (!voice || !ctx) return;
    const now = ctx.currentTime;
    const param = voice.gain.gain;
    const target = dbToGain(toDb);
    param.cancelScheduledValues(now);
    param.setValueAtTime(param.value, now);
    if (seconds > 0) param.linearRampToValueAtTime(target, now + seconds);
    else param.setValueAtTime(target, now);
  }

  #attenuate(key: string, db: number) {
    const voice = this.#voices.get(key);
    const ctx = this.#ctx;
    if (!voice || !ctx) return;
    voice.attenuation.gain.setTargetAtTime(dbToGain(db), ctx.currentTime, SMOOTH_S);
  }

  #buffer(ctx: AudioContext, uid: string): Promise<AudioBuffer> {
    let pending = this.#buffers.get(uid);
    if (!pending) {
      pending = this.#opts.loadBytes(uid).then((bytes) => ctx.decodeAudioData(bytes));
      this.#buffers.set(uid, pending);
      // Falha não fica no cache: a próxima tentativa lê o arquivo de novo.
      pending.catch(() => {
        if (this.#buffers.get(uid) === pending) this.#buffers.delete(uid);
      });
    }
    return pending;
  }

  // ---------- Barramentos ----------

  #override(bus: string, change: BusOverride) {
    this.#overrides.set(bus, { ...this.#overrides.get(bus), ...change });
    this.#syncBuses();
  }

  /** Entrada do barramento com o nome dado, ou do barramento mestre se ele não existe. */
  #busNode(name: string): AudioNode | null {
    return (this.#buses.get(name) ?? this.#buses.get(this.#masterName() ?? ''))?.input ?? null;
  }

  #masterName(): string | undefined {
    return (this.#configs.find((c) => c.name === DEFAULT_BUS) ?? this.#configs[0])?.name;
  }

  /** Alinha o grafo com os barramentos: cria os que faltam, tira os que sumiram e aplica volume e efeitos. */
  #syncBuses() {
    const ctx = this.#ctx;
    if (!ctx) return;
    const names = new Set(this.#configs.map((c) => c.name));
    for (const [name, bus] of this.#buses) {
      if (names.has(name)) continue;
      bus.fader.disconnect();
      this.#buses.delete(name);
    }
    for (const cfg of this.#configs) {
      if (!this.#buses.has(cfg.name)) this.#buses.set(cfg.name, this.#createBus(ctx));
    }
    const master = this.#masterName();
    const masterInput = master !== undefined ? this.#buses.get(master)?.input : undefined;
    const now = ctx.currentTime;
    for (const cfg of this.#configs) {
      const bus = this.#buses.get(cfg.name);
      if (!bus) continue;
      this.#applyBus(ctx, bus, cfg, now);
      const target: AudioNode = cfg.name === master || !masterInput ? ctx.destination : masterInput;
      if (bus.target !== target) {
        bus.fader.disconnect();
        bus.fader.connect(target);
        bus.target = target;
      }
    }
  }

  #createBus(ctx: AudioContext): Bus {
    const input = ctx.createGain();
    const lowPass = ctx.createBiquadFilter();
    lowPass.type = 'lowpass';
    const compressor = ctx.createDynamicsCompressor();
    const mix = ctx.createGain();
    const delay = ctx.createDelay(2);
    const delayFeedback = ctx.createGain();
    const delayWet = ctx.createGain();
    const convolver = ctx.createConvolver();
    const reverbWet = ctx.createGain();
    const fader = ctx.createGain();

    input.connect(lowPass);
    lowPass.connect(compressor);
    // Parte seca.
    compressor.connect(mix);
    // Eco: a linha de atraso realimenta a si mesma pelo ganho de realimentação.
    compressor.connect(delay);
    delay.connect(delayFeedback);
    delayFeedback.connect(delay);
    delay.connect(delayWet);
    delayWet.connect(mix);
    // Reverberação por convolução com o impulso gerado.
    compressor.connect(convolver);
    convolver.connect(reverbWet);
    reverbWet.connect(mix);
    mix.connect(fader);
    fader.connect(ctx.destination);

    return {
      input,
      lowPass,
      compressor,
      delay,
      delayFeedback,
      delayWet,
      convolver,
      reverbWet,
      mix,
      fader,
      target: ctx.destination,
      decay: -1,
    };
  }

  #applyBus(ctx: AudioContext, bus: Bus, cfg: BusConfig, now: number) {
    const fx = cfg.effects;
    const override = this.#overrides.get(cfg.name);
    const volumeDb = override?.volumeDb ?? cfg.volumeDb;
    // O solo dos outros canais silencia este mesmo que o mudo do comando seja falso.
    const mute = (override?.mute ?? cfg.mute) || cfg.silencedBySolo;
    bus.fader.gain.setTargetAtTime(mute ? 0 : dbToGain(volumeDb), now, SMOOTH_S);

    bus.lowPass.frequency.setTargetAtTime(fx.low_pass.enabled ? fx.low_pass.cutoff_hz : OPEN_CUTOFF_HZ, now, SMOOTH_S);
    // Sem compressão: razão 1 não altera o sinal.
    bus.compressor.threshold.value = fx.compressor.enabled ? fx.compressor.threshold_db : 0;
    bus.compressor.ratio.value = fx.compressor.enabled ? fx.compressor.ratio : 1;

    bus.delay.delayTime.setTargetAtTime(fx.delay.time, now, SMOOTH_S);
    bus.delayFeedback.gain.setTargetAtTime(fx.delay.enabled ? fx.delay.feedback : 0, now, SMOOTH_S);
    bus.delayWet.gain.setTargetAtTime(fx.delay.enabled ? fx.delay.mix : 0, now, SMOOTH_S);

    // O impulso só é gerado quando a reverberação está ligada e o tempo mudou.
    if (fx.reverb.enabled && bus.decay !== fx.reverb.decay) {
      bus.convolver.buffer = makeImpulse(ctx, fx.reverb.decay);
      bus.decay = fx.reverb.decay;
    }
    bus.reverbWet.gain.setTargetAtTime(fx.reverb.enabled ? fx.reverb.mix : 0, now, SMOOTH_S);
  }
}

function messageOf(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
