<script lang="ts">
  // Grade de máscara de bits (INS-003): uma caixa por camada, com o nome dado no projeto.
  // Camada livre (sem nome) fica desabilitada. O valor é o inteiro que o registro guarda: -1 = todas.
  import { projectSettings } from '$lib/state/settings.svelte';
  import { LAYER_COUNT, hasBit, withBit } from '$lib/settings';

  let {
    label,
    value,
    onchange,
  }: {
    label: string;
    value: number;
    onchange: (mask: number) => void;
  } = $props();

  const indices = Array.from({ length: LAYER_COUNT }, (_, i) => i);

  // withBit devolve sem sinal; o registro guarda com sinal (32 bits), então converte de volta.
  const set = (i: number, on: boolean) => onchange(withBit(value, i, on) | 0);
</script>

<div class="mask" role="group" aria-label={label}>
  <div class="head">
    <span class="flabel">{label}</span>
    <button class="link" onclick={() => onchange(-1)}>Todas</button>
    <button class="link" onclick={() => onchange(0)}>Nenhuma</button>
  </div>
  <div class="grid">
    {#each indices as i (i)}
      {@const name = projectSettings.layers[i] ?? ''}
      <label class="cell" class:free={!name} title={name || 'Camada livre (sem nome)'}>
        <input
          type="checkbox"
          disabled={!name}
          checked={hasBit(value, i)}
          onchange={(e) => set(i, e.currentTarget.checked)}
        />
        <span class="name">{name || i}</span>
      </label>
    {/each}
  </div>
</div>

<style>
  .mask {
    display: grid;
    gap: 4px;
    padding: 2px 0 2px 0;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .flabel {
    flex: 1;
    font-size: var(--fs-11);
    color: var(--text-dim);
  }
  .link {
    font-size: var(--fs-11);
    color: var(--accent-hi);
    padding: 0 2px;
  }
  .link:hover {
    text-decoration: underline;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 2px 4px;
  }
  .cell {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    height: 18px;
    font-size: var(--fs-11);
    color: var(--text);
  }
  .cell.free {
    color: var(--text-faint);
  }
  .cell input {
    flex: none;
    margin: 0;
    accent-color: var(--accent);
  }
  .cell input:disabled {
    opacity: 0.4;
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
