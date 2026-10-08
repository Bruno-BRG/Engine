<script lang="ts">
  import { SHAPES_DIR } from '$lib/state/assets.svelte';
  import { dragState } from './drag.svelte';
  import { Plus, Upload, ArrowUp, Search, ChevronRight } from '@lucide/svelte';

  let {
    crumbs,
    canUp,
    writable,
    search = $bindable(''),
    oncreate,
    onimport,
    onup,
    onopen,
  }: {
    /** Caminho da pasta aberta: raiz (nome do projeto) e as pastas até a aberta. */
    crumbs: { label: string; rel: string }[];
    canUp: boolean;
    writable: boolean;
    search?: string;
    oncreate: (btn: HTMLElement) => void;
    onimport: () => void;
    onup: () => void;
    onopen: (rel: string) => void;
  } = $props();
</script>

<header class="bar">
  <button class="btn" onclick={(e) => oncreate(e.currentTarget)} title="Criar pasta, cena ou outro item">
    <Plus size={13} /> Criar
  </button>
  <button class="btn" onclick={onimport} disabled={!writable} title="Importar arquivos para a pasta aberta">
    <Upload size={13} /> Importar
  </button>
  <button class="icon" onclick={onup} disabled={!canUp} title="Pasta acima" aria-label="Pasta acima">
    <ArrowUp size={13} />
  </button>
  <nav class="crumbs" aria-label="Caminho da pasta">
    {#each crumbs as c, i (c.rel + '#' + i)}
      {#if i > 0}<ChevronRight size={11} />{/if}
      <button
        class="crumb"
        class:last={i === crumbs.length - 1}
        class:drop={dragState.over === c.rel}
        data-dir={c.rel === SHAPES_DIR ? undefined : c.rel}
        onclick={() => onopen(c.rel)}
      >{c.label}</button>
    {/each}
  </nav>
  <label class="search">
    <Search size={12} />
    <input
      type="search"
      placeholder="Pesquisar nesta pasta"
      aria-label="Pesquisar nesta pasta"
      bind:value={search}
      onkeydown={(e) => {
        if (e.key === 'Escape') {
          search = '';
          e.stopPropagation();
        }
      }}
    />
  </label>
</header>

<style>
  .bar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
    height: 22px;
    padding: 0 4px;
    background: var(--bg-toolbar);
    border-bottom: 1px solid var(--line);
  }
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 20px;
    padding: 0 7px;
    font-size: var(--fs-12);
    font-weight: 500;
    color: var(--text);
    background: var(--bg-3);
    border: 1px solid var(--line);
    border-radius: var(--r-ctl);
    white-space: nowrap;
  }
  .btn:hover:not(:disabled) {
    background: var(--bg-active);
  }
  .btn:disabled,
  .icon:disabled {
    opacity: 0.45;
    pointer-events: none;
  }
  .icon {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    flex: none;
    color: var(--text-dim);
    border-radius: var(--r-ctl);
  }
  .icon:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
    overflow: hidden;
    color: var(--text-faint);
  }
  .crumb {
    height: 18px;
    padding: 0 4px;
    font-size: var(--fs-12);
    color: var(--text-dim);
    border-radius: var(--r-ctl);
    white-space: nowrap;
  }
  .crumb:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
  .crumb.last {
    color: var(--text);
  }
  .crumb.drop {
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .search {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: none;
    width: 170px;
    height: 18px;
    margin-left: auto;
    padding: 0 6px;
    color: var(--text-faint);
    background: var(--bg-2);
    border: 1px solid var(--line);
    border-radius: var(--r-ctl);
  }
  .search:focus-within {
    border-color: var(--accent);
  }
  .search input {
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    outline: none;
    font-size: var(--fs-12);
    color: var(--text);
  }
</style>
