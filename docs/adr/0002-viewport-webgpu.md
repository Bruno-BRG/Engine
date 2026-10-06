# ADR 0002: Viewport com o runtime em WASM + WebGPU

**Status:** proposto, depende da prova técnica da Fase 0

## Contexto

Embutir uma superfície nativa de renderização dentro de uma janela Tauri é possível, mas frágil (sobreposição de janelas, foco, redimensionamento, DPI).

## Decisão

Rodar o `nexo-runtime` compilado para WASM dentro do WebView, renderizando num `<canvas>` com WebGPU (fallback WebGL2).

## Alternativa (plano B)

Janela filha nativa com wgpu posicionada sobre a área do viewport, sincronizada pelo backend Tauri. Usada se o desempenho ou o suporte a WebGPU no WebKitGTK/WKWebView não for aceitável.

## Consequências

- Editor e exportação web usam o mesmo código: o que se vê é o que se exporta.
- WASM tem limite de threads e memória; cenas muito pesadas no editor podem exigir o plano B.
