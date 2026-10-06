# ADR 0001: Tauri 2 no editor e Rust no núcleo

**Status:** aceito

## Contexto

A interface precisa ser excepcionalmente bonita e o núcleo precisa ser rápido e portátil, incluindo web.

## Decisão

Editor em Tauri 2 com frontend SvelteKit + TypeScript. Núcleo, runtime e ferramentas em Rust. C/C++ só via bindings quando uma biblioteca indispensável não tiver equivalente em Rust.

## Consequências

- UI com toda a expressividade da web e instalador pequeno.
- Rust compila para nativo e WASM, permitindo um único runtime.
- Dependência do WebView do sistema: WebView2 (Windows), WKWebView (macOS), WebKitGTK (Linux). O Linux exige atenção extra com WebGPU.
