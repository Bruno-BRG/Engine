# ADR 0004: Everything is a node

**Status:** accepted

## Context

Users prefer Godot's model: every element is a node, and you shape behavior by choosing node types and editing properties. The editor should still look and feel like Unity.

## Decision

Scenes are trees of typed nodes with an inheritance chain (`Node → Node2D → CharacterBody2D`), stable UIDs, signals and scene instancing. The Unity-style "Add Component" button adds a typed child node. The runtime may store nodes in an ECS internally; users never see it.

## Consequences

- One concept to learn (nodes), consistent across physics, audio, UI and logic.
- Inspector, serializer, docs and visual scripting nodes are all generated from the same type metadata (SCN-002).
- Phase 0's flat object list and behaviors are replaced; existing projects are migrated (SCN-001).
