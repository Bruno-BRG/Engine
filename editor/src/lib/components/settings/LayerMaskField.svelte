<script lang="ts">
  // Grade de máscara de bits (PHY-020, PHY-022): uma caixa por camada nomeada, com o nome dado no projeto.
  // O número do bit não aparece, e a camada livre (sem nome) não entra na grade: nenhum corpo fica nela, então
  // o bit dela não muda nada. Se a camada ganhar nome depois, o bit volta com o valor guardado. O valor é o
  // inteiro que o registro guarda: -1 = todas.
  import { projectSettings } from '$lib/state/settings.svelte';
  import { hasBit, namedLayers, withBit } from '$lib/settings';

  let {
    label,
    value,
    onchange,
  }: {
    label: string;
    value: number;
    onchange: (mask: number) => void;
  } = $props();

  // Só as camadas nomeadas aparecem (ver o comentário acima).
  const visible = $derived(namedLayers(projectSettings));

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
    {#each visible as { index: i, name } (i)}
      <label class="cell" title={name}>
        <input type="checkbox" checked={hasBit(value, i)} onchange={(e) => set(i, e.currentTarget.checked)} />
        <span class="name">{name}</span>
      </label>
    {:else}
      <p class="vazio">Nenhuma camada com nome. Nomeie as camadas em Configurações do Projeto.</p>
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
  .vazio {
    grid-column: 1 / -1;
    margin: 0;
    font-size: var(--fs-11);
    color: var(--text-faint);
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
  .cell input {
    flex: none;
    margin: 0;
    accent-color: var(--accent);
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
