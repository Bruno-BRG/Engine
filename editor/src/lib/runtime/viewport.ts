import init, { Viewport } from './pkg/nexo_runtime_web.js';

export type { Viewport };

let ready: Promise<unknown> | null = null;

/** Carrega o runtime WASM (uma vez) e cria um viewport sobre o canvas. */
export async function createViewport(canvas: HTMLCanvasElement): Promise<Viewport> {
  ready ??= init();
  await ready;
  return Viewport.create(canvas);
}
