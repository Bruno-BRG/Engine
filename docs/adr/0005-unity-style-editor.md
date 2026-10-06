# ADR 0005: Unity-style editor visuals

**Status:** accepted

## Context

The Phase 0 UI (rounded, purple accent) looked good but did not feel like the professional tool users expect. Users asked for Unity's look: gray, square, dense, organized.

## Decision

Adopt a Unity-like dark theme and layout (menu bar, centered Play controls, Hierarchy, Scene/Game tabs, Inspector with component sections, Project and Console at the bottom). Keep the Phase 0 theme as an optional alternative, and keep the Logic canvas look (dotted background, animated dashed wires).

## Consequences

- Lower learning curve for Unity users.
- Token values are our approximation; no Unity assets or icons are copied.
