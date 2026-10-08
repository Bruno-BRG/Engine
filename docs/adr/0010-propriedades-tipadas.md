# ADR 0010: Typed property nodes

**Status:** accepted

## Context

GetProperty and SetProperty (VIS-021) read and write only decimals. The compiler does not see the scene, and the target of a property node is a dynamic Node pin. So the type of a property cannot come from the wire: the compiler has to know, when it builds the graph, which property is read. Godot and Unity solve this with the type metadata of each node type; the registry (`nexo_core::registry`) already has it.

## Decision

- A property node declares its target type and property name as embedded text: the `type` pin (a node type of the registry, for example "Sprite2D") and the `name` pin. Both are fixed on the node, as the variable name is in GetVariable. A wire into `type` is refused by `validate`. A wire into `name` is refused once a type is declared.
- Without a declared type, a graph from before typed properties keeps working: its `name` may still come from a wire, and its `value` pin is decimal.
- The `value` pin takes the type of the property in the registry, looked up through the inheritance chain. An empty `type` means decimal, as before this change, so graphs saved before it keep working.
- Properties of kind `Points` have no pin type and are refused.
- Without a wire or an embedded value, the `value` pin uses the default of the property in the registry. Without a declared type, it uses decimal zero.
- Changing `type` or `name` removes the embedded `value`, because it may be of another type. It also removes the wires of the `value` pin (its input, its output, and the wires downstream of it through reroutes) that no longer type-check. The engine returns those wires, and the editor warns the user.
- The IR ops `GetProp` and `SetProp` carry the type. The VM expects the host to return that type on a read. A decimal read from an integer property is accepted, because graphs without a declared type read integer properties as decimal. A read of any other type is a runtime error (`PropertyType`, which names the property and both types).
- A write the host refuses is a runtime error (`PropertyFailed` with `write: true`). It names the node and the property, not the types.
- The `Host` trait moves from `f64` to `Value` for properties. `set_property` returns `false` for a value the property does not take. Decimal and integer convert to each other, as the core does for numbers: an integer becomes decimal, and a decimal rounds into an integer property.
- Tween and Move To keep reading and writing decimals.
- The C# generator types the register by the operation and calls `Nexo.GetProperty<T>`.

## Consequences

- The node API changes: GetProperty and SetProperty gain a `type` input, and their `value` pin is typed by the registry. The graph file format does not change (`FORMAT` stays 1): the new values are ordinary embedded values.
- A declared type that does not match the real node in the scene fails only at runtime, as a `PropertyType` error when the read result has another type.
- Changing a type or a name can drop wires. Those wires are removed at once, not left as errors for `validate` to report, and the editor shows a warning with how many were dropped. The whole change is one undo step.
- Points, and Tween and Move To with non-decimal properties, remain out of scope.
