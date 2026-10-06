# Inspector

Shows and edits the properties of the selected node, generated from node type metadata. Unity look, Godot data.

Legend: **P0** next phase · **P1** before 1.0 · **P2** after 1.0. References: **U** Unity, **G** Godot, **GM** GameMaker.

- [ ] **INS-001** (P0) Header: active checkbox, editable name, node type, Tag and Layer dropdowns, static toggle. *U: GameObject header.*
- [ ] **INS-002** (P0) Sections per class in the inheritance chain (e.g. Node2D, then CharacterBody2D), collapsible, with icon and kebab menu (Reset, Copy/Paste values, Docs). *U: component headers; G: inspector categories.*
- [ ] **INS-003** (P0) Field widgets by type: float/int with scrub, Vector2 (X/Y), bool, enum dropdown, color, string, range slider, flags (layer bitmask grid), resource slot, node path picker.
- [ ] **INS-004** (P0) "Add Node / Add Component" button at the bottom opening searchable categories (creates a child node of the chosen type). *U: Add Component.*
- [ ] **INS-005** (P0) Script slot: attach new/existing visual script, open in Logic, list exposed variables. *U: script component; G: script property.*
- [ ] **INS-006** (P1) Resource slots accept drag from Project; inline sub-resource editing (shape, material). *G: sub-resources.*
- [ ] **INS-007** (P1) Multi-object editing with mixed-value display (—).
- [ ] **INS-008** (P1) Prefab overrides: bold labels and per-field revert.
- [ ] **INS-009** (P1) Lock inspector to the current selection. *U: lock icon.*
- [ ] **INS-010** (P1) Property tooltips from node docs; right-click "Copy property path" for visual scripting.
- [ ] **INS-011** (P1) Inspector for assets (texture import settings, audio import settings).
- [ ] **INS-012** (P2) Debug mode showing private/runtime fields during play. *U: Debug inspector.*
