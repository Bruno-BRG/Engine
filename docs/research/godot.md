# Godot

Reference for the **technical model**. Godot is open source (MIT), so its source code and docs are a valid technical reference. Nexo reimplements ideas; it does not copy code.

## Version context

- Stable: 4.7.2 (August 2026); 4.8 in testing ([download archive](https://godotengine.org/download/archive/)).
- 4.6 highlights ([release notes](https://godotengine.org/releases/4.6/)): floating/dockable editor docks, unique node IDs that keep references when reorganizing scenes, LibGodot (embed the engine in other apps), Jolt as default 3D physics, new editor theme.

## Editor model

![Godot editor overview](../assets/research/godot-editor-overview.png)

Layout read from the capture: main menu and workspace switch (2D, 3D, Script, Game, AssetLib) on top, playtest buttons on the right; Scene dock with "Create Root Node" (2D Scene, 3D Scene, User Interface, Other Node) and FileSystem on the left; Inspector / Node / History tabs on the right; Output, Debugger, Audio, Animation and Shader Editor panels at the bottom.

- The **Scene** dock shows the node tree of the open scene; the **Inspector** shows the selected node's properties, including variables exported by scripts ([first look at the editor](https://docs.godotengine.org/en/stable/getting_started/introduction/first_look_at_the_editor.html), [Inspector](https://docs.godotengine.org/en/stable/tutorials/editor/inspector_dock.html)).
- Signals are connected from the **Node** tab next to the Inspector ([signals](https://docs.godotengine.org/en/stable/getting_started/step_by_step/signals.html)).

## Everything is a node

- Every element is a node with a class, properties and children; scenes are reusable node trees that can be instanced.
- Nexo adopts: class inheritance chain, property metadata (hints/ranges) driving the Inspector, scene instancing, signals, groups, UIDs.

Typical player:

```text
Player (CharacterBody2D)
├── Sprite2D
├── CollisionShape2D
└── Hurtbox (Area2D)
    └── CollisionShape2D
```

## Physics nodes

- **CharacterBody2D** is moved by code using `velocity` and `move_and_slide()` in `_physics_process()`; gravity is not applied automatically ([CharacterBody2D](https://docs.godotengine.org/en/4.6/classes/class_characterbody2d.html)).
- **Area2D** needs a `CollisionShape2D` child and uses collision layers and masks; it emits `body_entered` when a body enters ([Area2D](https://docs.godotengine.org/en/stable/classes/class_area2d.html), [signals](https://docs.godotengine.org/en/stable/getting_started/step_by_step/signals.html)).

Nexo mapping: [scenes-nodes](../backlog/scenes-nodes.md), [physics](../backlog/physics.md).

## Lessons

- VisualScript was removed in Godot 4.0 for low use. It mirrored GDScript 1:1 and was slower to use than typing. Nexo's visual scripting must offer higher-level nodes and a better workflow (events, search, debugging).

## Pending research

Read the 4.6/4.7 editor docs in detail (docks, Project Settings, Input Map, export presets) and capture screenshots (DOC-004).
