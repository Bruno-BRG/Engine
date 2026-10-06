<script lang="ts">
  import type { ShapeKind } from '$lib/state/scene.svelte';
  let { shape, color = 'currentColor', size = 14 }: { shape: ShapeKind; color?: string; size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 16 16" aria-hidden="true">
  {#if shape === 1}
    <circle cx="8" cy="8" r="6" fill={color} />
  {:else if shape === 2}
    <path d="M8 1.5 14.5 8 8 14.5 1.5 8Z" fill={color} />
  {:else}
    <rect x="2" y="2" width="12" height="12" rx="3" fill={color} />
  {/if}
</svg>
