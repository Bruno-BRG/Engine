<script lang="ts">
  import { assets, type DirEntry } from '$lib/state/assets.svelte';
  import { dragState } from './drag.svelte';
  import FolderNode from './FolderNode.svelte';
  import { ChevronRight, ChevronDown, Folder, FolderOpen } from '@lucide/svelte';

  let {
    entry,
    depth,
    current,
    onopen,
    oncontext,
    onpress,
  }: {
    entry: DirEntry;
    depth: number;
    current: string;
    onopen: (rel: string) => void;
    oncontext: (e: MouseEvent, entry: DirEntry) => void;
    onpress: (e: PointerEvent, rel: string) => void;
  } = $props();

  let expanded = $state(false);
  const children = $derived((assets.listingOf(entry.rel) ?? []).filter((e) => e.isDir));

  // Pasta aberta por dentro da árvore (ou pelo breadcrumb): as pastas acima dela abrem sozinhas.
  $effect(() => {
    if (current.startsWith(entry.rel + '/')) expanded = true;
  });
  $effect(() => {
    if (expanded) void assets.load(entry.rel);
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events (a navegação por teclado fica no painel) -->
<div
  class="row"
  class:on={current === entry.rel}
  class:drop={dragState.over === entry.rel}
  class:dragging={dragState.from === entry.rel}
  data-dir={entry.rel}
  role="treeitem"
  aria-selected={current === entry.rel}
  aria-expanded={expanded}
  tabindex="-1"
  style:padding-left="{4 + depth * 14}px"
  onpointerdown={(e) => onpress(e, entry.rel)}
  onclick={() => onopen(entry.rel)}
  ondblclick={() => (expanded = !expanded)}
  oncontextmenu={(e) => {
    e.stopPropagation();
    oncontext(e, entry);
  }}
>
  <button
    class="chev"
    aria-label={expanded ? 'Recolher pasta' : 'Expandir pasta'}
    onpointerdown={(e) => e.stopPropagation()}
    onclick={(e) => {
      e.stopPropagation();
      expanded = !expanded;
    }}
  >
    {#if expanded}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
  </button>
  {#if expanded}<FolderOpen size={12} />{:else}<Folder size={12} />{/if}
  <span class="name">{entry.name}</span>
</div>

{#if expanded}
  {#each children as c (c.rel)}
    <FolderNode entry={c} depth={depth + 1} {current} {onopen} {oncontext} {onpress} />
  {/each}
{/if}

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 20px;
    padding-right: 6px;
    font-size: var(--fs-12);
    color: var(--text);
    white-space: nowrap;
    cursor: default;
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
  .row.dragging {
    opacity: 0.45;
  }
  .row :global(svg) {
    flex: none;
    color: var(--text-dim);
  }
  .row.on :global(svg) {
    color: var(--sel-text);
  }
  .chev {
    display: grid;
    place-items: center;
    width: 14px;
    height: 16px;
    flex: none;
    color: var(--text-faint);
    border-radius: var(--r-ctl);
  }
  .chev:hover {
    color: var(--text);
    background: var(--bg-active);
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
