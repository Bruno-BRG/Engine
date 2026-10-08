<script lang="ts">
  import { logic } from '$lib/logic/store.svelte';
  import { buscaNos, categoriasDe, familyOf, type PinFilter } from '$lib/logic/library';

  // Menu de busca de nós (VIS-011): categorias, busca difusa em PT e EN, e filtro pelo tipo do fio solto.
  let {
    x,
    y,
    filtro,
    onpick,
    onclose,
  }: {
    x: number;
    y: number;
    filtro: PinFilter | null;
    onpick: (kind: string) => void;
    onclose: () => void;
  } = $props();

  let q = $state('');
  let indice = $state(0);
  const grupos = $derived(categoriasDe(buscaNos(logic.defs, q, filtro)));
  const plano = $derived(grupos.flatMap((g) => g.itens));
  const ativo = $derived(Math.min(indice, Math.max(0, plano.length - 1)));

  function tecla(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onclose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      indice = Math.min(plano.length - 1, ativo + 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      indice = Math.max(0, ativo - 1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const escolhido = plano[ativo];
      if (escolhido) onpick(escolhido.def.kind);
    }
  }

  function foco(node: HTMLElement) {
    node.focus();
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="busca"
  style="left: {x}px; top: {y}px"
  role="dialog"
  tabindex="-1"
  aria-label="Buscar nó"
  onpointerdown={(e) => e.stopPropagation()}
  onwheel={(e) => e.stopPropagation()}
  oncontextmenu={(e) => e.preventDefault()}
>
  <input
    class="campo"
    bind:value={q}
    use:foco
    oninput={() => (indice = 0)}
    onkeydown={tecla}
    placeholder={filtro ? 'Buscar nó com pino compatível' : 'Buscar nó (somar, add, jump...)'}
    aria-label="Buscar nó"
    spellcheck="false"
  />
  <div class="lista" role="listbox" aria-label="Resultados">
    {#each grupos as g (g.nome)}
      <div class="cat">{g.nome}</div>
      {#each g.itens as h (h.def.kind)}
        <button
          class="item"
          class:ativo={plano[ativo]?.def.kind === h.def.kind}
          role="option"
          aria-selected={plano[ativo]?.def.kind === h.def.kind}
          title={h.def.description}
          onclick={() => onpick(h.def.kind)}
        >
          <span class="cor" style="background: {familyOf(h.def.category)}"></span>
          <span class="nome">{h.def.label}</span>
          <span class="tipo mono">{h.def.kind}</span>
        </button>
      {/each}
    {:else}
      <div class="vazio">Nenhum nó encontrado</div>
    {/each}
  </div>
</div>

<style>
  .busca {
    position: absolute;
    z-index: 5;
    width: 260px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 6px;
    border-radius: var(--r-dlg);
    background: var(--bg-overlay);
    border: 1px solid var(--line);
    box-shadow: var(--shadow-card);
    cursor: default;
  }
  .campo {
    height: 24px;
    padding: 0 8px;
    border-radius: var(--r-ctl);
    background: var(--bg-2);
    border: 1px solid var(--line);
    outline: none;
    font-size: var(--fs-12);
    user-select: text;
  }
  .campo:focus {
    border-color: var(--accent);
  }
  .lista {
    max-height: 300px;
    overflow: auto;
    display: flex;
    flex-direction: column;
  }
  .cat {
    padding: 6px 6px 2px;
    font-size: var(--fs-11);
    color: var(--text-faint);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 24px;
    padding: 0 6px;
    border-radius: var(--r-ctl);
    font-size: var(--fs-12);
    color: var(--text);
    text-align: left;
  }
  .item:hover,
  .item.ativo {
    background: var(--accent-soft);
  }
  .cor {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .nome {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tipo {
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .vazio {
    padding: 8px 6px;
    font-size: var(--fs-12);
    color: var(--text-faint);
  }
</style>
