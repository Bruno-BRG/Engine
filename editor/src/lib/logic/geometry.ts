// Geometria do canvas de lógica, em coordenadas do mundo (antes do zoom e do deslocamento). O tamanho dos nós é
// calculado a partir dos pinos, sem medir o DOM: assim o fio chega ao pino certo mesmo antes do desenho.

import type { NodeDef } from './model';

/** Largura de um nó comum. */
export const NODE_W = 200;
/** Altura do cabeçalho do nó. */
export const HEAD_H = 26;
/** Altura de cada linha de pino (também a altura dos campos de valor, que cabem nela). */
export const ROW_H = 22;
/** Tamanho de um reroute (VIS-014): um ponto, com entrada à esquerda e saída à direita. */
export const REROUTE_D = 16;
/** Tamanho inicial de uma caixa de comentário. */
export const COMMENT_SIZE: [number, number] = [240, 120];
/** Tamanho mínimo de uma caixa de comentário ao redimensionar. */
export const COMMENT_MIN: [number, number] = [120, 60];

export type Vec = [number, number];

/** Retângulo [x, y, largura, altura]. */
export type Rect = [number, number, number, number];

export function isReroute(kind: string): boolean {
  return kind === 'Reroute';
}

/** Tamanho de um nó: [largura, altura]. Um tipo desconhecido vira um cabeçalho vazio. */
export function nodeSize(kind: string, def: NodeDef | undefined): Vec {
  if (isReroute(kind)) return [REROUTE_D, REROUTE_D];
  const linhas = Math.max(def?.inputs.length ?? 0, def?.outputs.length ?? 0);
  return [NODE_W, HEAD_H + linhas * ROW_H + 6];
}

/** Ponto de um pino no mundo. Entradas ficam na borda esquerda, saídas na direita, cada uma na sua linha. */
export function pinPoint(
  position: Vec,
  kind: string,
  def: NodeDef | undefined,
  pin: string,
  output: boolean,
): Vec {
  const [x, y] = position;
  if (isReroute(kind)) {
    return output ? [x + REROUTE_D, y + REROUTE_D / 2] : [x, y + REROUTE_D / 2];
  }
  const lista = output ? (def?.outputs ?? []) : (def?.inputs ?? []);
  const i = Math.max(0, lista.findIndex((p) => p.name === pin));
  const linha = y + HEAD_H + i * ROW_H + ROW_H / 2;
  return [output ? x + NODE_W : x, linha];
}

/** Caminho SVG de um fio, de uma saída (a) até uma entrada (b). A curva sai para a direita e chega pela esquerda. */
export function wirePath(a: Vec, b: Vec): string {
  const d = Math.max(40, Math.abs(b[0] - a[0]) * 0.5);
  return `M ${a[0]} ${a[1]} C ${a[0] + d} ${a[1]}, ${b[0] - d} ${b[1]}, ${b[0]} ${b[1]}`;
}

/** Retângulo normalizado a partir de dois cantos (o arrasto pode ir em qualquer direção). */
export function normRect(x0: number, y0: number, x1: number, y1: number): Rect {
  return [Math.min(x0, x1), Math.min(y0, y1), Math.abs(x1 - x0), Math.abs(y1 - y0)];
}

/** True se os dois retângulos se tocam. */
export function rectsTouch(a: Rect, b: Rect): boolean {
  return a[0] <= b[0] + b[2] && b[0] <= a[0] + a[2] && a[1] <= b[1] + b[3] && b[1] <= a[1] + a[3];
}

/** Caixa envolvente de um conjunto de retângulos, ou null se o conjunto é vazio. */
export function boundsOf(rects: Rect[]): Rect | null {
  if (!rects.length) return null;
  const x0 = Math.min(...rects.map((r) => r[0]));
  const y0 = Math.min(...rects.map((r) => r[1]));
  const x1 = Math.max(...rects.map((r) => r[0] + r[2]));
  const y1 = Math.max(...rects.map((r) => r[1] + r[3]));
  return [x0, y0, x1 - x0, y1 - y0];
}

/** Câmera do canvas: o mundo aparece em `screen = mundo * zoom + (x, y)`, em pixels do canvas. */
export interface Camera {
  x: number;
  y: number;
  zoom: number;
}

export const ZOOM_MIN = 0.25;
export const ZOOM_MAX = 2.5;

/** Câmera que enquadra o retângulo dado no canvas de `w` por `h` pixels, com uma margem. */
export function frameCamera(r: Rect, w: number, h: number): Camera {
  const margem = 60;
  const zoom = Math.min(
    ZOOM_MAX,
    Math.max(ZOOM_MIN, Math.min(w / (r[2] + margem * 2), h / (r[3] + margem * 2))),
  );
  return {
    x: w / 2 - (r[0] + r[2] / 2) * zoom,
    y: h / 2 - (r[1] + r[3] / 2) * zoom,
    zoom,
  };
}

/** Zoom em torno de um ponto da tela: o mundo sob o cursor fica parado. */
export function zoomAt(cam: Camera, factor: number, sx: number, sy: number): Camera {
  const zoom = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, cam.zoom * factor));
  const k = zoom / cam.zoom;
  return {
    x: sx - (sx - cam.x) * k,
    y: sy - (sy - cam.y) * k,
    zoom,
  };
}

/** Ponto da tela (relativo ao canvas) para o mundo. */
export function toWorld(cam: Camera, sx: number, sy: number): Vec {
  return [(sx - cam.x) / cam.zoom, (sy - cam.y) / cam.zoom];
}
