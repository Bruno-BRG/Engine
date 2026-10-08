# Inspector

Shows and edits the properties of the selected node, generated from node type metadata. Unity look, Godot data.

Legend: **P0** next phase · **P1** before 1.0 · **P2** after 1.0. References: **U** Unity, **G** Godot, **GM** GameMaker.

- [x] **INS-001** (P0) Header: active checkbox, editable name, node type, Tag and Layer dropdowns, static toggle. *U: GameObject header.* *Status (Fase 1): cabeçalho com ativo, nome, estático, Tag e Layer (listas fixas até o gerenciador de tags).* *Status (Fase 2): Tag e Layer vêm das Configurações do Projeto (SCN-033).*
- [x] **INS-002** (P0) Sections per class in the inheritance chain (e.g. Node2D, then CharacterBody2D), collapsible, with icon and kebab menu (Reset, Copy/Paste values, Docs). *U: component headers; G: inspector categories.* *Status (Fase 1): uma seção por classe da cadeia de herança, com menu de opções (Restaurar valores). Copiar valores e documentação ficam desabilitados.*
- [ ] **INS-003** (P0) Field widgets by type: float/int with scrub, Vector2 (X/Y), bool, enum dropdown, color, string, range slider, flags (layer bitmask grid), resource slot, node path picker. *Status (Fase 1): parcial: número, vetor, booleano, enumeração, cor, texto e textura. Faltam a grade de flags, o slot de recurso arrastável e o seletor de caminho de nó.* *Status (Fase 2): parcial: grade de máscara de bits (32 camadas com nomes do projeto) para collision_mask. Faltam include/exclude (PHY-022 ainda não tem essas propriedades), o slot de recurso arrastável e o seletor de caminho de nó.*
- [ ] **INS-004** (P0) "Add Node / Add Component" button at the bottom opening searchable categories (creates a child node of the chosen type). *U: Add Component.* *Status (Fase 1): implementado (Adicionar componente cria um nó filho), ainda não testado no desktop.*
- [ ] **INS-005** (P0) Script slot: attach new/existing visual script, open in Logic, list exposed variables. *U: script component; G: script property.* *Status (Fase 2): pendente: depende da programação visual.*
- [ ] **INS-006** (P1) Resource slots accept drag from Project; inline sub-resource editing (shape, material). *G: sub-resources.*
- [ ] **INS-007** (P1) Multi-object editing with mixed-value display (—).
- [ ] **INS-008** (P1) Prefab overrides: bold labels and per-field revert.
- [ ] **INS-009** (P1) Lock inspector to the current selection. *U: lock icon.*
- [ ] **INS-010** (P1) Property tooltips from node docs; right-click "Copy property path" for visual scripting.
- [ ] **INS-011** (P1) Inspector for assets (texture import settings, audio import settings).
- [ ] **INS-012** (P2) Debug mode showing private/runtime fields during play. *U: Debug inspector.*
