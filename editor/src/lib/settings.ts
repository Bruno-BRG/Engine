import type { BusEffects } from './audio/types';
import type { NameList } from './state/scene.svelte';
// Configurações do projeto (settings.ron) no lado do TypeScript. Espelha `ProjectSettings` de
// crates/nexo-core/src/settings.rs. Os nomes de campo são os do serde (snake_case): o JSON vai e
// volta sem conversão. Se mudar um lado, mude o outro.

/** Número de camadas de colisão. Cada camada é um bit de um inteiro de 32 bits (PHY-020). */
export const LAYER_COUNT = 32;

/** Máscara com todas as camadas, como `u32::MAX` do Rust. */
export const ALL_BITS = 0xffffffff;

export interface Bus {
  name: string;
  /** Decibéis: 0 é o volume original, -80 é silêncio. */
  volume_db: number;
  mute: boolean;
  /** Efeitos (AUD-003). Preenchidos com `defaultBusEffects()` quando o arquivo não os traz. */
  effects: BusEffects;
}

/** Material de física (PHY-023). */
export interface PhysicsMaterial {
  name: string;
  /** Atrito entre 0 (gelo) e 1 ou mais. */
  friction: number;
  /** Quique entre 0 (sem quique) e 1 (quique total). */
  bounce: number;
  /** Superfície áspera: usa o atrito máximo entre os dois corpos. */
  rough: boolean;
  /** Superfície absorvente: usa o quique mínimo entre os dois corpos. */
  absorbent: boolean;
}

/** Mapa de entrada (INP-001): ação para as teclas (KeyboardEvent.code) que a acionam. */
export interface InputMap {
  actions: Record<string, string[]>;
}

export interface ProjectSettings {
  /** Gravidade em pixels por segundo², com Y para baixo. */
  gravity: [number, number];
  /** Nomes das 32 camadas. Índice = bit. Texto vazio = camada livre. */
  layers: string[];
  /** Para cada camada, os bits das camadas com que ela colide. Simétrica (A colide com B se B com A). */
  layer_matrix: number[];
  input: InputMap;
  buses: Bus[];
  /** Tags disponíveis para os nós (SCN-033). */
  tags: string[];
  physics_materials: PhysicsMaterial[];
}

/** Configurações vazias, usadas antes de abrir um projeto. Ao abrir, valem as do arquivo. */
export function emptySettings(): ProjectSettings {
  return {
    gravity: [0, 980],
    layers: Array<string>(LAYER_COUNT).fill(''),
    layer_matrix: Array<number>(LAYER_COUNT).fill(ALL_BITS),
    input: { actions: {} },
    buses: [],
    tags: [],
    physics_materials: [],
  };
}

/** True se o bit `i` está ligado na máscara. Aceita máscara com sinal (-1 = todos os bits). */
export function hasBit(mask: number, i: number): boolean {
  return ((mask >>> i) & 1) === 1;
}

/** Máscara com o bit `i` ligado ou desligado, sem sinal (como `u32`). */
export function withBit(mask: number, i: number, on: boolean): number {
  const bit = 1 << i;
  return (on ? mask | bit : mask & ~bit) >>> 0;
}

/**
 * Liga ou desliga a colisão entre duas camadas nos dois sentidos, para a matriz continuar simétrica.
 * Muda `s` no lugar: quem chama deve passar o objeto de `projectSettings`.
 */
export function setCollision(s: ProjectSettings, a: number, b: number, on: boolean): void {
  s.layer_matrix[a] = withBit(s.layer_matrix[a] ?? 0, b, on);
  s.layer_matrix[b] = withBit(s.layer_matrix[b] ?? 0, a, on);
}

/** Lista com `value` no fim quando ele não está na lista, para não sumir um valor de cena antigo. */
export function withValue(list: string[], value: string): string[] {
  return list.includes(value) ? list : [...list, value];
}

/** Lista só com as camadas nomeadas (índice e nome). */
export function namedLayers(s: ProjectSettings): { index: number; name: string }[] {
  return s.layers.flatMap((name, index) => (name ? [{ index, name }] : []));
}

/** Nomes da lista pedida pelo registro (`names` de um campo de texto), na ordem das configurações. */
export function namesOf(s: ProjectSettings, list: NameList): string[] {
  switch (list) {
    case 'tags':
      return s.tags;
    case 'layers':
      return namedLayers(s).map((l) => l.name);
    case 'buses':
      return s.buses.map((b) => b.name);
    case 'physics_materials':
      return s.physics_materials.map((m) => m.name);
  }
}
