# ADR 0012: Exported script values are stored on each node

**Status:** accepted

## Context

VIS-022 declares script variables, and some of them are exported to the Inspector (INS-005). Each node that uses a script needs its own value for each exported variable, as Unity and Godot do with exported fields. The script graph lives in `scripts/*.logic.ron`, outside the core, so the core cannot check a node's values against the variables.

The scene file stores a node's non-default properties in `props`. `put_raw` refuses a key that the registry does not declare, so a build that does not know a new key fails to open the scene.

A UID is a 64-bit number. The editor reads JSON numbers as JavaScript doubles, which lose precision above 2^53, so a UID cannot travel through JavaScript as a number.

## Decision

- A new property `script_vars` of kind `Map` is declared on the `Node` base type, with default `{}`. The core checks only that the value is a JSON object.
- The key is the variable name. The value is the graph's `Value` format, for example `{"Float": 2.5}`.
- The map stores only the values that differ from the script's default. A value equal to the default is removed from the map. The property itself is removed when the map is empty, as any other property is.
- A node reference (`Node` type) is written as text with up to 16 hexadecimal digits, for example `{"Node": "00000000000000ff"}`. The empty text is no node, the same as `Node(0)`. The graph snapshot sends a node-reference default the same way, as `default_node`, because a JSON number would lose the low digits above 2^53.
- The values are applied when a play instance is created (`Compiled::new_context_with` in `nexo-graph`). Only an exported variable with the same name and type takes a value. An entry that does not match (the variable was removed, is no longer exported, or changed type after the script was edited) is ignored, and the script's default stays. A malformed value stops the play and the message names the script and the variable.
- Changing the script attached to a node clears that node's values, as one undo step.
- The scene file stays at format 3. A node without values writes nothing, so existing scenes are unchanged. A scene that has values cannot be opened by an older build: that build refuses it with `Propriedade desconhecida: script_vars`, which is the explicit refusal `put_raw` already gives for an unknown key. The values are not dropped silently.

## Consequences

- Editing a value is undoable, and the value is copied with the node: duplicate, paste and save branch as scene. It is a normal property, so the existing history covers it.
- Entries for variables the script no longer exports stay in the file. The runtime ignores them. They are removed only when a different script is attached to the node. Attaching the same script again does nothing, so the values stay. The "Restore values" action on a section does not touch `script` or `script_vars`, because the editor skips both.
- The values are read when the play starts. Hot reload (VIS-023) keeps the values the running instances already have, for variables with the same name and type. The C# export (VIS-024) does not use the per-node values yet.
- Older builds cannot open a scene that has values. They refuse it with an error, so no value is lost without warning.
