// Registry properties for the logic property nodes (VIS-021): the type and name options the UI shows, and the default
// value of each property. The rules (the value pin type, validation) live in nexo-graph; here we only build the lists
// the user picks from.

import type { NodeType, PropDef } from '$lib/state/scene.svelte';
import type { Value } from './model';

/** Nodes that read or write a registry property. Their "type" and "name" pins are choices. */
export const PROPERTY_NODES = ['GetProperty', 'SetProperty'];

export const isPropertyNode = (kind: string): boolean => PROPERTY_NODES.includes(kind);

/** Properties of a type: its own and the inherited ones (the `base` chain), in registry order, without repeats. */
export function propertiesOfType(types: NodeType[], nodeType: string): PropDef[] {
  const out: PropDef[] = [];
  const seen = new Set<string>();
  let current = types.find((t) => t.name === nodeType);
  while (current) {
    for (const p of current.props) {
      if (!seen.has(p.name)) {
        seen.add(p.name);
        out.push(p);
      }
    }
    const parent: string | null = current.base;
    current = parent ? types.find((t) => t.name === parent) : undefined;
  }
  return out;
}

/** Property of a type by name, or undefined when the type does not have it. */
export function propertyOfType(types: NodeType[], nodeType: string, name: string): PropDef | undefined {
  return propertiesOfType(types, nodeType).find((p) => p.name === name);
}

/**
 * Default value of a property in the pin's format (the one the graph uses when the pin has no value).
 * Null when the property has no pin (a list of points).
 */
export function defaultValue(def: PropDef): Value | null {
  switch (def.kind) {
    case 'float':
      return { Float: Number(def.default) };
    case 'int':
    case 'mask':
      return { Int: Math.round(Number(def.default)) };
    case 'bool':
      return { Bool: def.default === true };
    case 'str':
    case 'enum':
    case 'color':
    case 'uid':
      return { String: typeof def.default === 'string' ? def.default : '' };
    case 'vec2': {
      const pair = Array.isArray(def.default) ? def.default : [];
      return { Vec2: [Number(pair[0] ?? 0), Number(pair[1] ?? 0)] };
    }
    default:
      return null;
  }
}
