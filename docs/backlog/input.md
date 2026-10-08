# Input

Action-based input, like Godot's Input Map and Unity's Input System.

Legend: **P0** next phase · **P1** before 1.0 · **P2** after 1.0. References: **U** Unity, **G** Godot, **GM** GameMaker.

- [ ] **INP-001** (P0) Input Map in Project Settings: actions (Jump, Left, Right...) bound to keys, mouse buttons, gamepad buttons/axes. *G: Input Map; U: Input Actions.* *Status (Fase 2): parcial: modelo de ações com teclas (InputMap, em RON) e leitura durante a reprodução. Faltam a tela em Configurações do Projeto, o salvamento no projeto e os gamepads.*
- [ ] **INP-002** (P0) Default actions (ui_left, ui_right, ui_accept...) created with new projects. *Status (Fase 2): parcial: ações padrão no código (ui_left, ui_right, ui_up, ui_down, ui_accept, ui_cancel). Ainda não são gravadas nos projetos novos.*
- [ ] **INP-003** (P0) Visual nodes: Is Action Pressed / Just Pressed / Released, Get Axis, Get Vector, Mouse position (world/screen). *Status (Fase 2): pendente: depende da lógica visual (VIS).*
- [ ] **INP-004** (P1) Touch input and virtual joystick node.
- [ ] **INP-005** (P1) Rebinding at runtime and saving bindings.
- [ ] **INP-006** (P2) Local multiplayer device assignment.
