# Physics 2D

Rapier 2D inside the runtime, exposed through Godot-style physics nodes and a Unity-style settings UI. See [research](../research/README.md) for references.

Legend: **P0** next phase · **P1** before 1.0 · **P2** after 1.0. References: **U** Unity, **G** Godot, **GM** GameMaker.

## Bodies

- [ ] **PHY-001** (P0) Integrate Rapier 2D in `nexo-physics`, fixed timestep (60 Hz default), interpolation for rendering.
- [ ] **PHY-002** (P0) `StaticBody2D`: immovable, with constant linear/angular velocity option (conveyor belts). *G.*
- [ ] **PHY-003** (P0) `RigidBody2D`: mass, gravity scale, linear/angular damp, lock rotation, continuous collision detection, sleeping, freeze modes. *U: Rigidbody2D Dynamic; G: RigidBody2D.*
- [ ] **PHY-004** (P0) `CharacterBody2D`: `velocity`, `move_and_slide`, `is_on_floor/wall/ceiling`, floor angle, snap, platform carry. Gravity is not automatic, as in Godot. *G: CharacterBody2D; U: Kinematic body.*
- [ ] **PHY-005** (P0) `Area2D`: overlap detection, gravity/damp override, signals `body_entered`, `body_exited`, `area_entered`. *G: Area2D; U: Is Trigger.*

## Shapes

- [ ] **PHY-010** (P0) `CollisionShape2D` with Rectangle, Circle, Capsule, Segment shape resources; offset; disabled; one-way collision.
- [ ] **PHY-011** (P0) `CollisionPolygon2D` (convex decomposition).
- [ ] **PHY-012** (P0) Editable shape gizmos in the viewport (drag handles).
- [ ] **PHY-013** (P1) Tilemap collision generated from tile data.

## Filtering and settings

- [ ] **PHY-020** (P0) Collision layers and masks (32 named layers) on every body/area. *G: layers/masks.*
- [ ] **PHY-021** (P0) Project Settings > Physics 2D: gravity vector, layer names, Layer Collision Matrix. *U: Physics 2D settings.*
- [ ] **PHY-022** (P1) Per-body include/exclude layer overrides with priority. *U 6: Include/Exclude Layers, Layer Override Priority.*
- [ ] **PHY-023** (P1) `PhysicsMaterial` resource: friction, bounce, rough, absorbent.

## Queries and joints

- [ ] **PHY-030** (P1) `RayCast2D` and `ShapeCast2D` nodes; raycast/overlap visual nodes.
- [ ] **PHY-031** (P1) Joints: pin, groove, damped spring.
- [ ] **PHY-032** (P1) Collision events available to visual scripts (contact point, normal, other node).
- [ ] **PHY-033** (P2) Deterministic mode for replays/networking.
