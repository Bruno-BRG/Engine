# Backlog

Every planned task, grouped by topic. Each item has an ID, a priority and a phase, plus how Unity (U), Godot (G) or GameMaker (GM) solve it. Check the box in the same PR that completes the item (see [CONTRIBUTING](../../CONTRIBUTING.md)).

| Priority | Meaning |
|---|---|
| P0 | Needed for the next phase |
| P1 | Needed before 1.0 |
| P2 | After 1.0 |

## Guiding decisions

- **Visual style from Unity:** neutral grays, dense rows, 2–3 px radius, familiar panel layout.
- **Scene model from Godot:** everything is a node with a type, properties, children and signals.
- **Logic workflow from GameMaker:** scripts are assets with events; each event opens a node graph on an infinite canvas.

## Topics

| Topic | File | Prefix | Main phase |
|---|---|---|---|
| Editor UI (Unity style) | [editor-ui.md](editor-ui.md) | EDT | 1 |
| Scenes and nodes | [scenes-nodes.md](scenes-nodes.md) | SCN | 1 |
| Inspector | [inspector.md](inspector.md) | INS | 1 |
| Scene viewport and gizmos | [viewport.md](viewport.md) | VPT | 1 |
| Project browser and assets | [assets.md](assets.md) | AST | 1 |
| Console, debugger, profiler | [console-debug.md](console-debug.md) | CON / DBG | 1–3 |
| Physics 2D | [physics.md](physics.md) | PHY | 2 |
| Visual scripting | [visual-scripting.md](visual-scripting.md) | VIS | 2 |
| Input | [input.md](input.md) | INP | 2 |
| Audio | [audio.md](audio.md) | AUD | 2 |
| 2D rendering | [rendering-2d.md](rendering-2d.md) | R2D | 3 |
| Animation | [animation.md](animation.md) | ANI | 3 |
| Game UI | [game-ui.md](game-ui.md) | GUI | 3 |
| Export and build | [export.md](export.md) | EXP | 3 |
| Documentation and community | [docs-community.md](docs-community.md) | DOC | all |
| 3D and networking (future) | [future.md](future.md) | F3D / NET | 5 |

## Already done (Phase 0)

- [x] Cargo workspace, Tauri 2 + Svelte 5 editor shell
- [x] WASM + WebGPU viewport with grid, game frame, procedural sprites, selection outline
- [x] Templates, recent projects, save/load scenes in RON, autosave
- [x] Hierarchy (flat), Inspector (transform, look, behaviors), scrub fields
- [x] Play/stop in viewport, live debugger table, live C# panel, Ctrl+K palette, undo/redo
