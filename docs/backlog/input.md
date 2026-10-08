# Input

Action-based input, like Godot's Input Map and Unity's Input System.

Legend: **P0** next phase · **P1** before 1.0 · **P2** after 1.0. References: **U** Unity, **G** Godot, **GM** GameMaker.

- [ ] **INP-001** (P0) Input Map in Project Settings: actions (Jump, Left, Right...) bound to keys, mouse buttons, gamepad buttons/axes. *G: Input Map; U: Input Actions.* *Status (Fase 2): parcial: modelo de ações com teclas (InputMap, em RON) e leitura durante a reprodução. Faltam os gamepads e o botão do mouse.* *Status (Fase 2, tela): a aba Entrada de Configurações do Projeto cria e remove ações, liga e desliga teclas por captura do próximo keydown e grava em settings.ron.*
- [ ] **INP-002** (P0) Default actions (ui_left, ui_right, ui_accept...) created with new projects. *Status (Fase 2): parcial: ações padrão no código (ui_left, ui_right, ui_up, ui_down, ui_accept, ui_cancel). Ainda não são gravadas nos projetos novos.*
- [ ] **INP-003** (P0) Visual nodes: Is Action Pressed / Just Pressed / Released, Get Axis, Get Vector, Mouse position (world/screen). *Status (Fase 2): parcial: os nós Ação pressionada, Acabou de ser pressionada, Ação soltada, Eixo, Vetor de entrada e Posição do mouse (mundo e tela) existem no grafo, com o contrato no `Host` (`nexo-graph/src/vm.rs`). Falta o Host da reprodução responder com a entrada real.*
- [ ] **INP-004** (P1) Touch input and virtual joystick node.
- [x] **INP-005** (P1) Rebinding at runtime and saving bindings. *Status (Fase 2): `rebind(action, key)` no editor WASM troca as teclas da ação e vale para o jogo em curso; o botão Substituir da aba Entrada usa esse caminho. As ligações são gravadas em settings.ron.*
- [ ] **INP-006** (P2) Local multiplayer device assignment.
