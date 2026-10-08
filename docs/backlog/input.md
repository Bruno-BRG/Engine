# Input

Action-based input, like Godot's Input Map and Unity's Input System.

Legend: **P0** next phase · **P1** before 1.0 · **P2** after 1.0. References: **U** Unity, **G** Godot, **GM** GameMaker.

- [x] **INP-001** (P0) Input Map in Project Settings: actions (Jump, Left, Right...) bound to keys, mouse buttons, gamepad buttons/axes. *G: Input Map; U: Input Actions.* *Status (Fase 2): feito: teclas, botões do mouse, botões e cruz do controle, analógicos e gatilhos LT e RT (eixos sem sinal), vários controles ao mesmo tempo (botões em união; cada eixo pelo de maior módulo) e captura de botão, gatilho ou analógico na aba Entrada. Validado com testes Rust (25 no módulo input, 336 na suíte), svelte-check e uma checagem em Node da lógica de dispositivos com controles simulados (script fora do repositório). Não testado com controle real, no navegador nem no app desktop. O navegador só mostra o controle depois de um aperto de botão. Controles sem mapeamento 'standard' têm índices de botão errados. Deriva de um controle move o personagem, pois a leitura é em união, não por jogador (INP-006).*
- [x] **INP-002** (P0) Default actions (ui_left, ui_right, ui_accept...) created with new projects. *Status (Fase 2): feito: os padrões são gravados no settings.ron de todo projeto novo, com teste.*
- [x] **INP-003** (P0) Visual nodes: Is Action Pressed / Just Pressed / Released, Get Axis, Get Vector, Mouse position (world/screen). *Status (Fase 2): feito: os nós leem bordas reais (acabou de ser pressionada e soltou), eixos, vetor e posição do mouse (mundo e tela) pelo Host. Validado com fumaça em Node, não no app desktop.*
- [ ] **INP-004** (P1) Touch input and virtual joystick node. *Status (Fase 2): parcial: joystick virtual desenhado sobre o viewport na reprodução, oculto no desktop a não ser com a opção. Falta testar o toque em um dispositivo real.*
- [x] **INP-005** (P1) Rebinding at runtime and saving bindings. *Status (Fase 2): `rebind(action, key)` no editor WASM troca as teclas da ação e vale para o jogo em curso; o botão Substituir da aba Entrada usa esse caminho. As ligações são gravadas em settings.ron.*
- [ ] **INP-006** (P2) Local multiplayer device assignment.
