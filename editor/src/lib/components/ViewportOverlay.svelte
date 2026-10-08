<script lang="ts">
  // Gizmos do viewport (VPT-001, VPT-005) desenhados em SVG por cima do canvas.
  // As coordenadas chegam já em pixels de tela; o componente não conhece o mundo.
  import type { Tool } from '$lib/state/app.svelte';

  type Pt = { x: number; y: number };
  export type Gizmo = { pivot: Pt; corners: Pt[]; rotR: number } | null;
  export type Box = { x0: number; y0: number; x1: number; y1: number } | null;
  /** Forma de colisão selecionada: contorno e alças, em pixels de tela (PHY-012). */
  export type ShapeGizmo = { pts: Pt[]; handles: Pt[] } | null;
  /** Raio de atenuação de um AudioStreamPlayer2D, em pixels de tela (VPT-005). */
  export type AudioRadius = { c: Pt; r: number; sel: boolean };
  /** Desenho de depuração da física, em pixels de tela (DBG-004). */
  export type DebugOverlay = {
    shapes: { role: string; pts: Pt[]; closed: boolean }[];
    contacts: { p: Pt; q: Pt }[];
    velocities: { from: Pt; to: Pt }[];
  } | null;

  let {
    tool,
    gizmo,
    frames = [],
    box = null,
    frame = null,
    colliders = [],
    shape = null,
    audio = [],
    debug = null,
  }: {
    tool: Tool;
    gizmo: Gizmo;
    frames?: Pt[][];
    box?: Box;
    frame?: Pt[] | null;
    /** Contornos das formas de colisão, já em pixels de tela (VPT-005). */
    colliders?: Pt[][];
    shape?: ShapeGizmo;
    audio?: AudioRadius[];
    debug?: DebugOverlay;
  } = $props();

  const pts = (p: Pt[]) => p.map((q) => `${q.x.toFixed(1)},${q.y.toFixed(1)}`).join(' ');

  /** Ponta de seta no fim de `to`, na direção de `from` (DBG-004). */
  function arrowHead(from: Pt, to: Pt): string {
    const dx = to.x - from.x;
    const dy = to.y - from.y;
    const len = Math.hypot(dx, dy) || 1;
    const ux = dx / len;
    const uy = dy / len;
    const bx = to.x - ux * 7;
    const by = to.y - uy * 7;
    return pts([to, { x: bx - uy * 3.5, y: by + ux * 3.5 }, { x: bx + uy * 3.5, y: by - ux * 3.5 }]);
  }
  const boxRect = $derived(
    box
      ? {
          x: Math.min(box.x0, box.x1),
          y: Math.min(box.y0, box.y1),
          w: Math.abs(box.x1 - box.x0),
          h: Math.abs(box.y1 - box.y0),
        }
      : null,
  );
</script>

