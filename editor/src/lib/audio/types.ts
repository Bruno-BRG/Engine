// Tipos do áudio do editor. O contrato dos comandos vem do Rust (crates/nexo-core/src/audio.rs): cada
// comando tem `type` em snake_case e os demais campos em camelCase. Volumes são em decibéis.

/** Comando que o núcleo envia ao motor de áudio, uma lista por quadro da reprodução. */
export type AudioCommand =
  | {
      type: 'play';
      node: string;
      stream: string;
      volumeDb: number;
      pitch: number;
      bus: string;
      looping: boolean;
      fromStart: boolean;
    }
  | { type: 'stop'; node: string }
  | { type: 'fade'; node: string; toDb: number; seconds: number }
  | { type: 'set_bus_volume'; bus: string; volumeDb: number }
  | { type: 'set_bus_mute'; bus: string; mute: boolean }
  | { type: 'set_attenuation'; node: string; db: number };

/**
 * Efeitos de um barramento (AUD-003). Os nomes são os do arquivo settings.ron (snake_case, como o serde
 * do Rust os grava). Quando estão nos padrões, o Rust não os grava: o carregamento preenche com `defaultBusEffects`.
 */
export interface BusEffects {
  reverb: { enabled: boolean; mix: number; decay: number };
  delay: { enabled: boolean; time: number; feedback: number; mix: number };
  low_pass: { enabled: boolean; cutoff_hz: number };
  compressor: { enabled: boolean; threshold_db: number; ratio: number };
}

/** Efeitos desligados, com os valores padrão do núcleo (crates/nexo-core/src/audio.rs). */
export function defaultBusEffects(): BusEffects {
  return {
    reverb: { enabled: false, mix: 0.3, decay: 2 },
    delay: { enabled: false, time: 0.3, feedback: 0.3, mix: 0.3 },
    low_pass: { enabled: false, cutoff_hz: 20000 },
    compressor: { enabled: false, threshold_db: -20, ratio: 4 },
  };
}

/** O que o motor precisa de um barramento: os valores do projeto e o solo dos outros canais. */
export interface BusConfig {
  name: string;
  volumeDb: number;
  /** Mudo gravado no projeto. */
  mute: boolean;
  /** Outro canal está em solo e este não: o canal fica mudo nesta sessão, sem mexer no projeto. */
  silencedBySolo: boolean;
  effects: BusEffects;
}

/** Nome do barramento padrão (o núcleo usa o mesmo nome ao tocar sem barramento válido). */
export const DEFAULT_BUS = 'Master';

/** Chave da voz de pré-visualização. Não é um UID de nó, então nunca colide com um player. */
export const PREVIEW_KEY = '@preview';
