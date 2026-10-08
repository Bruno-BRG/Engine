<script lang="ts">
  import { logic } from '$lib/logic/store.svelte';
  import {
    boundsOf,
    fitMap,
    minimapToCamera,
    normRect,
    toWorld,
    type MapTransform,
    type Rect,
  } from '$lib/logic/geometry';

  /** Size of the map in pixels, and the space kept free around the graph inside it (VIS-014). */
  const W = 168;
  const H = 110;
  const MARGIN = 6;

  /** The part of the canvas on screen, in world units. */
  const view = $derived.by<Rect>(() => {
    const a = toWorld(logic.camera, 0, 0);
    const b = toWorld(logic.camera, logic.viewW, logic.viewH);
    return normRect(a[0], a[1], b[0], b[1]);
  });
  const nodeRects = $derived(
    logic.snap.nodes.map((n) => ({ id: n.id, rect: logic.rectOf(n), sel: logic.selNodes.includes(n.id) })),
  );
  const commentRects = $derived(
    logic.snap.comments.map((c) => ({
      id: c.id,
      rect: [c.position[0], c.position[1], c.size[0], c.size[1]] as Rect,
      sel: logic.selComments.includes(c.id),
    })),
  );
  /** The map covers the whole graph and the part on screen, so the view box is always visible on it. */
  const bounds = $derived(
    boundsOf([view, ...nodeRects.map((n) => n.rect), ...commentRects.map((c) => c.rect)]) ?? view,
  );
  const fit = $derived(fitMap(bounds, W, H, MARGIN));

  /** The fit taken when a drag begins: the map does not move during the drag, so it does not drift. Null when idle. */
  let dragFit = $state.raw<MapTransform | null>(null);
  /** The fit the map is drawn with: the drag's own while dragging, otherwise the live one. */
  const shownFit = $derived(dragFit ?? fit);

  /** A world rectangle as the map draws it. Items under 2 px stay visible. */
  function onMap(r: Rect) {
    return {
      x: r[0] * shownFit.scale + shownFit.x,
      y: r[1] * shownFit.scale + shownFit.y,
      w: Math.max(r[2] * shownFit.scale, 2),
      h: Math.max(r[3] * shownFit.scale, 2),
    };
  }
  /** The view box on the map. */
  const frame = $derived(onMap(view));

  let svg = $state<SVGSVGElement>();
  let dragging = $state(false);

  /** Puts the world point under the pointer at the centre of the canvas. The zoom does not change. */
  function centerAt(e: PointerEvent) {
    if (!svg) return;
    const r = svg.getBoundingClientRect();
    logic.camera = minimapToCamera(
      logic.camera,
      dragFit ?? fit,
      e.clientX - r.left,
      e.clientY - r.top,
      logic.viewW,
      logic.viewH,
    );
  }

  // The map sits over the canvas: its pointer, wheel and menu events stop here, so they do not pan or zoom the graph.
  function pointerDown(e: PointerEvent) {
    e.stopPropagation();
    if (!svg) return;
    // The view box moves with the camera, so the fit used for the whole drag is the one taken now.
    dragFit = fit;
    dragging = true;
    svg.setPointerCapture(e.pointerId);
    centerAt(e);
  }

  function pointerMove(e: PointerEvent) {
    e.stopPropagation();
    if (dragging) centerAt(e);
  }

  function pointerUp(e: PointerEvent) {
    e.stopPropagation();
    dragging = false;
    dragFit = null;
  }
</script>

<div class="map">
  <svg
    bind:this={svg}
    width={W}
    height={H}
    viewBox="0 0 {W} {H}"
    role="application"
    aria-label="Mapa do grafo: clique ou arraste para ir a um ponto"
    onpointerdown={pointerDown}
    onpointermove={pointerMove}
    onpointerup={pointerUp}
    onpointercancel={pointerUp}
    onwheel={(e) => e.stopPropagation()}
    oncontextmenu={(e) => e.stopPropagation()}
  >
    {#each nodeRects as n (`n${n.id}`)}
      {@const p = onMap(n.rect)}
      <rect class="node" class:sel={n.sel} x={p.x} y={p.y} width={p.w} height={p.h} />
    {/each}
    {#each commentRects as c (`c${c.id}`)}
      {@const p = onMap(c.rect)}
      <rect class="box" class:sel={c.sel} x={p.x} y={p.y} width={p.w} height={p.h} />
    {/each}
    <rect class="view" x={frame.x} y={frame.y} width={frame.w} height={frame.h} />
  </svg>
</div>

<style>
  .map {
    position: absolute;
    right: 8px;
    bottom: 8px;
    z-index: 2;
    overflow: hidden;
    line-height: 0;
    border-radius: var(--r-ctl);
    border: 1px solid var(--line);
    background: var(--bg-overlay);
    box-shadow: var(--shadow-float);
  }
  svg {
    display: block;
    cursor: crosshair;
  }
  .node {
    fill: var(--text-faint);
    opacity: 0.6;
  }
  .node.sel {
    fill: var(--accent);
    opacity: 1;
  }
  .box {
    fill: none;
    stroke: var(--text-faint);
    stroke-width: 1;
    stroke-dasharray: 2 2;
  }
  .box.sel {
    stroke: var(--accent);
  }
  .view {
    fill: var(--accent-soft);
    stroke: var(--accent);
    stroke-width: 1;
  }
</style>
