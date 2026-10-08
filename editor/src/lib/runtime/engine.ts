// Carregamento único do runtime WASM: o viewport (desenho) e o editor de cena (árvore de nós)
// compartilham o mesmo módulo.
import init, { GraphEditor, SceneEditor, Viewport } from './pkg/nexo_runtime_web.js';

export type { Viewport, SceneEditor, GraphEditor };

let ready: Promise<unknown> | null = null;

/** Carrega o módulo WASM uma vez. Chamadas seguintes reaproveitam a mesma promessa. */
export function loadRuntime(): Promise<unknown> {
  ready ??= init();
  return ready;
}

/** Cria o viewport sobre um canvas. Usa WebGPU quando disponível e WebGL2 como reserva. */
export async function createViewport(canvas: HTMLCanvasElement): Promise<Viewport> {
  await loadRuntime();
  return Viewport.create(canvas);
}

/** Editor de cena vazio. `seed` alimenta a geração de UIDs. */
export async function createSceneEditor(): Promise<SceneEditor> {
  await loadRuntime();
  return new SceneEditor(randomSeed());
}

/** Registro de tipos de nó em JSON (método estático do WASM, sem instância). */
export function readNodeRegistry(): string {
  return SceneEditor.registry();
}

/** Grafo de lógica vazio (VIS-020). Um por script aberto. */
export async function createGraphEditor(): Promise<GraphEditor> {
  await loadRuntime();
  return new GraphEditor();
}

/** Biblioteca de nós da lógica em JSON (estática). Depende só de o módulo WASM estar carregado. */
export async function readNodeLibrary(): Promise<string> {
  await loadRuntime();
  return GraphEditor.node_library();
}

/** Nome da classe C# de um script, como o gerador a escreve (VIS-024). Só vale depois de carregar o WASM. */
export function csharpClassName(script: string): string {
  return GraphEditor.csharp_class_name(script);
}

/** Semente de 52 bits para os UIDs (cabe em um número de JavaScript sem perda). */
export function randomSeed(): number {
  const b = new Uint32Array(2);
  crypto.getRandomValues(b);
  return (b[0] * 0x100000) + (b[1] >>> 12);
}
