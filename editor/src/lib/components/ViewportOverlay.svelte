<script lang="ts">
  // Gizmos do viewport (VPT-001, VPT-005) desenhados em SVG por cima do canvas.
  // As coordenadas chegam já em pixels de tela; o componente não conhece o mundo.
  import type { Tool } from '$lib/state/app.svelte';

  type Pt = { x: number; y: number };
  export type Gizmo = { pivot: Pt; corners: Pt[]; rotR: number } | null;
  export type Box = { x0: number; y0: number; x1: number; y1: number } | null;

  let {
    tool,
    gizmo,
    frames = [],
    box = null,
    frame = null,
    colliders = [],
  }: {
    tool: Tool;
    gizmo: Gizmo;
    frames?: Pt[][];
    box?: Box;
    frame?: Pt[] | null;
    /** Contornos das formas de colisão, já em pixels de tela (VPT-005). */
    colliders?: Pt[][];
  } = $props();

  const pts = (p: Pt[]) => p.map((q) => `${q.x.toFixed(1)},${q.y.toFixed(1)}`).join(' ');
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
