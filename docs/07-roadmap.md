# 07. Roadmap

Each phase ends with something usable. Task-level detail lives in the [backlog](backlog/README.md); the IDs below point there.

## Phase 0: Foundation ✅ (Windows)

Cargo workspace, Tauri 2 + Svelte 5 app, design tokens, editor shell, WASM + WebGPU viewport proof (144 fps in WebView2), templates, scene save/load in RON, live C# panel.

## Phase 1: Unity-style editor + node scenes

- Unity-style theme and layout: menu bar, toolbar with Play/Pause/Step, Scene/Game tabs, dockable panels (EDT-001…EDT-030).
- Node tree model in Rust with types, UIDs, parenting, scene instancing (SCN-001…SCN-020).
- Inspector driven by node type metadata, Add Node/Component menu (INS-001…INS-020).
- Project browser and asset import with thumbnails and hot reload (AST-001…AST-015).
- Gizmos: move, rotate, scale, rect tool, snapping (VPT-001…VPT-015).
- Unity-style Console (CON-001…CON-010).

**Exit:** build a level with real sprites as a node tree, save, reopen, nothing lost.

## Phase 2: Physics, input, audio, visual logic

- Rapier 2D: bodies, shapes, layers/masks, materials, joints, raycasts, debug draw (PHY).
- Input map and actions (INP), audio players and buses (AUD).
- GameMaker-style Logic workspace: script list, events, infinite canvas, node search, IR + VM, live debugging (VIS).

**Exit:** someone who has never coded makes a playable platformer with coins in 30 minutes.

## Phase 3: Public alpha

Desktop + web export (EXP), tilemaps (R2D), sprite animation and AnimationPlayer (ANI), game UI nodes (GUI), particles, profiler basics (DBG), three example projects, node reference docs.

**Exit:** a small complete game published on itch.io made only with Nexo.

## Phase 4: Depth (1.0)

User behaviors, eject to C#, graph diff, 2D lights and shadows, post-processing, shader graph, localization, full profiler.

## Phase 5: Expansion

Hand-written C#, mobile, initial 3D, asset/extension store, multiplayer.
