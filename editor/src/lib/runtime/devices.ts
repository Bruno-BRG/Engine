// Dispositivos de entrada no navegador (INP-001): botões do mouse e do toque, botões e eixos do controle.
// Os códigos são os do mapa de ações (crates/nexo-core/src/input.rs); se mudar um lado, mude o outro.
import type { SceneEditor } from '$lib/runtime/engine';

/** Botões do mouse pelo bit de `PointerEvent.buttons`: 1 esquerdo, 2 direito, 4 meio. Toque conta como esquerdo. */
const MOUSE_BITS: [number, string][] = [
  [1, 'mouse_left'],
  [2, 'mouse_right'],
  [4, 'mouse_middle'],
];

/** Botões do controle pelo índice do mapeamento padrão (W3C "standard"). null: botão fora do mapa de ações. */
const PAD_BUTTONS: (string | null)[] = [
  'gamepad_a', 'gamepad_b', 'gamepad_x', 'gamepad_y', 'gamepad_lb', 'gamepad_rb',
  null, null, // gatilhos LT e RT
  'gamepad_back', 'gamepad_start',
  null, null, // botões dos analógicos
  'dpad_up', 'dpad_down', 'dpad_left', 'dpad_right',
];

/** Eixos do controle, na ordem do mapeamento padrão: analógico esquerdo X e Y, direito X e Y. */
const PAD_AXES = ['gamepad_left_x', 'gamepad_left_y', 'gamepad_right_x', 'gamepad_right_y'];

/** Códigos que o editor oferece no mapa de ações, por grupo. Analógicos aparecem por sentido (com sinal). */
export const DEVICE_GROUPS: { label: string; codes: string[] }[] = [
  { label: 'Mouse', codes: ['mouse_left', 'mouse_right', 'mouse_middle'] },
  {
    label: 'Controle: botões',
    codes: [
      'gamepad_a', 'gamepad_b', 'gamepad_x', 'gamepad_y', 'gamepad_lb', 'gamepad_rb',
      'gamepad_start', 'gamepad_back', 'dpad_up', 'dpad_down', 'dpad_left', 'dpad_right',
    ],
  },
  {
    label: 'Controle: analógicos',
    codes: [
      'gamepad_left_x-', 'gamepad_left_x+', 'gamepad_left_y-', 'gamepad_left_y+',
      'gamepad_right_x-', 'gamepad_right_x+', 'gamepad_right_y-', 'gamepad_right_y+',
    ],
  },
];

/** Rótulos em português para os códigos de dispositivo. Teclas não entram: aparecem pelo próprio código. */
const ROTULOS: Record<string, string> = {
  mouse_left: 'Mouse esquerdo',
  mouse_right: 'Mouse direito',
  mouse_middle: 'Mouse meio',
  gamepad_a: 'Controle A',
  gamepad_b: 'Controle B',
  gamepad_x: 'Controle X',
  gamepad_y: 'Controle Y',
  gamepad_lb: 'Controle LB',
  gamepad_rb: 'Controle RB',
  gamepad_start: 'Controle Start',
  gamepad_back: 'Controle Back',
  dpad_up: 'Cruz para cima',
  dpad_down: 'Cruz para baixo',
  dpad_left: 'Cruz para a esquerda',
  dpad_right: 'Cruz para a direita',
  'gamepad_left_x-': 'Analógico esquerdo para a esquerda',
  'gamepad_left_x+': 'Analógico esquerdo para a direita',
  'gamepad_left_y-': 'Analógico esquerdo para cima',
  'gamepad_left_y+': 'Analógico esquerdo para baixo',
  'gamepad_right_x-': 'Analógico direito para a esquerda',
  'gamepad_right_x+': 'Analógico direito para a direita',
  'gamepad_right_y-': 'Analógico direito para cima',
  'gamepad_right_y+': 'Analógico direito para baixo',
};

/** Nome amigável de um código do mapa de ações. Código sem rótulo (tecla, por exemplo) volta como está. */
export function labelOf(code: string): string {
  return ROTULOS[code] ?? code;
}

/** Botões do controle pressionados no quadro anterior: só as mudanças vão ao WASM. */
let held = new Set<string>();

/**
 * Lê o primeiro controle conectado e manda ao WASM os eixos e os botões que mudaram. Chamado a cada quadro
 * da reprodução, antes do quadro da lógica. Sem controle, os eixos voltam a zero.
 */
export function pollGamepad(ed: SceneEditor): void {
  const pad = connectedPad();
  const now = new Set<string>();
  PAD_AXES.forEach((code, i) => ed.set_axis(code, pad?.axes[i] ?? 0));
  pad?.buttons.forEach((button, i) => {
    const code = PAD_BUTTONS[i];
    if (code && button.pressed) now.add(code);
  });
  for (const code of held) if (!now.has(code)) ed.key_event(code, false);
  for (const code of now) if (!held.has(code)) ed.key_event(code, true);
  held = now;
}

/** Esquece os botões do controle lidos. Ao parar ou perder o foco o WASM solta tudo, e a próxima leitura refaz. */
export function resetGamepad(): void {
  held = new Set();
}

/** Botões do mouse (e do toque) segurados agora, a partir de `PointerEvent.buttons`. */
export function syncMouse(ed: SceneEditor, buttons: number): void {
  for (const [bit, code] of MOUSE_BITS) ed.key_event(code, (buttons & bit) !== 0);
}

function connectedPad(): Gamepad | null {
  if (typeof navigator === 'undefined' || typeof navigator.getGamepads !== 'function') return null;
  for (const pad of navigator.getGamepads()) if (pad?.connected) return pad;
  return null;
}
