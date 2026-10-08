// Funções puras e construções do WebAudio usadas pelo motor. Sem estado, para ficarem fáceis de conferir.

/** Abaixo deste volume o som é silêncio absoluto (igual ao núcleo: SILENCE_DB). */
export const SILENCE_DB = -80;

/** Ganho linear de um volume em dB: 0 dB é 1, -6 dB é cerca de 0,5, -80 dB ou menos é 0. */
export function dbToGain(db: number): number {
  if (!Number.isFinite(db) || db <= SILENCE_DB) return 0;
  return 10 ** (db / 20);
}

/**
 * Impulso de reverberação: ruído branco com queda exponencial, de 0 dB no início a -60 dB no fim de
 * `seconds`. Dois canais, para uma cauda estéreo.
 */
export function makeImpulse(ctx: BaseAudioContext, seconds: number): AudioBuffer {
  const length = Math.max(1, Math.round(ctx.sampleRate * seconds));
  const buffer = ctx.createBuffer(2, length, ctx.sampleRate);
  for (let ch = 0; ch < 2; ch++) {
    const data = buffer.getChannelData(ch);
    for (let i = 0; i < length; i++) {
      const t = i / ctx.sampleRate;
      data[i] = (Math.random() * 2 - 1) * Math.exp((-6.9 * t) / seconds);
    }
  }
  return buffer;
}
