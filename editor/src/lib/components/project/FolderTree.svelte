<script lang="ts">
  import { assets, SHAPES_DIR, type DirEntry } from '$lib/state/assets.svelte';
  import { dragState } from './drag.svelte';
  import FolderNode from './FolderNode.svelte';
  import { Shapes, FolderOpen } from '@lucide/svelte';

  let {
    label,
    current,
    onopen,
    oncontext,
    onpress,
  }: {
    /** Nome do projeto, para a raiz da árvore. */
    label: string;
    /** Pasta aberta no painel. */
    current: string;
    onopen: (rel: string) => void;
    oncontext: (e: MouseEvent, entry: DirEntry) => void;
    onpress: (e: PointerEvent, rel: string) => void;
  } = $props();

  const folders = $derived((assets.listingOf('') ?? []).filter((e) => e.isDir));
</script>

<div class="tree" role="tree" aria-label="Pastas do projeto">
  <button class="row" class:on={current === SHAPES_DIR} onclick={() => onopen(SHAPES_DIR)}>
    <Shapes size={12} />
    <span>Formas básicas</span>
  </button>
  <!-- svelte-ignore a11y_click_events_have_key_events (a navegação por teclado fica no painel) -->
  <div
    class="row"
    class:on={current === ''}
    class:drop={dragState.over === ''}
    data-dir=""
    role="treeitem"
    aria-selected={current === ''}
    tabindex="-1"
    onpointerdown={(e) => onpress(e, '')}
    onclick={() => onopen('')}
  >
    <FolderOpen size={12} />
    <span class="name">{label}</span>
  </div>
  {#each folders as f (f.rel)}
    <FolderNode entry={f} depth={1} {current} {onopen} {oncontext} {onpress} />
  {/each}
</div>

<style>
  .tree {
    padding: 2px 0 8px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 5px;
    width: 100%;
    height: 20px;
    padding: 0 6px 0 4px;
    font-size: var(--fs-12);
    color: var(--text);
    text-align: left;
    border-radius: 0;
    white-space: nowrap;
  }
  .row span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .row:hover {
    background: var(--bg-hover);
  }
  .row.on {
    background: var(--selection-unfocused);
    color: var(--sel-text);
  }
  .row.drop {
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .row :global(svg) {
    flex: none;
    color: var(--text-dim);
  }
  .row.on :global(svg) {
    color: var(--sel-text);
  }
</style>
