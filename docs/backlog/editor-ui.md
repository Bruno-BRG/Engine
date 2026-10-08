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

- [x] **EDT-010** (P0) Menu bar: File (New/Open/Save scene, Build), Edit (Undo/Redo/Cut/Copy/Paste/Duplicate/Delete, Project Settings, Preferences), Assets, Node, Window, Help. *Partial: menu bar with Arquivo, Editar (incl. Cut/Copy/Paste), Node, Assets, Janela, Ajuda, built from the command list. Missing: Build, Project Settings, Preferences.* *Status (Fase 1): parcial: menu com Arquivo, Editar, Node, Assets, Janela e Ajuda. Compilar fica desabilitado até a Fase 3. Configurações do projeto e Preferências (P1) faltam.* *Status (Fase 2): Editar > Configurações do Projeto… abre a janela (EDT-023). Compilar segue na Fase 3; Preferências é EDT-024 (P1).*
- [x] **EDT-011** (P0) Main toolbar: tool group (hand, move, rotate, scale, rect), Play / Pause / Step centered, layout selector on the right. *U: Toolbar.* *Partial: Play / Pause / Step centered (F5, F6, F10). Missing: rotate, scale and rect tools, layout selector.* *Status (Fase 1): Mão, Mover, Girar, Escala e Retângulo (Q a T); Play, Pausa e Passo centralizados; seletor de layout à direita.*
- [x] **EDT-012** (P0) Center tabs: Scene, Game, Logic. Game tab shows the game camera at the project resolution with aspect presets (16:9, 4:3, free) and scale slider. *U: Game view.* *Status (Fase 1): aba Jogo com a câmera atual, proporções 16:9, 4:3 e livre, e escala. Verificada no desktop.*
- [ ] **EDT-013** (P0) Status bar: last console message (click opens Console), save state, background tasks progress. *Partial: last console message, click opens Console. Missing: background tasks progress.* *Status (Fase 1): implementado: última mensagem do console e tarefas em execução. Progresso de tarefa não testado ponta a ponta.*
- [ ] **EDT-014** (P1) Dockable panels: drag tabs between areas, split, float into separate windows, close/reopen from Window menu. *U: docking; G 4.6: floating docks.*
- [ ] **EDT-015** (P1) Saved layouts (Default, 2 by 3, Tall, Wide, custom). *U: Layout menu.*
- [ ] **EDT-016** (P1) Maximize panel on Shift+Space. *U.*

## Editor-wide behaviors

- [ ] **EDT-020** (P0) Panel focus model: focused panel gets highlighted tab; selection color depends on focus. *Status (Fase 1): implementado: o painel focado usa a seleção cheia. Não verificado a fundo.*
- [ ] **EDT-021** (P0) Context menus everywhere (Hierarchy, Project, Inspector headers, viewport). *Status (Fase 1): implementado no Hierarchy, no viewport, no Inspector e no painel de Projeto. Verificado no Hierarchy.*
- [ ] **EDT-022** (P0) Keyboard shortcuts editor (Edit > Shortcuts) with conflicts detection. *U: Shortcuts Manager.* *Status (Fase 1): implementado: editor de atalhos com captura de tecla e detecção de conflito. Não testado no desktop.*
- [x] **EDT-023** (P1) Project Settings window: General, Display, Input Map, Physics 2D (gravity, layer matrix), Audio, Layers & Tags. *U: Project Settings; G: Project Settings.* *Status (Fase 2): janela com abas Geral, Entrada, Física 2D, Áudio e Camadas e Tags, gravada em settings.ron. Exibição (Display) não entrou na Fase 2. Os valores de física só afetam o jogo quando o agente de física ler as configurações (ver PHY-021).*
- [ ] **EDT-024** (P1) Preferences window: theme, UI scale, external tools, autosave interval.
- [ ] **EDT-025** (P1) Local history: restore previous versions of a scene. 
- [ ] **EDT-030** (P2) Plugin API for editor panels.

## Language

- [ ] **EDT-040** (P0) i18n: English default, Portuguese translation, all UI strings through keys. *Status (Fase 1): não iniciado: as strings continuam fixas em português.*
