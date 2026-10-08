# Physics 2D

Rapier 2D inside the runtime, exposed through Godot-style physics nodes and a Unity-style settings UI. See [research](../research/README.md) for references.

Legend: **P0** next phase · **P1** before 1.0 · **P2** after 1.0. References: **U** Unity, **G** Godot, **GM** GameMaker.

## Bodies

- [x] **PHY-001** (P0) Integrate Rapier 2D in `nexo-physics`, fixed timestep (60 Hz default), interpolation for rendering.
- [x] **PHY-002** (P0) `StaticBody2D`: immovable, with constant linear/angular velocity option (conveyor belts). *G.* *Status (Fase 2): feito: esteira com velocidade linear e angular, com ponto de contato por contato e teste de disco que gira.*
- [x] **PHY-003** (P0) `RigidBody2D`: mass, gravity scale, linear/angular damp, lock rotation, continuous collision detection, sleeping, freeze modes. *U: Rigidbody2D Dynamic; G: RigidBody2D.*
- [x] **PHY-004** (P0) `CharacterBody2D`: `velocity`, `move_and_slide`, `is_on_floor/wall/ceiling`, floor angle, snap, platform carry. Gravity is not automatic, as in Godot. *G: CharacterBody2D; U: Kinematic body.* *Status (Fase 2): feito: colagem ao chão configurável (floor_snap_length), carga por plataforma e ângulo do chão. is_on_floor, wall e ceiling já existiam.*
- [x] **PHY-005** (P0) `Area2D`: overlap detection, gravity/damp override, signals `body_entered`, `body_exited`, `area_entered`. *G: Area2D; U: Is Trigger.*

## Shapes

- [x] **PHY-010** (P0) `CollisionShape2D` with Rectangle, Circle, Capsule, Segment shape resources; offset; disabled; one-way collision.
- [x] **PHY-011** (P0) `CollisionPolygon2D` (convex decomposition).
- [ ] **PHY-012** (P0) Editable shape gizmos in the viewport (drag handles). *Status (Fase 2): parcial: alças para arrastar as formas no viewport, validadas no navegador. Falta validar o snap opcional.*
- [ ] **PHY-013** (P1) Tilemap collision generated from tile data.

## Filtering and settings

- [x] **PHY-020** (P0) Collision layers and masks (32 named layers) on every body/area. *G: layers/masks.* *Status (Fase 2): feito: a física usa os 32 nomes e a matriz de ProjectSettings; o Inspector mostra a grade com os nomes, sem número de bit.*
- [x] **PHY-021** (P0) Project Settings > Physics 2D: gravity vector, layer names, Layer Collision Matrix. *U: Physics 2D settings.* *Status (Fase 2): gravidade, nomes das camadas e matriz são editados em Configurações do Projeto > Física 2D, gravados em settings.ron e lidos pela física a cada Jogar.*
- [x] **PHY-022** (P1) Per-body include/exclude layer overrides with priority. *U 6: Include/Exclude Layers, Layer Override Priority.* *Status (Fase 2): feito: include e exclude são seletores de camadas por nome no Inspector.*
- [ ] **PHY-023** (P1) `PhysicsMaterial` resource: friction, bounce, rough, absorbent. *Status (Fase 2): parcial: a física aplica atrito e quique do material; o Inspector escolhe o material numa lista e avisa se o nome não existe. Falta atualizar as referências ao renomear um material.*

## Queries and joints

- [ ] **PHY-030** (P1) `RayCast2D` and `ShapeCast2D` nodes; raycast/overlap visual nodes. *Status (Fase 2): parcial: nós RayCast2D e ShapeCast2D (alvo, máscara, ativo, atingir áreas), consultas por nó no WASM (`ray_hit`, `shape_hit`) e consultas livres (`raycast`, `overlap_point`, `overlap_rect`). Faltam os nós de consulta na programação visual (depende da VM) e o desenho do raio no viewport.*
- [x] **PHY-031** (P1) Joints: pin, groove, damped spring.
- [ ] **PHY-032** (P1) Collision events available to visual scripts (contact point, normal, other node). *Status (Fase 2): parcial: eventos de colisão sólida chegam à fila (lado a lado). Faltam ponto de contato e normal.*
- [ ] **PHY-033** (P2) Deterministic mode for replays/networking.
