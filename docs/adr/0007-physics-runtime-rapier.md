# ADR 0007: Physics runs in the play copy, on Rapier 2D

**Status:** accepted

## Context

Phase 2 needs bodies that fall, collide and carry triggers (PHY-001 to PHY-005). The scene is already the source of truth in `nexo-core` behind WebAssembly (ADR 0006). The physics has to read the node tree, step with the play clock and write the results back, without touching the scene being edited.

Options considered for the engine: a hand-written solver (too much to get right for the first slice), Box2D through FFI (C toolchain in the WebAssembly build), and Rapier 2D (pure Rust, compiles to `wasm32-unknown-unknown`, has a kinematic character controller).

## Decision

- `crates/nexo-physics` wraps `rapier2d` 0.36. It builds the world from the scene nodes: `StaticBody2D` and `Area2D` as fixed bodies, `RigidBody2D` as dynamic, `CharacterBody2D` as kinematic position-based. `CollisionShape2D` children become colliders relative to their body.
- The world is built when play starts, from the play copy (`Scene::play_copy`). Each frame, `play_tick` advances it in fixed 1/60 s steps (at most 8 per frame). Poses are written back into the play copy with `Scene::set_runtime_prop`, which skips the undo history. The edit scene never changes while playing.
- Poses are interpolated between the last two steps, so the display follows the monitor's rate.
- Layers and masks: the node's `layer` names one of the 32 layers in Project Settings, and each layer is one bit. `collision_mask` is a 32-bit mask (-1 for all) saying which layers the node detects. Rapier's interaction groups do the filtering, and the collision matrix from Project Settings decides which layer pairs collide.
- Areas are sensors. Their colliders use all active collision types, so area against area and area against fixed bodies also report. The events are `body_entered`, `body_exited`, `area_entered`, `area_exited`, plus `collision_started` and `collision_ended` for solid pairs. The events queue up and the editor reads them after each frame.
- Characters move with Rapier's `KinematicCharacterController`, with `up` pointing up the screen (`-Y`). Velocity on a blocked axis is zeroed, as in `move_and_slide`.
- Gravity, layer names, the collision matrix and the physics materials come from Project Settings (`settings.ron`, PHY-021 and PHY-023). The world reads them when play starts, so a change in the settings window applies to the next Play.

## Consequences

- The physics state is internal to the play copy. The visual scripting VM (VIS-023) will read and drive it through the same API, not a second copy.
- **Rapier 0.36 gotcha:** calling `detect_collisions` before the first step freezes dynamic bodies (they never integrate). Rapier's own collision pipeline is not required before stepping, so the first real step reports the overlaps that existed at the start. Do not add that call back.
- Edge case: the first step's character queries see an empty broad phase, so a character that starts within one step of a wall can move into it. Documented, not fixed.
- Only rectangles and circles exist. Capsules, segments, polygons, one-way collision and physics materials are still open (PHY-010, PHY-011, PHY-023).
- Interpolation lags by at most one step (about 3 px at 200 px/s).
