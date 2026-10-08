# Scenes and nodes

Everything is a node (Godot model). A scene is a tree of typed nodes; scenes can be instanced inside other scenes. See [research/godot.md](../research/godot.md).

Legend: **P0** next phase · **P1** before 1.0 · **P2** after 1.0. References: **U** Unity, **G** Godot, **GM** GameMaker.

## Core model (Rust)

- [x] **SCN-001** (P0) Node tree in `nexo-core`: type, name, UID, parent/children, ordered children, typed property bag. Migrate Phase 0 projects (`projeto.nexo`, `cenas/`) to `project.nexo`, `scenes/` automatically. *Status (Fase 1): migração automática do formato 1 verificada no desktop (pastas, backup .v1.bak).*
- [x] **SCN-002** (P0) Node type registry with inheritance and property metadata (type, default, range, hint, group) used by Inspector, serializer and visual scripting. *G: ClassDB + property hints.* *Status (Fase 1): Inspector e menu de criação são gerados do registro de tipos.*
- [x] **SCN-003** (P0) Stable UIDs for nodes and resources so references survive renames and moves. *G 4.6: unique node IDs.* *Status (Fase 1): UIDs de 16 dígitos estáveis; ramo salvo mantém os UIDs; referências de textura verificadas.*
- [x] **SCN-004** (P0) Scene serialization in RON with deterministic order; only non-default properties saved. *Status (Fase 1): RON em pré-ordem, só propriedades não padrão; verificado no arquivo.*
- [x] **SCN-005** (P0) Move the scene state of truth from the UI to Rust (editor commands through Tauri / WASM). *Status (Fase 1): estado da cena no nexo-core, acessado pelo WASM (ADR 0006).*

## Built-in node types (first set)

- [x] **SCN-010** (P0) `Node`, `Node2D` (position, rotation, scale, skew, z-index, visible, modulate). *Status (Fase 1): Node e Node2D com posição, rotação, escala, ordem Z, visibilidade e modulação.*
- [ ] **SCN-011** (P0) `Sprite2D` (texture, region, flip, offset, centered, pixel snap). *Status (Fase 1): parcial: textura, forma de teste, espelhamento, deslocamento e centralização. Faltam região e pixel snap.*
- [ ] **SCN-012** (P0) `Camera2D` (zoom, limits, smoothing, current). *Status (Fase 1): parcial: zoom, câmera atual e suavização. Faltam limites da câmera.*
- [x] **SCN-013** (P0) Physics nodes: `StaticBody2D`, `RigidBody2D`, `CharacterBody2D`, `Area2D`, `CollisionShape2D`, `CollisionPolygon2D` (see [physics.md](physics.md)). *Status (Fase 2): feito: os seis tipos existem no registro e têm física; RayCast2D e ShapeCast2D seguem em PHY-030.*
- [x] **SCN-014** (P0) `Timer`, `AudioStreamPlayer`, `AudioStreamPlayer2D`. *Status (Fase 2): feito. `Timer` (wait_time, one_shot, autostart) conta na reprodução e emite o sinal `timeout` pela fila de sinais (`SceneEditor::timer_events`, em `nexo-core/src/timer.rs`). Ainda não há lógica visual para consumir o sinal. Os players estão em AUD-001.*
- [ ] **SCN-015** (P1) `AnimatedSprite2D`, `AnimationPlayer`, `TileMapLayer`, `Marker2D`, `Path2D`/`PathFollow2D`, `Line2D`, `Polygon2D`, `GPUParticles2D`, `CanvasLayer`, `ParallaxBackground`. *Status (Fase 2): parcial: `Path2D` feito via VPT-005 (registro com a propriedade `points` e polilinha no viewport). Demais tipos pendentes, incluindo `PathFollow2D`.*
- [ ] **SCN-016** (P1) Control nodes for game UI (see [game-ui.md](game-ui.md)).

## Hierarchy panel

- [ ] **SCN-020** (P0) Tree with expand/collapse arrows, indent guides, node-type icons, Unity-style row selection. *Status (Fase 1): parcial: árvore com recolher/expandir, ícones e seleção verificados. Faltam as guias de indentação.*
- [ ] **SCN-021** (P0) Drag to reparent and reorder, with drop indicators (above / inside / below). *Status (Fase 1): implementado (arrastar entre níveis com indicador antes/dentro/depois), ainda não testado no desktop.*
- [ ] **SCN-022** (P0) Right-click menu: Create (by category, with search), Rename (F2), Duplicate (Ctrl+D), Delete, Copy/Paste, Change Type, Attach Script, Save Branch as Scene. *U: Hierarchy context menu; G: Scene dock.* *Status (Fase 1): parcial: criar, renomear, duplicar, excluir, copiar e colar, trocar tipo e salvar ramo como cena. Anexar script fica para a Fase 2.*
- [ ] **SCN-023** (P0) Search/filter by name and by type (`t:Sprite2D`). *U: Hierarchy search.* *Status (Fase 1): implementado (busca por nome e t:Tipo), ainda não testado no desktop.*
- [ ] **SCN-024** (P0) Multi-select (Ctrl/Shift), visibility and lock toggles, script and signal badges. *Status (Fase 1): parcial: seleção múltipla, visibilidade e trava verificadas. Faltam os distintivos de script e sinal (Fase 2).*
- [ ] **SCN-025** (P1) Scene root header with scene name, unsaved dot, and scene-wide actions.

## Scene composition

- [ ] **SCN-030** (P1) Scene instancing (prefabs) with per-property overrides shown in bold, revert per property. *U: Prefab overrides; G: instanced scenes.*
- [ ] **SCN-031** (P1) Open instanced scene for editing, "Editable children".
- [ ] **SCN-032** (P1) Multiple open scenes as tabs; scene switching at runtime.
- [ ] **SCN-033** (P1) Groups/tags on nodes (`enemies`, `coins`) usable by logic. *G: groups; U: tags.* *Status (Fase 2): parcial: tag e camada são dropdowns no Inspector, com os nomes de Configurações do Projeto. Falta a lógica visual procurar nós por tag.*
- [ ] **SCN-034** (P1) Signals: list in a Node tab next to Inspector, connect dialog to target node + script event. *G: Node dock > Signals.* *Status (Fase 2): parcial: aba Nó ao lado do Inspector, com os sinais do nó e as ligações de cada um; diálogo com nó alvo e nome do método; desfazer; ligações removidas ao excluir o alvo; aviso de alvo ausente (ADR 0009). Sinais de temporizador e de física cobertos. Falta o método ser um evento escolhido do script do alvo, em vez de nome digitado: a lista de eventos do script alvo entra com VIS-002 (o evento Ao receber sinal ainda não existe na biblioteca), e o despacho das ligações ao script também é dele.*
- [ ] **SCN-035** (P2) Autoloads/singletons (global nodes). *G: Autoload.*
- [ ] **SCN-036** (P1) Duplicar e colar não remapeiam as referências a nós nas propriedades (ex.: `node_b` das juntas, `PropKind::Uid`), então a cópia continua ligada ao nó original. Referências a assets (texturas, streams) não entram nisso e continuam compartilhadas. Pré-existente e independente de sinais: `instantiate` (scene.rs) copia `props` sem remapear. *Status: pendente.*
