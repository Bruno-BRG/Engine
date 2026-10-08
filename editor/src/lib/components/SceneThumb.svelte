<script lang="ts">
  import type { LegacyScene } from '$lib/templates';
  let { data }: { data: LegacyScene } = $props();
  const [w, h] = $derived(data.screen);
</script>

<svg viewBox="0 0 {w} {h}" preserveAspectRatio="xMidYMid slice" class="thumb" aria-hidden="true">
  <defs>
    <pattern id="thumb-grid" width="64" height="64" patternUnits="userSpaceOnUse">
      <path d="M64 0H0V64" fill="none" stroke="white" stroke-opacity="0.05" stroke-width="2" />
    </pattern>
  </defs>
  <rect width={w} height={h} style="fill: var(--bg-0)" />
  <rect width={w} height={h} fill="url(#thumb-grid)" />
  {#each data.objects as o (o.id)}
    {#if o.visible}
      <g transform="translate({o.x} {o.y}) rotate({o.rotation})">
        {#if o.shape === 1}
          <ellipse rx={o.w / 2} ry={o.h / 2} fill={o.color} />
        {:else if o.shape === 2}
          <path d="M0 {-o.h / 2} L{o.w / 2} 0 L0 {o.h / 2} L{-o.w / 2} 0Z" fill={o.color} />
        {:else}
          <rect x={-o.w / 2} y={-o.h / 2} width={o.w} height={o.h} rx={Math.min(o.w, o.h) * 0.22} fill={o.color} />
        {/if}
      </g>
    {/if}
  {/each}
</svg>

<style>
  .thumb {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
