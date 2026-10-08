<script lang="ts">
  import { fly } from 'svelte/transition';
  import { flip } from 'svelte/animate';
  import { log } from '$lib/state/log.svelte';
  import { CircleCheck, CircleAlert, TriangleAlert, Info, X } from '@lucide/svelte';

  const icons = { success: CircleCheck, error: CircleAlert, warn: TriangleAlert, info: Info };
</script>

<div class="stack">
  {#each log.toasts as t (t.id)}
    {@const Icon = icons[t.level]}
    <div class="toast {t.level}" in:fly={{ y: 16, duration: 220 }} out:fly={{ x: 24, duration: 180 }} animate:flip={{ duration: 200 }}>
      <Icon size={16} />
      <div class="body">
        <strong>{t.title}</strong>
        {#if t.detail}<span>{t.detail}</span>{/if}
      </div>
      <button class="x" onclick={() => log.dismiss(t.id)} aria-label="Fechar"><X size={14} /></button>
    </div>
  {/each}
</div>

<style>
  .stack {
    position: fixed;
    right: 16px;
    bottom: 40px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 200;
    pointer-events: none;
  }
  .toast {
    pointer-events: auto;
    display: flex;
    align-items: flex-start;
    gap: 10px;
    min-width: 260px;
    max-width: 380px;
    padding: 11px 10px 11px 12px;
    border-radius: var(--r-dlg);
    background: var(--bg-2);
    box-shadow: var(--shadow-pop);
  }
  .toast :global(svg) {
    flex: none;
    margin-top: 1px;
  }
  .success > :global(svg:first-child) {
    color: var(--success);
  }
  .error > :global(svg:first-child) {
    color: var(--danger);
  }
  .warn > :global(svg:first-child) {
    color: var(--warning);
  }
  .info > :global(svg:first-child) {
    color: var(--info);
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }
  .body strong {
    font-weight: 600;
  }
  .body span {
    color: var(--text-dim);
    font-size: var(--fs-12);
    overflow-wrap: anywhere;
  }
  .x {
    color: var(--text-faint);
    border-radius: 4px;
    padding: 2px;
  }
  .x:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
</style>
