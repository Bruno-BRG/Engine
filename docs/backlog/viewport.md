# Scene viewport and gizmos

Editing scenes in the Scene tab.

Legend: **P0** next phase · **P1** before 1.0 · **P2** after 1.0. References: **U** Unity, **G** Godot, **GM** GameMaker.

- [ ] **VPT-001** (P0) Transform tools: Hand (Q), Move (W), Rotate (E), Scale (R), Rect (T) with Unity-style gizmos. *U: transform tools.* *Status (Fase 1): implementado: Mão Q, Mover W, Girar E, Escala R, Retângulo T. Mover e girar verificados no desktop; escala e retângulo não.*
- [ ] **VPT-002** (P0) Snapping: grid snap, rotation snap, configurable increments; hold Ctrl to toggle.  *Status (Fase 1): implementado: snap de grade (16 px) e de rotação (15°), Ctrl inverte. Ainda não testado no desktop.*
- [ ] **VPT-003** (P0) Box selection, multi-select, Alt-drag duplicate. *Status (Fase 1): implementado: caixa de seleção, Shift/Ctrl para somar, Alt-arrastar duplica. Ainda não testado no desktop.*
- [ ] **VPT-004** (P0) Right-click context menu in the viewport (create node here, paste, frame). *Status (Fase 1): implementado: menu de contexto no viewport. Ainda não testado no desktop.*
- [ ] **VPT-005** (P0) Gizmos for colliders, cameras (game frame), audio radius, paths. *U: Gizmos; G: editor plugins.* *Status (Fase 2): parcial. Feito: contornos dos colisores (`CollisionShape2D` e `CollisionPolygon2D` ativos, em `query::shape_outlines`), raio de áudio do `AudioStreamPlayer2D` (`max_distance` × zoom), moldura da câmera atual (tela do jogo ÷ zoom) e caminho do `Path2D` (polilinha aberta da propriedade `points`, em `query::path_outlines`, desenhada em `ViewportOverlay.svelte`). Verificado: testes Rust da geometria (contorno, câmera, atenuação e caminho com escala, rotação e polilinha aberta) e do registro do `Path2D`. Não verificado: o desenho SVG (`ViewportOverlay.svelte`) dos colisores, câmeras, raios e caminhos no desktop ou no navegador, porque o editor não foi aberto nesta sessão. O item fica `[ ]` até essa verificação.*
- [ ] **VPT-006** (P1) Scene view overlay: grid size, gizmo toggles, 2D/3D later. *U: Overlays.*
- [ ] **VPT-007** (P1) Rulers and guides; smart guides when aligning nodes.
- [ ] **VPT-008** (P1) Pixel-perfect preview and pixel grid at high zoom.
- [ ] **VPT-009** (P1) Drag texture from Project creates a Sprite2D; drag scene creates an instance.
- [ ] **VPT-010** (P1) Game tab: real camera view, aspect presets, stats overlay (fps, draw calls).
