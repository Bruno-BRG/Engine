# Scenes and nodes

Everything is a node (Godot model). A scene is a tree of typed nodes; scenes can be instanced inside other scenes. See [research/godot.md](../research/godot.md).

Legend: **P0** next phase · **P1** before 1.0 · **P2** after 1.0. References: **U** Unity, **G** Godot, **GM** GameMaker.

## Core model (Rust)

- [ ] **SCN-001** (P0) Node tree in `nexo-core`: type, name, UID, parent/children, ordered children, typed property bag. Migrate Phase 0 projects (`projeto.nexo`, `cenas/`) to `project.nexo`, `scenes/` automatically.
- [ ] **SCN-002** (P0) Node type registry with inheritance and property metadata (type, default, range, hint, group) used by Inspector, serializer and visual scripting. *G: ClassDB + property hints.*
- [ ] **SCN-003** (P0) Stable UIDs for nodes and resources so references survive renames and moves. *G 4.6: unique node IDs.*
- [ ] **SCN-004** (P0) Scene serialization in RON with deterministic order; only non-default properties saved.
- [ ] **SCN-005** (P0) Move the scene state of truth from the UI to Rust (editor commands through Tauri / WASM).

## Built-in node types (first set)

- [ ] **SCN-010** (P0) `Node`, `Node2D` (position, rotation, scale, skew, z-index, visible, modulate).
- [ ] **SCN-011** (P0) `Sprite2D` (texture, region, flip, offset, centered, pixel snap).
- [ ] **SCN-012** (P0) `Camera2D` (zoom, limits, smoothing, current).
- [ ] **SCN-013** (P0) Physics nodes: `StaticBody2D`, `RigidBody2D`, `CharacterBody2D`, `Area2D`, `CollisionShape2D`, `CollisionPolygon2D` (see [physics.md](physics.md)).
- [ ] **SCN-014** (P0) `Timer`, `AudioStreamPlayer`, `AudioStreamPlayer2D`.
- [ ] **SCN-015** (P1) `AnimatedSprite2D`, `AnimationPlayer`, `TileMapLayer`, `Marker2D`, `Path2D`/`PathFollow2D`, `Line2D`, `Polygon2D`, `GPUParticles2D`, `CanvasLayer`, `ParallaxBackground`.
- [ ] **SCN-016** (P1) Control nodes for game UI (see [game-ui.md](game-ui.md)).

## Hierarchy panel

- [ ] **SCN-020** (P0) Tree with expand/collapse arrows, indent guides, node-type icons, Unity-style row selection.
- [ ] **SCN-021** (P0) Drag to reparent and reorder, with drop indicators (above / inside / below).
- [ ] **SCN-022** (P0) Right-click menu: Create (by category, with search), Rename (F2), Duplicate (Ctrl+D), Delete, Copy/Paste, Change Type, Attach Script, Save Branch as Scene. *U: Hierarchy context menu; G: Scene dock.*
- [ ] **SCN-023** (P0) Search/filter by name and by type (`t:Sprite2D`). *U: Hierarchy search.*
- [ ] **SCN-024** (P0) Multi-select (Ctrl/Shift), visibility and lock toggles, script and signal badges.
- [ ] **SCN-025** (P1) Scene root header with scene name, unsaved dot, and scene-wide actions.

## Scene composition

- [ ] **SCN-030** (P1) Scene instancing (prefabs) with per-property overrides shown in bold, revert per property. *U: Prefab overrides; G: instanced scenes.*
- [ ] **SCN-031** (P1) Open instanced scene for editing, "Editable children".
- [ ] **SCN-032** (P1) Multiple open scenes as tabs; scene switching at runtime.
- [ ] **SCN-033** (P1) Groups/tags on nodes (`enemies`, `coins`) usable by logic. *G: groups; U: tags.*
- [ ] **SCN-034** (P1) Signals: list in a Node tab next to Inspector, connect dialog to target node + script event. *G: Node dock > Signals.*
- [ ] **SCN-035** (P2) Autoloads/singletons (global nodes). *G: Autoload.*
