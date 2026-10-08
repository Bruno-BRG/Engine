// Tests of the canvas geometry added for VIS-014: the flow layout, the minimap transform and the semantic zoom
// threshold. Run from editor/ with:
//   node --test src/lib/logic/geometry.test.mjs
// Node 24 loads the .ts module directly (type stripping), so there is no build step and no dependency.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  DETAIL_MIN_ZOOM,
  LAYOUT_GAP_X,
  LAYOUT_GAP_Y,
  boundsOf,
  centerCamera,
  fitMap,
  flowLayout,
  minimapToCamera,
  showsDetail,
  toWorld,
} from './geometry.ts';

const node = (id, x, y, w = 200, h = 80) => ({ id, size: [w, h], position: [x, y] });
const wire = (from, to) => ({ from, to });
/** Maps a world point through a transform, as the minimap does. */
const onMap = (t, x, y) => [x * t.scale + t.x, y * t.scale + t.y];

test('a chain takes one column per node, from left to right', () => {
  const out = flowLayout(
    [node(1, 0, 0), node(2, 0, 0), node(3, 0, 0)],
    [wire(1, 2), wire(2, 3)],
  );
  const column = 200 + LAYOUT_GAP_X;
  assert.deepEqual(out.get(1), [0, 0]);
  assert.deepEqual(out.get(2), [column, 0]);
  assert.deepEqual(out.get(3), [2 * column, 0]);
});

test('the layout keeps the top-left corner of the nodes it moves', () => {
  const out = flowLayout([node(1, 500, 300), node(2, 700, 320)], [wire(1, 2)]);
  assert.deepEqual(out.get(1), [500, 300]);
  assert.deepEqual(out.get(2), [500 + 200 + LAYOUT_GAP_X, 300]);
});

test('a node sits in the column of its deepest input, not the first one found', () => {
  // s -> m -> t and s -> t: t must come after m, so it is two columns right of s.
  const out = flowLayout(
    [node(1, 0, 0), node(2, 0, 100), node(3, 0, 200)],
    [wire(1, 2), wire(2, 3), wire(1, 3)],
  );
  const column = 200 + LAYOUT_GAP_X;
  assert.equal(out.get(1)[0], 0);
  assert.equal(out.get(2)[0], column);
  assert.equal(out.get(3)[0], 2 * column);
});

test('a data node feeding a flow node sits to the left of it', () => {
  // Event -> Print (flow), and a data node -> Print. Both inputs of Print are in the column before it.
  const out = flowLayout(
    [node(1, 0, 0), node(2, 0, 200), node(3, 300, 100)],
    [wire(1, 3), wire(2, 3)],
  );
  assert.ok(out.get(1)[0] < out.get(3)[0]);
  assert.ok(out.get(2)[0] < out.get(3)[0]);
  assert.equal(out.get(1)[0], out.get(2)[0]);
});

test('nodes in one column keep their top-to-bottom order and never overlap', () => {
  // Two roots in column 0. The one higher up stays higher; the second starts a node height and a gap below it.
  const out = flowLayout([node(1, 0, 500, 200, 90), node(2, 0, 100, 200, 60)], []);
  assert.deepEqual(out.get(2), [0, 100]);
  assert.deepEqual(out.get(1), [0, 100 + 60 + LAYOUT_GAP_Y]);
});

test('a loop does not hang the layout, and only the wire that closes it is dropped', () => {
  // A While: 1 -> 2 is the body, 2 -> 1 goes back to the loop head.
  const out = flowLayout([node(1, 0, 0), node(2, 0, 50)], [wire(1, 2), wire(2, 1)]);
  assert.equal(out.size, 2);
  assert.equal(out.get(1)[0], 0);
  assert.equal(out.get(2)[0], 200 + LAYOUT_GAP_X);
});

test('a self wire is ignored, and a wire to a node outside the set does not move anything', () => {
  const out = flowLayout([node(1, 0, 0)], [wire(1, 1), wire(1, 99)]);
  assert.deepEqual([...out.entries()], [[1, [0, 0]]]);
});

test('an empty graph gives an empty layout', () => {
  assert.equal(flowLayout([], []).size, 0);
});

