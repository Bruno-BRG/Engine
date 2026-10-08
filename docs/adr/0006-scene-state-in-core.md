# ADR 0006: The scene state lives in nexo-core, behind WebAssembly

**Status:** accepted

## Context

Phase 0 kept the scene as a flat list of objects in the TypeScript store. Rust only drew it. ADR 0004 asks for a node tree with stable UIDs, a type registry and migration, and backlog item SCN-005 asks for the state of truth to move from the UI to Rust. The Phase 2 visual-logic VM (ADR 0003) needs the same state, so keeping a second copy in TypeScript would split the model again.

Two ways to move the state to Rust were on the table:

1. Editor commands through Tauri (`invoke`). Every edit becomes an async round trip. A drag at 60 Hz would send and parse a full tree dozens of times per second, and the Inspector would have to wait for the reply before updating.
2. The same Rust model compiled to WebAssembly, which the viewport already loads. Calls are synchronous and cost no IPC.

## Decision

- `crates/nexo-core` owns the scene: the node tree (`scene.rs`), the registry of node types with inheritance and property metadata (`registry.rs`), the RON file format and the migration of the Phase 0 format (`io.rs`), and the queries used by the viewport: world transforms, the draw list, picking, bounds and the outline (`query.rs`).
- `crates/nexo-runtime-web` exposes `SceneEditor` to JavaScript. The editor sends operations (`add_node`, `set_prop`, `move_node`, `undo`…) and reads a JSON outline after each one. The TypeScript store (`state/scene.svelte.ts`) is a reactive mirror, not a second source of truth.
- Undo and redo are snapshots taken inside the core. Operations that share a coalesce key (a gizmo drag, a scrub) form one step until `end_group`.
- The play-in-editor copy is a second `Scene` created by `play_copy`. Behaviors (spin, float, pulse) are applied as a pose computed from the base values at each time, so the edit scene is never changed while playing.
- Files: format 2 is RON, written in pre-order, with only the non-default properties and UIDs as 16 hex digits. Format 1 (Phase 0) is migrated when a project is opened, and the original file is kept as `.v1.bak`.

## Consequences

- Scene rules (validation, cycle checks, name uniqueness, default values) are tested with `cargo test` and do not depend on the UI.
- The same model can run in the Phase 2 VM and the exporter without a second implementation.
- Editing needs a WebAssembly rebuild when the core changes (`scripts/build-wasm.mjs`); the dev app does not pick it up by hot reload.
- Multi-select editing, scene instancing and the other ADR 0004 items build on this model; their UI is still to come.
