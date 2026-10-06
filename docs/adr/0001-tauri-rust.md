# ADR 0001: Tauri 2 for the editor, Rust for the core

**Status:** accepted

## Context

The UI must be beautiful and productive; the core must be fast and portable, including the web.

## Decision

Editor in Tauri 2 with a Svelte 5 + Vite + TypeScript frontend. Core, runtime and tools in Rust. C/C++ only through bindings when an essential library has no Rust equivalent.

## Consequences

- Rich UI and a small installer.
- Rust compiles to native and WASM, enabling one runtime.
- Depends on the system WebView: WebView2 (Windows), WKWebView (macOS), WebKitGTK (Linux). Linux needs extra care with WebGPU.
