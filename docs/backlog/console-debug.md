# Console, debugger and profiler

Tools to understand what the game is doing.

Legend: **P0** next phase · **P1** before 1.0 · **P2** after 1.0. References: **U** Unity, **G** Godot, **GM** GameMaker.

## Console (Unity style)

- [ ] **CON-001** (P0) Toolbar: Clear, Collapse, Clear on Play, Error Pause, search, toggle buttons with counts for Log / Warning / Error. *U: Console window.*
- [ ] **CON-002** (P0) Two-pane layout: message list on top, full message + stack/source on the bottom.
- [ ] **CON-003** (P0) Double-click a message jumps to the source node in a visual script or to the node in the Hierarchy.
- [ ] **CON-004** (P0) `Print`, `Warn`, `Error` visual nodes write to the Console.
- [ ] **CON-005** (P1) Persist the last session log; export to file.

## Debugger

- [ ] **DBG-001** (P1) Remote scene tree during play: live nodes, edit properties at runtime. *G: Remote tree.*
- [ ] **DBG-002** (P1) Pause, step one frame, inspect.
- [ ] **DBG-003** (P1) Visual script debugging: highlighted wires, pin values, breakpoints, call stack (see [visual-scripting.md](visual-scripting.md)).
- [ ] **DBG-004** (P1) Physics debug draw toggle (shapes, contacts, velocities).

## Profiler

- [ ] **DBG-010** (P2) Frame time graph (CPU script, physics, render), memory, draw calls. *U: Profiler; G: Debugger > Profiler.*
- [ ] **DBG-011** (P2) Per-script and per-node cost; most expensive graphs.
