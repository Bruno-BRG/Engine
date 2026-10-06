# Engine research

Study of the engines Nexo wants to rival, focused on what to copy and what to improve. Collected in October 2026 from official documentation, release notes and blogs. Every claim links to its source.

| Engine | File | What we take |
|---|---|---|
| Unity | [unity.md](unity.md) | Visual language and layout of the editor, Inspector components, Console, physics settings UI |
| Godot | [godot.md](godot.md) | Technical model: everything is a node, scenes, signals, physics nodes, UIDs (open source, MIT) |
| GameMaker | [gamemaker.md](gamemaker.md) | Logic workflow: objects with events, scripts as assets, visual code |
| Others | [others.md](others.md) | Unreal Blueprints, Construct, GDevelop, Defold, Bevy |

Screenshots live in [../assets/research/](../assets/research/). Capturing more reference screenshots is tracked in DOC-004.

## Current versions (October 2026)

| Engine | Version | Source |
|---|---|---|
| Unity | 6.6 (2D focus release) | [What's new in Unity 6.6](https://docs.unity.com/en-us/engine/6000.7/manual/whats-new/unity66) |
| Godot | 4.7.2 stable (Aug 2026), 4.8 in testing | [Download archive](https://godotengine.org/download/archive/) |
| GameMaker | LTS 2026.0, monthly 2026.100 | [LTS 2026](https://gamemaker.io/en/blog/lts-2026-release) |

## Feature matrix

| Feature | Unity | Godot | GameMaker | **Nexo plan** |
|---|---|---|---|---|
| Visual programming as main path | No | No (VisualScript removed in 4.0) | Partial (GML Visual) | **Yes** (VIS) |
| Node graphs | Visual Scripting package | No | No (blocks) | **Yes** |
| Event-based objects | No | Signals | Yes | **Yes** (events per script) |
| Scene = tree of typed nodes | GameObject + components | Yes | Objects + rooms | **Yes** (SCN) |
| Prefabs / scene instancing | Prefabs with overrides | Instanced scenes | Partial | **Yes** (SCN-030) |
| Dockable editor panels | Yes | Yes (floating since 4.6) | Yes | **Yes** (EDT-014) |
| Built-in sprite editor | Sprite Editor (slicing) | No | Yes | **Phase 3** |
| Tilemaps | Yes | Yes | Yes | **Yes** (R2D-003) |
| 2D lights and shadows | Yes | Yes | Limited | **P2** |
| Code view of visual logic | No | No | GML | **C#** |
| Web export | Yes | Yes | Yes | **Yes** |
| Light editor | No | Yes | Yes | **Yes** |

## Key conclusions

1. Visual programming only works with **game-level abstractions** (physics nodes, events, built-in behaviors), not a 1:1 translation of code. Godot removing VisualScript in 4.0 is the warning.
2. GameMaker is not investing in GML Visual; its roadmap goes to a new runtime and to JavaScript/TypeScript and C# ([spring 2026 update](https://gamemaker.io/en/blog/update-spring-2026)). There is room for a visual-first engine.
3. Unity's current investment is 2D tooling (tiles, lights, skinning, profiler modules). That is the bar for Nexo's 2D.
4. Godot's node model plus Unity's editor familiarity is a combination no engine offers today.
