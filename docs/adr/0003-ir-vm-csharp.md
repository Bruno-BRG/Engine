# ADR 0003: Graphs compile to an IR; C# is generated output

**Status:** accepted

## Context

Visual logic needs instant hot reload and must run on desktop and web. The textual representation should be C#.

## Decision

Visual scripts compile to a typed IR executed by `nexo-vm` in Rust. A generator produces readable C# from the same IR. Hand-written C# arrives later, on the same API.

## Consequences

- Hot reload without an external compiler and without a .NET runtime in games.
- Simple web and mobile export.
- The engine API must be designed with C# in mind from day one (names, types, events).
