# 01. Product vision

## The problem

People who want to make games without knowing how to code choose between easy tools with a low ceiling (Construct, GDevelop) and powerful tools with a steep curve (Unity, Unreal, Godot). Unreal's Blueprints prove visual programming scales to large games, but they come tied to a heavy, AAA-oriented 3D engine. GameMaker has GML Visual, but it is a second-class citizen next to GML code.

## The proposal

Nexo makes **visual programming the primary path**, with the same care other engines give their scripting languages:

- the accessibility of **event-driven objects** from GameMaker (Create, Step, Collision, Draw);
- the power of **node graphs** from Blueprints (execution flow, functions, typed variables);
- Godot's **"everything is a node"** scene model;
- a **Unity-style editor** that professionals already know;
- a bridge to real code: every graph can be viewed as **C#**.

## Audience

| Profile | Needs |
|---|---|
| Beginner / student | Ship a first game in an afternoon, no syntax walls |
| Artist / designer | Prototype mechanics alone |
| Indie developer | Ship a complete 2D game on PC, web and mobile |
| Educator | Teach logic with something visual that becomes real code |

## Differentiators

1. **Two views, one logic.** Event sheets and node graphs over the same model.
2. **First-class visual debugging.** Wires light up, values on pins, breakpoints, per-frame history.
3. **Live C#.** A panel shows the C# of the selected graph, updated on every edit.
4. **Familiar, dense editor.** Unity-style layout and density; Godot-style node tree.
5. **Light.** Small installer, opens in seconds.

## Non-goals (for now)

A AAA 3D engine. 3D arrives after 2D is mature, starting with simple stylized scenes.
