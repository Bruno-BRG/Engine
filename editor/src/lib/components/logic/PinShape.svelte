<script lang="ts">
  import { pinStyle, type PinType } from '$lib/logic/model';

  /** Forma e cor de um pino pelo tipo (VIS-013). `ty` null = tipo ainda não resolvido. */
  let { ty, size = 10 }: { ty: PinType | null; size?: number } = $props();
  const estilo = $derived(pinStyle(ty));
</script>

<svg class="pin" width={size} height={size} viewBox="0 0 10 10" aria-hidden="true" style="color: {estilo.color}">
  {#if estilo.shape === 'tri'}
    <polygon points="1,0.5 9.5,5 1,9.5" style="fill: {estilo.color}" />
  {:else if estilo.shape === 'diamond'}
    <polygon points="5,0 10,5 5,10 0,5" style="fill: {estilo.color}" />
  {:else if estilo.shape === 'square'}
    <rect x="1" y="1" width="8" height="8" style="fill: {estilo.color}" />
  {:else if estilo.shape === 'hex'}
    <polygon points="2.5,0.5 7.5,0.5 10,5 7.5,9.5 2.5,9.5 0,5" style="fill: {estilo.color}" />
  {:else if estilo.shape === 'ring'}
    <circle cx="5" cy="5" r="3.5" style="fill: none; stroke: {estilo.color}" stroke-width="2" />
  {:else}
    <circle cx="5" cy="5" r="4" style="fill: {estilo.color}" />
  {/if}
</svg>

<style>
  .pin {
    flex: none;
    display: block;
  }
</style>
