# ADR 0009: Signal connections are stored on the emitting node

**Status:** accepted

## Context

SCN-034 connects a node's signals (the `timeout` of a Timer, the `body_entered` of an Area2D) to a script event on another node. Godot keeps these connections in the scene data of the emitter. The visual scripting runtime (VIS-002) will run them, but that dispatch is not part of this change. The scene file is at format 2, and `NodeFile` ignores unknown fields: an older build that loads a scene with connections would save it again without them, and nothing would warn the user.

## Decision

- A connection is `Connection { signal, target, method }`. It lives on the emitter, in `Node.connections`, outside `props`.
- Each node type declares the signals it emits in the registry (`registry::signals_of`). The core checks a connection against them when it is created and when a file is loaded.
- `target` is a node UID. `method` is the name of the handler on the target's script: an identifier of up to 64 ASCII characters. The core does not check that the script has it; the script graph lives outside the core.
- Removing a node removes every connection that points to it, in the same undo step. Play copies do the same when a node is destroyed at runtime.
- Duplicating or pasting a subtree remaps the targets that are inside the copy. Targets outside it keep their UID. Saving a branch as a scene drops the targets outside the branch.
- Changing a node's type drops the connections whose signal the new type does not emit.
- A target that does not exist when a file is loaded is kept, and the editor reports it.
- Scene file format 3 writes the `connections` field of a node only when it is not empty. Format 2 scenes load with no connections. The format number is bumped, so an older build refuses a format 3 scene instead of dropping the connections.
- The layout migration accepts format 2 or later, so a format 3 scene is not rewritten each time the project opens.

## Consequences

- Connections are saved, undone and reloaded, but nothing runs them yet. Until VIS-002, the method name has no effect at runtime.
- A project saved by this version cannot be opened by an older editor build. The refusal is explicit, so no data is lost silently.
- Duplicate and paste remap only connections. Other UID references, such as `node_b` in the joints, are still copied as they are. That is a known gap, tracked as SCN-036 in the backlog.
