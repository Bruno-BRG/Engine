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
  null, null, // LT and RT are read as axes, see PAD_TRIGGERS
  'gamepad_back', 'gamepad_start',
  null, null, // botões dos analógicos
  'dpad_up', 'dpad_down', 'dpad_left', 'dpad_right',
];

/** Eixos do controle, na ordem do mapeamento padrão: analógico esquerdo X e Y, direito X e Y. */
const PAD_AXES = ['gamepad_left_x', 'gamepad_left_y', 'gamepad_right_x', 'gamepad_right_y'];

/** Triggers LT and RT: buttons 6 and 7 of the standard mapping, read as unsigned axes from 0 to 1. */
const PAD_TRIGGERS: [string, number][] = [
  ['gamepad_lt', 6],
  ['gamepad_rt', 7],
];

/** Every axis the WASM receives: the sticks and the triggers. */
const ALL_AXES = [...PAD_AXES, ...PAD_TRIGGERS.map(([code]) => code)];

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
  { label: 'Controle: gatilhos', codes: ['gamepad_lt', 'gamepad_rt'] },
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
  gamepad_lt: 'Gatilho LT',
  gamepad_rt: 'Gatilho RT',
  'gamepad_left_x-':'Analógico esquerdo para a esquerda',
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
 * Lê todos os controles conectados e manda ao WASM os eixos e os botões que mudaram. Chamado a cada quadro
 * da reprodução, antes do quadro da lógica. Os botões valem em união: o de qualquer controle aciona. Cada
 * eixo vale o do controle com maior módulo. Sem controle, os eixos voltam a zero.
 */
export function pollGamepad(ed: SceneEditor): void {
  const pads = connectedPads();
  const now = new Set<string>();
  const axes = new Map<string, number>();
  for (const code of ALL_AXES) axes.set(code, 0);
  for (const pad of pads) {
    pad.buttons.forEach((button, i) => {
      const code = PAD_BUTTONS[i];
      if (code && button.pressed) now.add(code);
    });
    PAD_AXES.forEach((code, i) => keepStrongest(axes, code, pad.axes[i]));
    PAD_TRIGGERS.forEach(([code, index]) => keepStrongest(axes, code, pad.buttons[index]?.value));
  }
  for (const [code, value] of axes) ed.set_axis(code, value);
  for (const code of held) if (!now.has(code)) ed.key_event(code, false);
  for (const code of now) if (!held.has(code)) ed.key_event(code, true);
  held = now;
}

/** Guarda em `axes` o valor de maior módulo entre os controles. Valor ausente ou não numérico conta como zero. */
function keepStrongest(axes: Map<string, number>, code: string, raw: number | undefined): void {
  const value = typeof raw === 'number' && Number.isFinite(raw) ? raw : 0;
  if (Math.abs(value) > Math.abs(axes.get(code) ?? 0)) axes.set(code, value);
}

/** Esquece os botões do controle lidos. Ao parar ou perder o foco o WASM solta tudo, e a próxima leitura refaz. */
export function resetGamepad(): void {
  held = new Set();
}

/** Botão apertado para a captura: a partir de 0,5 do curso. */
const CAPTURE_BUTTON = 0.5;
/** Analógico ou gatilho empurrado para a captura: a partir de 0,7, acima da zona morta e do limiar de ação. */
const CAPTURE_AXIS = 0.7;

/**
 * Espera o primeiro botão, gatilho ou analógico do controle empurrado além do limiar e chama `onCode` com o
 * código dele ("gamepad_a", "gamepad_lt", "gamepad_left_x-"). Só lê o controle: não manda nada ao WASM. Um
 * botão já apertado quando a espera começa não conta até ser solto. Devolve a função que cancela a espera.
 */
export function watchGamepad(onCode: (code: string) => void): () => void {
  let frame = 0;
  let active = true;
  let before: Set<string> | null = null;
  const stop = () => {
    active = false;
    cancelAnimationFrame(frame);
  };
  const tick = () => {
    if (!active) return;
    const now = pressedForCapture();
    if (before) {
      for (const code of now) {
        if (!before.has(code)) {
          stop();
          onCode(code);
          return;
        }
      }
    }
    before = now;
    frame = requestAnimationFrame(tick);
  };
  frame = requestAnimationFrame(tick);
  return stop;
}

/** Códigos apertados agora, em qualquer controle conectado, pelos limiares da captura. Só lê. */
function pressedForCapture(): Set<string> {
  const codes = new Set<string>();
  for (const pad of connectedPads()) {
    pad.buttons.forEach((button, i) => {
      const code = PAD_BUTTONS[i];
      if (code && (button.pressed || button.value >= CAPTURE_BUTTON)) codes.add(code);
    });
    PAD_TRIGGERS.forEach(([code, index]) => {
      if ((pad.buttons[index]?.value ?? 0) >= CAPTURE_AXIS) codes.add(code);
    });
    PAD_AXES.forEach((code, i) => {
      const raw = pad.axes[i] ?? 0;
      if (raw >= CAPTURE_AXIS) codes.add(`${code}+`);
      else if (raw <= -CAPTURE_AXIS) codes.add(`${code}-`);
    });
  }
  return codes;
}

/** Botões do mouse (e do toque) segurados agora, a partir de `PointerEvent.buttons`. */
export function syncMouse(ed: SceneEditor, buttons: number): void {
  for (const [bit, code] of MOUSE_BITS) ed.key_event(code, (buttons & bit) !== 0);
}

/** Controles conectados agora. Vazio sem a API (fora do navegador) ou sem controle. */
function connectedPads(): Gamepad[] {
  if (typeof navigator === 'undefined' || typeof navigator.getGamepads !== 'function') return [];
  return Array.from(navigator.getGamepads()).filter((pad): pad is Gamepad => pad?.connected === true);
}
