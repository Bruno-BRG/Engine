// Live highlight of the debugger (VIS-030): the nodes and flow wires of the followed instance that ran in the last
// LIT_MS. It is plain TypeScript without runes, so `highlight.test.mjs` runs it in Node. The store in debug.svelte.ts
// keeps the result in reactive state.

import type { Trace, Value } from './model';

/** How long a node or a flow wire stays lit after it ran, in ms (VIS-030). */
export const LIT_MS = 300;

/** What is lit now: the followed instance, its nodes, and its flow wires (keys from `wireKey`). */
export interface Lit {
  uid: string | null;
  nodes: number[];
  wires: string[];
}

/** Key of a flow wire (origin node, output pin, destination node), the same in the trace and on the canvas. */
export function wireKey(from: number, pin: string, to: number): string {
  return `${from}|${pin}|${to}`;
}

/** Whether two lists hold the same items in the same order. */
export function sameList<T>(a: readonly T[], b: readonly T[]): boolean {
  return a.length === b.length && a.every((x, i) => x === b[i]);
}

/**
 * Merges the values of one read into `current`, the last values of an instance. Returns null when no value changed, so
 * the caller keeps the record it has.
 */
export function updateValues(current: Record<number, Value>, added: Trace['values']): Record<number, Value> | null {
  const next = { ...current };
  let changed = false;
  for (const { node, value } of added) {
    if (JSON.stringify(next[node]) !== JSON.stringify(value)) {
      next[node] = value;
      changed = true;
    }
  }
  return changed ? next : null;
}

/**
 * The highlight of the followed instance (VIS-030). Each node and wire remembers when it last ran, and goes dark once
 * LIT_MS have passed. `read` returns the same `Lit` object while nothing changed, so the store assigns it only when it
 * differs.
 */
export class LiveHighlight {
  #lit: Lit = { uid: null, nodes: [], wires: [] };
  /** Time (ms) of the last run of each lit node ("n<id>") and wire ("w<key>"). */
  #lastRun = new Map<string, number>();

  /**
   * Records one read of the trace of instance `uid` at time `now` (ms). `trace` is null when there was nothing to read,
   * and the highlight still ages.
   */
  read(uid: string, trace: Trace | null, now: number): Lit {
    if (this.#lit.uid !== uid) this.#lastRun.clear();
    if (trace) {
      for (const node of trace.nodes) this.#lastRun.set(`n${node}`, now);
      for (const wire of trace.wires) this.#lastRun.set(`w${wireKey(wire.from, wire.pin, wire.to)}`, now);
    }
    // Ages the highlight: what ran LIT_MS or more ago goes dark.
    for (const [key, when] of this.#lastRun) {
      if (now - when >= LIT_MS) this.#lastRun.delete(key);
    }
    const nodes: number[] = [];
    const wires: string[] = [];
    for (const key of this.#lastRun.keys()) {
      if (key.startsWith('n')) nodes.push(Number(key.slice(1)));
      else wires.push(key.slice(1));
    }
    if (this.#lit.uid !== uid || !sameList(this.#lit.nodes, nodes) || !sameList(this.#lit.wires, wires)) {
      this.#lit = { uid, nodes, wires };
    }
    return this.#lit;
  }

  /** Turns the highlight off. Returns what is lit now, which is nothing. */
  clear(): Lit {
    this.#lastRun.clear();
    if (this.#lit.uid !== null || this.#lit.nodes.length > 0 || this.#lit.wires.length > 0) {
      this.#lit = { uid: null, nodes: [], wires: [] };
    }
    return this.#lit;
  }
}
