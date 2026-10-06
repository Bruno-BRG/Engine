# Editor UI (Unity style)

Goal: someone coming from Unity feels at home in seconds. Layout and density from Unity, node workflow from Godot. Design reference: [05-editor-ui.md](../05-editor-ui.md), [research/unity.md](../research/unity.md).

Legend: **P0** next phase · **P1** before 1.0 · **P2** after 1.0. References: **U** Unity, **G** Godot, **GM** GameMaker.

## Theme and design system

- [ ] **EDT-001** (P0) Unity-like dark theme tokens: window #191919, panel #383838, toolbar #282828, field #2A2A2A, hover #454545, selection #2C5D87, unfocused selection #4D4D4D, border #232323, text #D2D2D2. Keep the Phase 0 purple theme as an alternative. *U: Editor dark skin.*
- [ ] **EDT-002** (P0) Radius 2–3 px on controls, 0 on panels/tabs; 20 px rows; 12 px UI font, 11 px labels; 16 px icons.
- [ ] **EDT-003** (P0) Node-family icon set (2D blue, physics orange, UI green, audio yellow, script purple). *G: class icons.*
- [ ] **EDT-004** (P1) Light theme with the same structure.
- [ ] **EDT-005** (P1) Editor tint while playing (configurable color). *U: Preferences > Colors > Playmode tint.*

## Window structure

- [ ] **EDT-010** (P0) Menu bar: File (New/Open/Save scene, Build), Edit (Undo/Redo/Cut/Copy/Paste/Duplicate/Delete, Project Settings, Preferences), Assets, Node, Window, Help.
- [ ] **EDT-011** (P0) Main toolbar: tool group (hand, move, rotate, scale, rect), Play / Pause / Step centered, layout selector on the right. *U: Toolbar.*
- [ ] **EDT-012** (P0) Center tabs: Scene, Game, Logic. Game tab shows the game camera at the project resolution with aspect presets (16:9, 4:3, free) and scale slider. *U: Game view.*
- [ ] **EDT-013** (P0) Status bar: last console message (click opens Console), save state, background tasks progress.
- [ ] **EDT-014** (P1) Dockable panels: drag tabs between areas, split, float into separate windows, close/reopen from Window menu. *U: docking; G 4.6: floating docks.*
- [ ] **EDT-015** (P1) Saved layouts (Default, 2 by 3, Tall, Wide, custom). *U: Layout menu.*
- [ ] **EDT-016** (P1) Maximize panel on Shift+Space. *U.*

## Editor-wide behaviors

- [ ] **EDT-020** (P0) Panel focus model: focused panel gets highlighted tab; selection color depends on focus.
- [ ] **EDT-021** (P0) Context menus everywhere (Hierarchy, Project, Inspector headers, viewport).
- [ ] **EDT-022** (P0) Keyboard shortcuts editor (Edit > Shortcuts) with conflicts detection. *U: Shortcuts Manager.*
- [ ] **EDT-023** (P1) Project Settings window: General, Display, Input Map, Physics 2D (gravity, layer matrix), Audio, Layers & Tags. *U: Project Settings; G: Project Settings.*
- [ ] **EDT-024** (P1) Preferences window: theme, UI scale, external tools, autosave interval.
- [ ] **EDT-025** (P1) Local history: restore previous versions of a scene. 
- [ ] **EDT-030** (P2) Plugin API for editor panels.

## Language

- [ ] **EDT-040** (P0) i18n: English default, Portuguese translation, all UI strings through keys.
