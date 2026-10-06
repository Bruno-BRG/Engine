# Unity

Reference for **visual design and editor layout**. Nexo copies the familiarity, not the GameObject + component data model (we use Godot nodes).

## Version context

Unity 6.6 focused on 2D ([what's new](https://docs.unity.com/en-us/engine/6000.7/manual/whats-new/unity66), [release notes](https://unity.com/releases/editor/whats-new/6000.6.0f1)):

- Sprite Editor and Skinning Editor improvements.
- Tile Palette with filtering and locking to the active Tilemap.
- 2D lights that target objects by light layers.
- 2D shadows for skinned (bone-animated) sprites.
- New Profiler modules for 2D animation, tilemaps and lighting.
- Tighter sprite outline generation.

## Editor layout to replicate

- **Toolbar** with transform tools on the left, Play / Pause / Step in the center, layout menu on the right.
- **Hierarchy** (left): tree of objects, Create menu on "+" and right-click, search with type filter.
- **Scene / Game** tabs (center): Scene for editing with gizmos and overlays; Game shows the camera at a chosen aspect.
- **Inspector** (right): header (active toggle, name, tag, layer), one collapsible section per component, "Add Component" at the bottom.
- **Project** (bottom): two columns, folder tree and asset grid with a size slider.
- **Console** (bottom tab): Clear, Collapse, Error Pause, counters for logs/warnings/errors, detail pane.
- Play mode tint so it is obvious edits will be lost.

Nexo mapping: [editor-ui](../backlog/editor-ui.md), [inspector](../backlog/inspector.md), [assets](../backlog/assets.md), [console-debug](../backlog/console-debug.md).

## Visual language

Neutral grays, dense 20 px rows, almost square controls, blue selection, flat panels separated by 1 px darker borders. Proposed tokens are in [05-editor-ui.md](../05-editor-ui.md). Values are our approximation of the dark skin, not official Unity constants.

## Physics 2D (Inspector reference)

- **Rigidbody2D** puts an object's colliders under physics control. Body types: **Dynamic** (gravity and forces), **Kinematic** (moved by code), **Static** (does not move). **Simulated** enables/disables the body; visible properties change with the body type ([bodyType](https://docs.unity3d.com/cn/6000.0/ScriptReference/Rigidbody2D-bodyType.html)).
- **Collider2D** defines the shape, with **Material**, **Is Trigger** and **Offset** ([Tilemap Collider 2D reference](https://docs.unity3d.com/6000.0/Documentation/Manual/tilemaps/work-with-tilemaps/tilemap-collider-2d-reference.html)).
- Unity 6 adds per-collider layer rules: **Include Layers**, **Exclude Layers**, **Layer Override Priority** ([includeLayers](https://docs.unity3d.com/cn/6000.0/ScriptReference/Rigidbody2D-includeLayers.html), [layerOverridePriority](https://docs.unity3d.com/cn/6000.0/ScriptReference/Collider2D-layerOverridePriority.html)).
- **Project Settings > Physics 2D** holds gravity and the **Layer Collision Matrix** ([Physics 2D settings](https://docs.unity3d.com/ja/2022.2/Manual/class-Physics2DManager.html)).

Nexo mapping: [physics](../backlog/physics.md) PHY-003, PHY-010, PHY-021, PHY-022.

## What we do differently

- Nodes instead of GameObject + components: the "Add Component" button adds a typed child node.
- Fast startup and no domain reload: logic runs in the VM with hot reload.
- Visual scripting is first class, not a package.

## Pending research

Searches about the Hierarchy create menu and the Console window returned no results in the first pass. Redo them and capture screenshots (DOC-004).
