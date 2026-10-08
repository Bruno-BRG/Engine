// Tests of the live highlight (VIS-030). Run from editor/ with:
//   node --test src/lib/logic/highlight.test.mjs
// Node 24 loads the .ts module directly (type stripping), so there is no build step and no dependency.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { LIT_MS, LiveHighlight, sameList, updateValues, wireKey } from './highlight.ts';

const trace = (nodes, wires, values = []) => ({ nodes, wires, values });
const wire = (from, pin, to) => ({ from, pin, to });

test('wireKey joins origin, pin and destination', () => {
  assert.equal(wireKey(3, 'true', 7), '3|true|7');
});

test('sameList compares the items in order', () => {
  assert.equal(sameList([], []), true);
  assert.equal(sameList([1, 2], [1, 2]), true);
  assert.equal(sameList([1, 2], [2, 1]), false);
  assert.equal(sameList([1], [1, 2]), false);
});

test('updateValues returns null when no value changed and leaves its input alone', () => {
  const current = { 2: { String: 'oi' } };
  assert.equal(updateValues(current, []), null);
  assert.equal(updateValues(current, [{ node: 2, value: { String: 'oi' } }]), null);
  assert.deepEqual(updateValues(current, [{ node: 2, value: { String: 'tchau' } }]), { 2: { String: 'tchau' } });
  assert.deepEqual(current, { 2: { String: 'oi' } });
});

test('a read with activity lights the nodes and the flow wires taken', () => {
  const highlight = new LiveHighlight();
  const lit = highlight.read('u1', trace([1, 2], [wire(1, 'exec', 2)]), 1000);
  assert.equal(lit.uid, 'u1');
  assert.deepEqual(lit.nodes, [1, 2]);
  assert.deepEqual(lit.wires, ['1|exec|2']);
});

test('a lit item stays lit until LIT_MS and then goes dark', () => {
  assert.equal(LIT_MS, 300);
  const highlight = new LiveHighlight();
  highlight.read('u1', trace([1], [wire(1, 'exec', 2)]), 1000);
  assert.deepEqual(highlight.read('u1', trace([], []), 1000 + LIT_MS - 1).wires, ['1|exec|2']);
  const dark = highlight.read('u1', null, 1000 + LIT_MS);
  assert.deepEqual(dark.nodes, []);
  assert.deepEqual(dark.wires, []);
});

test('a run seen again renews its time', () => {
  const highlight = new LiveHighlight();
  highlight.read('u1', trace([], [wire(1, 'exec', 2)]), 1000);
  highlight.read('u1', trace([], [wire(1, 'exec', 2)]), 1200);
  assert.deepEqual(highlight.read('u1', null, 1000 + LIT_MS).wires, ['1|exec|2']);
  assert.deepEqual(highlight.read('u1', null, 1200 + LIT_MS).wires, []);
});

test('only the wires in the trace light up, so an untaken branch stays dark', () => {
  const highlight = new LiveHighlight();
  const lit = highlight.read('u1', trace([5, 6], [wire(5, 'false', 6)]), 2000);
  assert.deepEqual(lit.wires, ['5|false|6']);
  assert.equal(lit.wires.includes('5|true|6'), false);
});

test('a read with nothing new returns the same object, so the store does not update', () => {
  const highlight = new LiveHighlight();
  const first = highlight.read('u1', trace([9], []), 2100);
  assert.equal(highlight.read('u1', trace([], []), 2150), first);
  assert.equal(highlight.read('u1', trace([9], []), 2150), first);
});

test('switching the followed instance drops the old highlight', () => {
  const highlight = new LiveHighlight();
  highlight.read('u1', trace([5], [wire(5, 'exec', 6)]), 2000);
  const lit = highlight.read('u2', trace([9], []), 2100);
  assert.equal(lit.uid, 'u2');
  assert.deepEqual(lit.nodes, [9]);
  assert.deepEqual(lit.wires, []);
});

test('clear turns the highlight off, and clearing again keeps the same object', () => {
  const highlight = new LiveHighlight();
  highlight.read('u1', trace([5], [wire(5, 'exec', 6)]), 2000);
  const cleared = highlight.clear();
  assert.deepEqual(cleared, { uid: null, nodes: [], wires: [] });
  assert.equal(highlight.clear(), cleared);
  assert.deepEqual(highlight.read('u1', null, 2100), { uid: 'u1', nodes: [], wires: [] });
});