test('the minimap fits a rectangle in the map with the margin on every side', () => {
  const t = fitMap([0, 0, 100, 50], 200, 100, 10);
  assert.deepEqual(onMap(t, 0, 0), [20, 10]);
  assert.deepEqual(onMap(t, 100, 50), [180, 90]);
});

test('the minimap fit is centred when the rectangle is not the map shape', () => {
  const t = fitMap([0, 0, 100, 100], 200, 100);
  // The limit is the height: 100 px of map for 100 units, so the rectangle is 100 px wide, centred in 200.
  assert.deepEqual(onMap(t, 0, 0), [50, 0]);
  assert.deepEqual(onMap(t, 100, 100), [150, 100]);
});

test('a degenerate rectangle still gets a finite, positive scale', () => {
  const t = fitMap([5, 5, 0, 0], 160, 100);
  assert.ok(Number.isFinite(t.scale) && t.scale > 0);
  assert.ok(Number.isFinite(t.x) && Number.isFinite(t.y));
});

test('a click on the minimap centres that world point and keeps the zoom', () => {
  const cam = { x: 10, y: -20, zoom: 0.5 };
  const next = centerCamera(cam, 300, 120, 800, 500);
  assert.equal(next.zoom, 0.5);
  assert.deepEqual(toWorld(next, 400, 250), [300, 120]);
});

test('detail shows from the threshold up, and not below it', () => {
  assert.equal(DETAIL_MIN_ZOOM, 0.6);
  assert.equal(showsDetail(DETAIL_MIN_ZOOM), true);
  assert.equal(showsDetail(DETAIL_MIN_ZOOM - 0.01), false);
  assert.equal(showsDetail(0.25), false);
  assert.equal(showsDetail(1), true);
});

test('a minimap drag does not drift: with the fit fixed, the camera lands on the pointer at every event', () => {
  // The fit is the one taken when the drag began. The pointer goes six pixels right, then six back to the start.
  const fit = fitMap([-300, -200, 900, 600], 168, 110, 6);
  const start = { x: 120, y: 90, zoom: 1 };
  const xs = [85, 86, 87, 88, 89, 90, 89, 88, 87, 86, 85, 84];
  let cam = start;
  for (const x of xs) {
    cam = minimapToCamera(cam, fit, x, 55, 800, 500);
    // The world point under the pointer, measured with that same fit, is the centre of the canvas.
    const [cx, cy] = toWorld(cam, 400, 250);
    assert.ok(Math.abs(cx - (x - fit.x) / fit.scale) < 1e-9);
    assert.ok(Math.abs(cy - (55 - fit.y) / fit.scale) < 1e-9);
  }
  // Back at the first pointer position, the camera is the one that position gives from any start.
  assert.deepEqual(cam, minimapToCamera(start, fit, 84, 55, 800, 500));
});

test('the camera a minimap pointer gives depends only on the pointer and the zoom, not on the old camera', () => {
  const fit = fitMap([0, 0, 1000, 800], 168, 110, 6);
  const a = minimapToCamera({ x: 0, y: 0, zoom: 0.5 }, fit, 100, 60, 800, 500);
  const b = minimapToCamera({ x: -4000, y: 900, zoom: 0.5 }, fit, 100, 60, 800, 500);
  assert.deepEqual(a, b);
});

test('a click at the centre of the map centres the camera on the centre of the graph', () => {
  const graph = [200, -100, 600, 400];
  const fit = fitMap(graph, 168, 110, 6);
  // The centre of the map is (84, 55): fitMap puts the centre of the graph there.
  const cam = minimapToCamera({ x: 0, y: 0, zoom: 1 }, fit, 84, 55, 800, 500);
  const [cx, cy] = toWorld(cam, 400, 250);
  assert.ok(Math.abs(cx - 500) < 1e-9);
  assert.ok(Math.abs(cy - 100) < 1e-9);
});

test('an empty graph gives no bounds, and the view alone still gives a usable minimap', () => {
  assert.equal(boundsOf([]), null);
  // Without a graph the minimap fits the view rectangle only (the fallback in Minimap.svelte).
  const view = [-50, -20, 800, 500];
  const fit = fitMap(boundsOf([view]) ?? view, 168, 110, 6);
  const cam = minimapToCamera({ x: 0, y: 0, zoom: 1 }, fit, 84, 55, 800, 500);
  assert.ok([cam.x, cam.y, cam.zoom].every(Number.isFinite));
});
