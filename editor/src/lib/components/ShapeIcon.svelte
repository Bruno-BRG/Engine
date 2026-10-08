<script lang="ts">
  // Aceita o nome da forma (Sprite2D) ou o índice da Fase 0 (templates).
  let { shape, color = 'currentColor', size = 14 }: { shape: string | number; color?: string; size?: number } = $props();
  const kind = $derived(shape === 1 || shape === 'circle' ? 1 : shape === 2 || shape === 'diamond' ? 2 : 0);
</script>

<svg width={size} height={size} viewBox="0 0 16 16" aria-hidden="true">
  {#if kind === 1}
    <circle cx="8" cy="8" r="6" fill={color} />
  {:else if kind === 2}
    <path d="M8 1.5 14.5 8 8 14.5 1.5 8Z" fill={color} />
  {:else}
    <rect x="2" y="2" width="12" height="12" rx="3" fill={color} />
  {/if}
</svg>
