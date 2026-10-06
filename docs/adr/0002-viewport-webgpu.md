# ADR 0002: Viewport runs the runtime in WASM + WebGPU

**Status:** accepted. The Phase 0 proof ran in WebView2 (Windows) with WebGPU at 144 fps (the monitor refresh rate). WKWebView (macOS) and WebKitGTK (Linux) still need validation.

## Context

Embedding a native render surface inside a Tauri window is possible but fragile (window overlap, focus, resize, DPI).

## Decision

Run `nexo-runtime` compiled to WASM inside the WebView, rendering to a `<canvas>` with WebGPU (WebGL2 fallback).

## Fallback

A native wgpu child window positioned over the viewport area, synchronized by the Tauri backend, if WebGPU support or performance is not acceptable on a platform.

## Consequences

- Editor and web export share code: what you see is what you ship.
- WASM has thread and memory limits; very heavy scenes may require the fallback.
