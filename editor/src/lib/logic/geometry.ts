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

/**
 * Below this zoom, nodes drop their labels and value fields, and comment boxes show a preview (semantic zoom, VIS-014).
 */
export const DETAIL_MIN_ZOOM = 0.6;

/** True when the canvas draws labels, value fields and the editable comment text at this zoom (VIS-014). */
export function showsDetail(zoom: number): boolean {
  return zoom >= DETAIL_MIN_ZOOM;
}

/** Gap between two columns, and between two nodes stacked in the same column, in the flow layout (VIS-014). */
export const LAYOUT_GAP_X = 80;
export const LAYOUT_GAP_Y = 40;

/** A node for the flow layout: its id, its size and its current position. */
export interface LayoutNode {
  id: number;
  size: Vec;
  position: Vec;
}

/** A wire for the flow layout: the node that outputs (`from`) and the node that takes the input (`to`). */
export interface LayoutEdge {
  from: number;
  to: number;
}

/**
 * Flow layout (VIS-014): puts each node in the column of the longest chain of wires that leads to it, so the flow
 * reads from left to right and a data node sits left of the node that consumes it. Inside a column the nodes keep
 * their current top-to-bottom order, stacked without overlap. A wire that closes a loop (a While whose body goes
 * back to it) does not count, so the layout always ends. Wires to nodes outside `nodes` are ignored. Returns the new
 * position of each node; the nodes themselves are not moved here.
 */
export function flowLayout(nodes: LayoutNode[], edges: LayoutEdge[]): Map<number, Vec> {
  const out = new Map<number, Vec>();
  if (!nodes.length) return out;
  const byId = new Map(nodes.map((n) => [n.id, n] as const));
  // Top to bottom, then left to right: the order the nodes already have, used for ties and for the depth-first roots.
  const order = [...nodes]
    .sort((a, b) => a.position[1] - b.position[1] || a.position[0] - b.position[0] || a.id - b.id)
    .map((n) => n.id);

  const next = new Map<number, number[]>(nodes.map((n) => [n.id, []] as const));
  for (const e of edges) {
    if (e.from !== e.to && byId.has(e.from) && byId.has(e.to)) next.get(e.from)?.push(e.to);
  }

  // Depth-first search: an edge that reaches a node still on the current path closes a loop, so it is dropped.
  // What is left is acyclic.
  const state = new Map<number, 'onPath' | 'done'>();
  const acyclic = new Map<number, number[]>(nodes.map((n) => [n.id, []] as const));
  for (const root of order) {
    if (state.has(root)) continue;
    state.set(root, 'onPath');
    const path: { id: number; step: number }[] = [{ id: root, step: 0 }];
    while (path.length) {
      const top = path[path.length - 1];
      if (!top) break;
      const targets = next.get(top.id) ?? [];
      if (top.step >= targets.length) {
        state.set(top.id, 'done');
        path.pop();
        continue;
      }
      const target = targets[top.step];
      if (target === undefined) break;
      top.step += 1;
      const seen = state.get(target);
      if (seen === 'onPath') continue;
      acyclic.get(top.id)?.push(target);
      if (seen === undefined) {
        state.set(target, 'onPath');
        path.push({ id: target, step: 0 });
      }
    }
  }

  // Column of each node: one more than its deepest input. Kahn's order visits every node, since the graph is acyclic.
  const inputs = new Map<number, number>(nodes.map((n) => [n.id, 0] as const));
  for (const targets of acyclic.values()) {
    for (const t of targets) inputs.set(t, (inputs.get(t) ?? 0) + 1);
  }
  const column = new Map<number, number>(nodes.map((n) => [n.id, 0] as const));
  const queue = order.filter((id) => inputs.get(id) === 0);
  for (let i = 0; i < queue.length; i++) {
    const id = queue[i];
    if (id === undefined) break;
    for (const t of acyclic.get(id) ?? []) {
      column.set(t, Math.max(column.get(t) ?? 0, (column.get(id) ?? 0) + 1));
      const left = (inputs.get(t) ?? 0) - 1;
      inputs.set(t, left);
      if (left === 0) queue.push(t);
    }
  }

  // Columns in order, each one stacked from the top-left corner of the whole set. A column is as wide as its
  // widest node.
  const columns: number[][] = [];
  for (const id of order) {
    const c = column.get(id) ?? 0;
    (columns[c] ??= []).push(id);
  }
  const originX = Math.min(...nodes.map((n) => n.position[0]));
  const originY = Math.min(...nodes.map((n) => n.position[1]));
  let x = originX;
  for (const members of columns) {
    if (!members) continue;
    let y = originY;
    let width = 0;
    for (const id of members) {
      const n = byId.get(id);
      if (!n) continue;
      out.set(id, [Math.round(x), Math.round(y)]);
      y += n.size[1] + LAYOUT_GAP_Y;
      width = Math.max(width, n.size[0]);
    }
    x += width + LAYOUT_GAP_X;
  }
  return out;
}

/** Where a point of the world lands on a map: `map = world * scale + (x, y)`, in pixels of the map (VIS-014). */
export interface MapTransform {
  scale: number;
  x: number;
  y: number;
}

/** Transform that fits the world rectangle `r` in a map of `w` by `h` pixels, centred, with `margin` pixels around. */
export function fitMap(r: Rect, w: number, h: number, margin = 0): MapTransform {
  const scale = Math.min((w - margin * 2) / Math.max(r[2], 1), (h - margin * 2) / Math.max(r[3], 1));
  return {
    scale,
    x: w / 2 - (r[0] + r[2] / 2) * scale,
    y: h / 2 - (r[1] + r[3] / 2) * scale,
  };
}

/** Camera that puts the world point (wx, wy) at the centre of a `w` by `h` canvas, keeping the zoom (map click). */
export function centerCamera(cam: Camera, wx: number, wy: number, w: number, h: number): Camera {
  return { x: w / 2 - wx * cam.zoom, y: h / 2 - wy * cam.zoom, zoom: cam.zoom };
}

/**
 * Camera for a pointer on the minimap (VIS-014): the world point under the pointer goes to the centre of the canvas,
 * and the zoom is kept. `fit` must be the transform taken when the drag began, and it must stay the same for the whole
 * drag. Then the result depends only on the pointer, so the camera does not drift from one event to the next.
 */
export function minimapToCamera(
  cam: Camera,
  fit: MapTransform,
  mapX: number,
  mapY: number,
  w: number,
  h: number,
): Camera {
  const wx = (mapX - fit.x) / fit.scale;
  const wy = (mapY - fit.y) / fit.scale;
  return centerCamera(cam, wx, wy, w, h);
}