<svg class="overlay" aria-hidden="true">
  {#if frame}
    <polygon points={pts(frame)} class="game-frame" />
  {/if}

  {#each frames as f}
    <polygon points={pts(f)} class="camera-frame" />
  {/each}

  {#each colliders as c}
    <polygon points={pts(c)} class="collider" />
  {/each}

  {#each audio as a}
    <circle cx={a.c.x} cy={a.c.y} r={a.r} class="audio-radius" class:sel={a.sel} />
  {/each}

  {#if debug}
    {#each debug.shapes as s}
      {#if s.closed}
        <polygon points={pts(s.pts)} class="dbg-shape dbg-{s.role}" />
      {:else}
        <polyline points={pts(s.pts)} class="dbg-shape dbg-{s.role}" />
      {/if}
    {/each}
    {#each debug.contacts as k}
      <line x1={k.p.x} y1={k.p.y} x2={k.q.x} y2={k.q.y} class="dbg-normal" />
      <circle cx={k.p.x} cy={k.p.y} r="2.5" class="dbg-contact" />
    {/each}
    {#each debug.velocities as v}
      <line x1={v.from.x} y1={v.from.y} x2={v.to.x} y2={v.to.y} class="dbg-vel" />
      <polygon points={arrowHead(v.from, v.to)} class="dbg-vel-head" />
    {/each}
  {/if}

  {#if shape}
    {#if shape.pts.length > 1}
      <polygon points={pts(shape.pts)} class="shape-outline" />
    {/if}
    {#each shape.handles as h}
      <rect x={h.x - 4} y={h.y - 4} width="8" height="8" class="handle" />
    {/each}
  {/if}

  {#if gizmo}
    <polygon points={pts(gizmo.corners)} class="outline" />

    {#if tool === 'move'}
      <g class="move" transform="translate({gizmo.pivot.x} {gizmo.pivot.y})">
        <line x1="0" y1="0" x2="56" y2="0" class="axis-x" />
        <polygon points="56,-4 64,0 56,4" class="axis-x-head" />
        <line x1="0" y1="0" x2="0" y2="56" class="axis-y" />
        <polygon points="-4,56 0,64 4,56" class="axis-y-head" />
        <rect x="-4" y="-4" width="8" height="8" class="pivot" />
      </g>
    {:else if tool === 'rotate'}
      <circle cx={gizmo.pivot.x} cy={gizmo.pivot.y} r={gizmo.rotR} class="ring" />
      <circle cx={gizmo.pivot.x} cy={gizmo.pivot.y} r="3" class="pivot-dot" />
    {:else if tool === 'scale' || tool === 'rect'}
      {#each gizmo.corners as c}
        <rect x={c.x - 4} y={c.y - 4} width="8" height="8" class="handle" />
      {/each}
    {/if}
  {/if}

  {#if boxRect}
    <rect x={boxRect.x} y={boxRect.y} width={boxRect.w} height={boxRect.h} class="box" />
  {/if}
</svg>

<style>
  .overlay {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    overflow: visible;
  }
  .outline {
    fill: none;
    stroke: var(--accent-hi);
    stroke-width: 1.5;
  }
  .game-frame {
    fill: none;
    stroke: var(--line-strong);
    stroke-width: 1;
  }
  .camera-frame {
    fill: none;
    stroke: var(--text-faint);
    stroke-width: 1;
    stroke-dasharray: 5 4;
  }

  /* Colisores em verde, como o Unity: são formas de física, não desenho. */
  .collider {
    fill: rgba(107, 191, 89, 0.08);
    stroke: var(--node-ui);
    stroke-width: 1;
  }
  /* Forma selecionada: contorno em destaque e alças para arrastar (PHY-012). */
  .shape-outline {
    fill: none;
    stroke: var(--accent-hi);
    stroke-width: 2;
  }
  /* Raio de atenuação do som 2D (VPT-005): tracejado e discreto; mais forte quando selecionado. */
  .audio-radius {
    fill: color-mix(in srgb, var(--node-audio) 6%, transparent);
    stroke: var(--node-audio);
    stroke-width: 1;
    stroke-dasharray: 6 4;
    opacity: 0.7;
  }
  .audio-radius.sel {
    opacity: 1;
    stroke-width: 1.5;
  }
  /* Depuração da física (DBG-004): cor por papel do corpo, como no Box2D e no Unity. */
  .dbg-shape {
    fill: none;
    stroke-width: 1.5;
  }
  .dbg-static {
    stroke: var(--text-dim);
  }
  .dbg-dynamic {
    stroke: var(--info);
  }
  .dbg-kinematic {
    stroke: var(--node-physics);
  }
  .dbg-character {
    stroke: var(--success);
  }
  .dbg-area {
    stroke: var(--accent-hi);
    stroke-dasharray: 4 3;
  }
  .dbg-normal {
    stroke: var(--danger);
    stroke-width: 1;
  }
  .dbg-contact {
    fill: var(--danger);
  }
  .dbg-vel {
    stroke: var(--warning);
    stroke-width: 1.5;
  }
  .dbg-vel-head {
    fill: var(--warning);
  }
  .axis-x,
  .axis-x-head {
    stroke: var(--danger);
    fill: var(--danger);
    stroke-width: 2;
  }
  .axis-y,
  .axis-y-head {
    stroke: var(--success);
    fill: var(--success);
    stroke-width: 2;
  }
  .pivot {
    fill: var(--text);
    stroke: var(--bg-0);
    stroke-width: 1;
  }
  .ring {
    fill: none;
    stroke: var(--accent-hi);
    stroke-width: 1.5;
    stroke-dasharray: 3 3;
  }
  .pivot-dot {
    fill: var(--accent-hi);
  }
  .handle {
    fill: var(--text);
    stroke: var(--accent);
    stroke-width: 1.5;
  }
  .box {
    fill: var(--accent-soft);
    stroke: var(--accent-hi);
    stroke-width: 1;
  }
</style>
