// Exported script values, per node (VIS-022). A node keeps only the differences from the script's default, in
// props.script_vars: variable name -> value in the graph's format. A node UID travels as hexadecimal text
// ({Node: "00ab..."}), because a 64-bit number loses precision in JavaScript. Empty text is no node (Node(0)).
// Rust checks the map when a play starts (nexo-graph, ir.rs). See docs/adr/0012.
import type { PinType, Variable } from './model';

/**
 * Value of an exported variable, as the node stores it: the graph's `Value`, except that Node is hexadecimal text.
 */
export type StoredValue =
  | { Bool: boolean }
  | { Int: number }
  | { Float: number }
  | { String: string }
  | { Vec2: [number, number] }
  | { Node: string };

/** The `props.script_vars` map of a node: only the variables whose value differs from the default. */
export type ScriptVars = Record<string, StoredValue>;

/** The value map of a node. A value that is not an object counts as empty. */
export function scriptVarsOf(props: Record<string, unknown>): ScriptVars {
  const raw = props.script_vars;
  return raw && typeof raw === 'object' && !Array.isArray(raw) ? (raw as ScriptVars) : {};
}

/**
 * Default value of the variable in the script, in the stored format. A node default is read from `default_node`
 * (hex text), not from the number in `default`, which loses digits above 2^53.
 */
export function defaultOf(variable: Variable): StoredValue {
  const value = variable.default;
  if ('Node' in value) return { Node: variable.default_node ?? '' };
  return value;
}

/** Type of a stored value, from its tag: the same name as the pin type. */
export function typeOf(value: StoredValue): PinType {
  return Object.keys(value)[0] as PinType;
}

/** Equality of stored values: same type and same content. */
export function sameValue(a: StoredValue, b: StoredValue): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/**
 * The value the interface shows for the variable: the one stored on the node, if it has the variable's current type;
 * otherwise the script's default. A value of another type (the script changed later) is ignored, as in Rust.
 */
export function valueOf(variable: Variable, vars: ScriptVars): StoredValue {
  const stored = vars[variable.name];
  return stored && typeOf(stored) === variable.ty ? stored : defaultOf(variable);
}

/**
 * True when this node overrides the variable: it holds a value of the variable's type that differs from the default.
 */
export function isOverridden(variable: Variable, vars: ScriptVars): boolean {
  const stored = vars[variable.name];
  return !!stored && typeOf(stored) === variable.ty && !sameValue(stored, defaultOf(variable));
}

/**
 * Map with the variable set to `value`. A value equal to the script's default is not stored: the key leaves the map,
 * and the node follows the script again.
 */
export function withValue(vars: ScriptVars, variable: Variable, value: StoredValue): ScriptVars {
  const next: ScriptVars = { ...vars };
  if (sameValue(value, defaultOf(variable))) delete next[variable.name];
  else next[variable.name] = value;
  return next;
}

// Reading the value for each field. Only called with a value already of the variable's type (see `valueOf`).

export function asNumber(value: StoredValue): number {
  if ('Float' in value) return value.Float;
  if ('Int' in value) return value.Int;
  return 0;
}

export function asBool(value: StoredValue): boolean {
  return 'Bool' in value && value.Bool;
}

export function asText(value: StoredValue): string {
  return 'String' in value ? value.String : '';
}

export function asVec2(value: StoredValue): [number, number] {
  return 'Vec2' in value ? value.Vec2 : [0, 0];
}

export function asNode(value: StoredValue): string {
  return 'Node' in value ? value.Node : '';
}
