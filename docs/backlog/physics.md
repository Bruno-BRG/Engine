# Physics 2D

Rapier 2D inside the runtime, exposed through Godot-style physics nodes and a Unity-style settings UI. See [research](../research/README.md) for references.

Legend: **P0** next phase · **P1** before 1.0 · **P2** after 1.0. References: **U** Unity, **G** Godot, **GM** GameMaker.

## Bodies

- [x] **PHY-001** (P0) Integrate Rapier 2D in `nexo-physics`, fixed timestep (60 Hz default), interpolation for rendering.
- [ ] **PHY-002** (P0) `StaticBody2D`: immovable, with constant linear/angular velocity option (conveyor belts). *G.* *Status (Fase 2): parcial: velocidade linear constante (esteiras) e carregamento prontos; falta a velocidade angular constante.*
- [x] **PHY-003** (P0) `RigidBody2D`: mass, gravity scale, linear/angular damp, lock rotation, continuous collision detection, sleeping, freeze modes. *U: Rigidbody2D Dynamic; G: RigidBody2D.*
- [ ] **PHY-004** (P0) `CharacterBody2D`: `velocity`, `move_and_slide`, `is_on_floor/wall/ceiling`, floor angle, snap, platform carry. Gravity is not automatic, as in Godot. *G: CharacterBody2D; U: Kinematic body.* *Status (Fase 2): parcial: velocidade, movimento com deslizamento, is_on_floor/wall/ceiling, ângulo do chão, carregamento em plataforma móvel e consulta character_state(uid) no WASM. Falta snap (grudar no chão).*
- [x] **PHY-005** (P0) `Area2D`: overlap detection, gravity/damp override, signals `body_entered`, `body_exited`, `area_entered`. *G: Area2D; U: Is Trigger.*

## Shapes

- [x] **PHY-010** (P0) `CollisionShape2D` with Rectangle, Circle, Capsule, Segment shape resources; offset; disabled; one-way collision.
- [x] **PHY-011** (P0) `CollisionPolygon2D` (convex decomposition).
- [ ] **PHY-012** (P0) Editable shape gizmos in the viewport (drag handles). *Status (Fase 2): pendente: alças de arrasto. Os contornos dos colisores já aparecem na Cena (VPT-005).*
- [ ] **PHY-013** (P1) Tilemap collision generated from tile data.

## Filtering and settings

- [ ] **PHY-020** (P0) Collision layers and masks (32 named layers) on every body/area. *G: layers/masks.* *Status (Fase 2): parcial: a física usa os 32 nomes e a matriz de ProjectSettings. Falta o Inspector mostrar as camadas nomeadas em vez de texto e número de bits.*
- [x] **PHY-021** (P0) Project Settings > Physics 2D: gravity vector, layer names, Layer Collision Matrix. *U: Physics 2D settings.* *Status (Fase 2): gravidade, nomes das camadas e matriz são editados em Configurações do Projeto > Física 2D, gravados em settings.ron e lidos pela física a cada Jogar.*
- [ ] **PHY-022** (P1) Per-body include/exclude layer overrides with priority. *U 6: Include/Exclude Layers, Layer Override Priority.* *Status (Fase 2): parcial: a prioridade está na física; no Inspector, include/exclude ainda são números de bits, não nomes de camadas.*
- [ ] **PHY-023** (P1) `PhysicsMaterial` resource: friction, bounce, rough, absorbent. *Status (Fase 2): parcial: materiais são editados em Configurações do Projeto > Física 2D e a física aplica atrito e quique via physics_material. Falta o Inspector escolher o material de uma lista (hoje é texto) e renomear um material não atualiza as referências.*

## Queries and joints

- [ ] **PHY-030** (P1) `RayCast2D` and `ShapeCast2D` nodes; raycast/overlap visual nodes. *Status (Fase 2): parcial: nós RayCast2D e ShapeCast2D (alvo, máscara, ativo, atingir áreas), consultas por nó no WASM (`ray_hit`, `shape_hit`) e consultas livres (`raycast`, `overlap_point`, `overlap_rect`). Faltam os nós de consulta na programação visual (depende da VM) e o desenho do raio no viewport.*
- [x] **PHY-031** (P1) Joints: pin, groove, damped spring.
- [ ] **PHY-032** (P1) Collision events available to visual scripts (contact point, normal, other node). *Status (Fase 2): parcial: eventos de colisão sólida chegam à fila (lado a lado). Faltam ponto de contato e normal.*
- [ ] **PHY-033** (P2) Deterministic mode for replays/networking.
