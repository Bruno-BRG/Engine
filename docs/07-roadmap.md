# 07. Roadmap

Each phase ends with something usable. Task-level detail lives in the [backlog](backlog/README.md); the IDs below point there.

## Phase 0: Foundation ✅ (Windows)

Cargo workspace, Tauri 2 + Svelte 5 app, design tokens, editor shell, WASM + WebGPU viewport proof (144 fps in WebView2), templates, scene save/load in RON, live C# panel.

## Phase 1: Unity-style editor + node scenes ✅ (Windows)

Status: the node tree (with migration from Phase 0), the registry-driven Inspector, the transform tools, the Game tab, the Project panel with textures, and the console are in place. A small test level was built, saved and reopened on Windows with nothing lost, which meets the exit criterion.

Carried over, not blocking Phase 2: i18n (EDT-040), Build and Project Settings (EDT-010; Build waits for Phase 3), and the items marked partial in the [backlog](backlog/README.md). Physics and audio nodes (SCN-013, SCN-014), visual print nodes (CON-004) and script attachment (INS-005) moved to Phase 2.

- Unity-style theme and layout: menu bar, toolbar with Play/Pause/Step, Scene/Game tabs, dockable panels (EDT-001…EDT-030).
- Node tree model in Rust with types, UIDs, parenting, scene instancing (SCN-001…SCN-020).
- Inspector driven by node type metadata, Add Node/Component menu (INS-001…INS-020).
- Project browser and asset import with thumbnails and hot reload (AST-001…AST-015).
- Gizmos: move, rotate, scale, rect tool, snapping (VPT-001…VPT-015).
- Unity-style Console (CON-001…CON-010).

**Exit:** build a level with real sprites as a node tree, save, reopen, nothing lost.

## Phase 2: Physics, input, audio, visual logic (in progress)

Status: the physics runtime (Rapier 2D: static, rigid and character bodies, areas, rectangle and circle shapes, layers and masks, fixed step with interpolation) and the input action map are in `nexo-physics` and `nexo-core`, tested headless, and collision shapes show in the Scene view. The visual scripting core (graph model, node library, IR and VM) is in `nexo-graph`, tested headless, but has no Logic workspace yet. Still open: Project Settings for physics and input, the Logic workspace, and audio. See the [backlog](backlog/README.md).

- Rapier 2D: bodies, shapes, layers/masks, materials, joints, raycasts, debug draw (PHY).
- Input map and actions (INP), audio players and buses (AUD).
- GameMaker-style Logic workspace: script list, events, infinite canvas, node search, IR + VM, live debugging (VIS).

**Exit:** someone who has never coded makes a playable platformer with coins in 30 minutes.

**Done when:** every P0 and P1 item of [physics](backlog/physics.md), [input](backlog/input.md), [audio](backlog/audio.md) and [visual scripting](backlog/visual-scripting.md) is checked, plus SCN-013, SCN-014, SCN-033, SCN-034, INS-003, INS-005, CON-004, DBG-003, DBG-004, VPT-005 and EDT-023; and the exit above is demonstrated. PHY-013 (tilemap collision) waits for tilemaps in Phase 3. P2 items stay after 1.0.

## Phase 3: Public alpha

Desktop + web export (EXP), tilemaps (R2D), sprite animation and AnimationPlayer (ANI), game UI nodes (GUI), particles, profiler basics (DBG), three example projects, node reference docs.

**Done when:** every P0 and P1 item of [export](backlog/export.md), [rendering 2D](backlog/rendering-2d.md), [animation](backlog/animation.md), [game UI](backlog/game-ui.md) and [console and debug](backlog/console-debug.md) is checked, plus DOC-006 and DOC-007 (three example projects). Two items need things outside the repository: DOC-004 (screenshots of the Unity editor) and the itch.io publication of the exit game. P2 items stay after 1.0.

**Exit:** a small complete game published on itch.io made only with Nexo.

## Phase 4: Depth (1.0)

User behaviors, eject to C#, graph diff, 2D lights and shadows, post-processing, shader graph, localization, full profiler.

## Phase 5: Expansion

Hand-written C#, mobile, initial 3D, asset/extension store, multiplayer.
