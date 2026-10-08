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
  normRect,
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

// The minimap drag as Minimap.svelte runs it. The map is fitted to the bounds of the graph and of the view box.
// The view box moves with the camera, so when the graph is smaller than the view, a fit recomputed on every
// event moves too.
const MAP = { w: 168, h: 110, margin: 6 };
const CANVAS = { w: 800, h: 500 };
const GRAPH = [0, 0, 400, 300];
const DRAG_Y = 55;
/**
 * Pointer positions of the drag: pointerdown at x = 84, then 12 px to the right and 12 px back, one event per pixel.
 */
const DRAG_XS = [84, ...Array.from({ length: 12 }, (_, i) => 85 + i), ...Array.from({ length: 12 }, (_, i) => 95 - i)];
const viewOf = (cam) => {
  const a = toWorld(cam, 0, 0);
  const b = toWorld(cam, CANVAS.w, CANVAS.h);
  return normRect(a[0], a[1], b[0], b[1]);
};
/** The fit Minimap.svelte computes from a camera: the bounds of the view box and of the graph. */
const fitFor = (cam) => fitMap(boundsOf([viewOf(cam), GRAPH]), MAP.w, MAP.h, MAP.margin);

/**
 * Runs the drag with one event per entry of `xs`, at height `y`, and returns the world point at the centre of the
 * canvas after each event. `pinned` true keeps the fit taken at pointerdown for the whole drag (the code now). `pinned`
 * false takes the fit again on every event, from the camera of that moment (the code before 81a2b57).
 */
function dragCentres(pinned, start, xs, y) {
  const pinnedFit = fitFor(start);
  let cam = start;
  return xs.map((x) => {
    cam = minimapToCamera(cam, pinned ? pinnedFit : fitFor(cam), x, y, CANVAS.w, CANVAS.h);
    return toWorld(cam, CANVAS.w / 2, CANVAS.h / 2);
  });
}

test('a minimap drag keeps the fit taken at pointerdown, so the camera does not drift', () => {
  // A pixel of pointer travel moves the world point under the pointer by 1 / scale. So after each event, the centre of
  // the canvas is the pointerdown point plus the sum of the pointer's displacements so far, divided by the scale.
  const start = { x: 0, y: 0, zoom: 0.5 };
  const fit = fitFor(start);
  const centres = dragCentres(true, start, DRAG_XS, DRAG_Y);
  const origin = (DRAG_XS[0] - fit.x) / fit.scale;
  centres.forEach((c, k) => {
    const expected = origin + (DRAG_XS[k] - DRAG_XS[0]) / fit.scale;
    assert.ok(Math.abs(c[0] - expected) < 1e-9, `event ${k}: centre at ${c[0]}, expected ${expected}`);
    assert.ok(Math.abs(c[1] - (DRAG_Y - fit.y) / fit.scale) < 1e-9, `event ${k}: centre y`);
  });
});

test('the same drag with the fit recomputed on every event drifts, which is what the pinned fit prevents', () => {
  const start = { x: 0, y: 0, zoom: 0.5 };
  const fit = fitFor(start);
  const centres = dragCentres(false, start, DRAG_XS, DRAG_Y);
  // The pointer is back at the pointerdown position, but the centre is more than one world unit away from its point.
  const drift = centres[centres.length - 1][0] - (DRAG_XS[0] - fit.x) / fit.scale;
  assert.ok(Math.abs(drift) > 1, `the recomputed fit should drift, got ${drift}`);
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
