# Visual scripting

Nexo's core. GameMaker workflow (scripts as assets made of events), Blueprint-style graphs, IR + VM, generated C#. Spec: [04-visual-programming.md](../04-visual-programming.md).

Legend: **P0** next phase · **P1** before 1.0 · **P2** after 1.0. References: **U** Unity, **G** Godot, **GM** GameMaker.

## Logic workspace (GameMaker style)

- [ ] **VIS-001** (P0) Script list panel: Script 1, Script 2... create, rename, duplicate, delete, folders; also listed in the Project browser. *GM: Asset Browser.* *Status (Fase 2): pendente: a aba Lógica ainda não tem interface.*
- [ ] **VIS-002** (P0) Events panel per script: add event menu (On Ready, On Process, On Physics Process, On Collision with..., On Input Action, On Signal, On Draw, On Destroy, On Timer). *GM: Object events Create/Step/Collision/Draw.*
- [ ] **VIS-003** (P0) "Used by" list: which nodes use the script; attach to selected node.
- [ ] **VIS-004** (P0) Keep the Phase 0 look: dotted background, animated dashed execution wires.

## Canvas

- [ ] **VIS-010** (P0) Infinite canvas: pan with middle mouse, Space+drag, or left-drag on empty background; zoom with wheel around cursor; Home to frame all. *Status (Fase 2): pendente: canvas infinito não iniciado.*
- [ ] **VIS-011** (P0) Right-click opens the node search menu (categories + fuzzy search, EN/PT); dropping a wire on empty space opens it filtered by pin type.
- [ ] **VIS-012** (P0) Box selection, move, delete, copy/paste (also between scripts), undo/redo.
- [ ] **VIS-013** (P0) Typed pins with colors and shapes; connection validation with clear errors.
- [ ] **VIS-014** (P1) Comment boxes, reroute nodes, auto-align, minimap, semantic zoom.
- [ ] **VIS-015** (P1) Collapse selection to function; macros.

## Language

- [ ] **VIS-020** (P0) Graph model in `nexo-graph` with serialization (`scripts/*.graph.ron`). *Status (Fase 2): parcial: modelo de grafo com fios, tipos de pino e validação de conexão em `nexo-graph`, com RON. Falta a integração com o editor e a remoção de nós e fios.*
- [ ] **VIS-021** (P0) Node library v1: flow (Branch, Sequence, For Each, While), math, compare, Vector2, string, Print, Get/Set property of any node (from type metadata), Get Node, Instance Scene, Destroy, Input action pressed, Move and Slide, Play sound, Timer start. *Status (Fase 2): parcial: biblioteca v1 com 14 nós (eventos OnReady e OnProcess, Branch, Sequence, Add, Multiply, Greater, Print, Destroy, ActionPressed, GetProperty, SetProperty, GetVariable, SetVariable). Faltam For Each, While, Get Node, Instance Scene, Move and Slide, Play sound e Timer.*
- [ ] **VIS-022** (P0) Variables (script-level, exported to Inspector, local) with types and defaults. *Status (Fase 2): parcial: variáveis com tipo, padrão e marcação de exportação no modelo. Falta mostrá-las no Inspector.*
- [ ] **VIS-023** (P0) Compile to IR, run in `nexo-vm`, hot reload during play. *Status (Fase 2): parcial: compilação para IR e VM com orçamento de passos, testadas com host simulado. Falta ligar a VM à reprodução e a recarga durante o jogo.*
- [ ] **VIS-024** (P0) C# generation from IR, shown in a side panel.
- [ ] **VIS-025** (P1) Latent nodes: Wait, Tween, Move To, Await signal (coroutines in IR).
- [ ] **VIS-026** (P1) Functions with inputs/outputs; custom signals; calling other scripts.
- [ ] **VIS-027** (P1) State machine graph.
- [ ] **VIS-028** (P2) Event sheets (Construct style) over the same model.
- [ ] **VIS-029** (P2) Publish a script as a reusable behavior with Inspector parameters.

## Debugging

- [ ] **VIS-030** (P1) Live execution highlight on wires; pin value on hover.
- [ ] **VIS-031** (P1) Breakpoints, step, instance picker.
- [ ] **VIS-032** (P2) Per-frame execution history.
