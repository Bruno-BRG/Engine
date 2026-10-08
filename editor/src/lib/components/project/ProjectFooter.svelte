<script lang="ts">
  import { List, LayoutGrid } from '@lucide/svelte';

  // Rodapé: contagem à esquerda e o controle de tamanho à direita. No mínimo, a grade vira lista.
  let {
    label = '',
    size = $bindable(50),
  }: {
    label?: string;
    size?: number;
  } = $props();
</script>

<footer class="foot">
  <span>{label}</span>
  <span class="grow"></span>
  <List size={12} />
  <input
    type="range"
    min="0"
    max="100"
    bind:value={size}
    aria-label="Tamanho dos itens. No mínimo, lista."
    title="Tamanho dos itens (no mínimo, lista)"
  />
  <LayoutGrid size={12} />
</footer>

<style>
  .foot {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    padding: 0 8px;
    font-size: var(--fs-11);
    color: var(--text-faint);
    background: var(--bg-toolbar);
    border-top: 1px solid var(--line);
  }
  .foot :global(svg) {
    flex: none;
  }
  .grow {
    flex: 1;
  }
  .foot input[type='range'] {
    width: 96px;
    margin: 0;
    accent-color: var(--accent);
  }
</style>
