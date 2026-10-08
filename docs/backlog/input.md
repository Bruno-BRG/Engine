# Input

Action-based input, like Godot's Input Map and Unity's Input System.

Legend: **P0** next phase · **P1** before 1.0 · **P2** after 1.0. References: **U** Unity, **G** Godot, **GM** GameMaker.

- [ ] **INP-001** (P0) Input Map in Project Settings: actions (Jump, Left, Right...) bound to keys, mouse buttons, gamepad buttons/axes. *G: Input Map; U: Input Actions.* *Status (Fase 2): parcial: teclas, botões do mouse, botões e cruz do controle, e eixos com sinal, lidos a cada quadro da reprodução. Faltam LT e RT, mais de um controle ao mesmo tempo e a captura de botão do controle na aba Entrada.*
- [x] **INP-002** (P0) Default actions (ui_left, ui_right, ui_accept...) created with new projects. *Status (Fase 2): feito: os padrões são gravados no settings.ron de todo projeto novo, com teste.*
- [x] **INP-003** (P0) Visual nodes: Is Action Pressed / Just Pressed / Released, Get Axis, Get Vector, Mouse position (world/screen). *Status (Fase 2): feito: os nós leem bordas reais (acabou de ser pressionada e soltou), eixos, vetor e posição do mouse (mundo e tela) pelo Host. Validado com fumaça em Node, não no app desktop.*
- [ ] **INP-004** (P1) Touch input and virtual joystick node. *Status (Fase 2): parcial: joystick virtual desenhado sobre o viewport na reprodução, oculto no desktop a não ser com a opção. Falta testar o toque em um dispositivo real.*
- [x] **INP-005** (P1) Rebinding at runtime and saving bindings. *Status (Fase 2): `rebind(action, key)` no editor WASM troca as teclas da ação e vale para o jogo em curso; o botão Substituir da aba Entrada usa esse caminho. As ligações são gravadas em settings.ron.*
- [ ] **INP-006** (P2) Local multiplayer device assignment.
