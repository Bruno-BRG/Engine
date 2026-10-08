<script lang="ts">
  import type { DirEntry } from '$lib/state/assets.svelte';
  import AssetItem from './AssetItem.svelte';
  import type { NewItem } from './names';
  import { Folder, Clapperboard } from '@lucide/svelte';

  let {
    entries,
    tile,
    list = false,
    focused = false,
    selected = null,
    renaming = null,
    creating = $bindable(null),
    filtering = false,
    onpress,
    onopen,
    oncontext,
    onempty,
    onclear,
    oncommit,
    oncancelrename,
    oncreatecommit,
    oncreatecancel,
  }: {
    entries: DirEntry[];
    tile: number;
    list?: boolean;
    focused?: boolean;
    selected?: string | null;
    renaming?: string | null;
    creating?: NewItem | null;
    filtering?: boolean;
    onpress: (e: PointerEvent, entry: DirEntry) => void;
    onopen: (entry: DirEntry) => void;
    oncontext: (e: MouseEvent, entry: DirEntry) => void;
    onempty: (e: MouseEvent) => void;
    onclear: () => void;
    oncommit: (entry: DirEntry, typed: string) => void;
    oncancelrename: () => void;
    oncreatecommit: () => void;
    oncreatecancel: () => void;
  } = $props();

  // Enter confirma, Escape cancela, sair do campo confirma. Um desfecho vale só uma vez.
  let createSettled = false;
  function startCreate(node: HTMLInputElement) {
    createSettled = false;
    node.focus();
    node.select();
  }
  function settleCreate(commit: boolean) {
    if (createSettled) return;
    createSettled = true;
    if (commit) oncreatecommit();
    else oncreatecancel();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events (o teclado fica no painel: Escape limpa a seleção) -->
<div
  class="grid"
  class:list
  role="listbox"
  tabindex="-1"
  aria-label="Itens da pasta"
  style:--tile="{tile}px"
  onclick={(e) => { if (e.target === e.currentTarget) onclear(); }}
  oncontextmenu={(e) => { if (e.target === e.currentTarget) onempty(e); }}
>
  {#if creating}
    <div class="item new" class:list>
      {#if creating.kind === 'folder'}<Folder size={list ? 12 : 28} />{:else}<Clapperboard size={list ? 12 : 28} />{/if}
      <input
        class="rename"
        bind:value={creating.name}
        use:startCreate
        onkeydown={(e) => {
          e.stopPropagation();
          if (e.key === 'Enter') settleCreate(true);
          if (e.key === 'Escape') settleCreate(false);
        }}
        onblur={() => settleCreate(true)}
        onclick={(e) => e.stopPropagation()}
        aria-label={creating.kind === 'folder' ? 'Nome da nova pasta' : 'Nome da nova cena'}
      />
    </div>
  {/if}

  {#each entries as entry (entry.rel)}
    <AssetItem
      {entry}
      {tile}
      {list}
      {focused}
      selected={selected === entry.rel}
      renaming={renaming === entry.rel}
      onpress={(e) => onpress(e, entry)}
      onopen={() => onopen(entry)}
      oncontext={(e) => oncontext(e, entry)}
      oncommit={(typed) => oncommit(entry, typed)}
      oncancel={oncancelrename}
    />
  {:else}
    {#if !creating}
      <p class="empty">
        {#if filtering}
          Nada encontrado nesta pasta.
        {:else}
          Pasta vazia. Importe arquivos com o botão Importar, arraste arquivos para cá ou use o botão Criar.
        {/if}
      </p>
    {/if}
  {/each}
</div>

<style>
  .grid {
    display: flex;
    flex-wrap: wrap;
    align-content: flex-start;
    gap: 2px;
    min-height: 100%;
    padding: 6px;
  }
  .grid.list {
    flex-direction: column;
    flex-wrap: nowrap;
    gap: 0;
    padding: 2px 0;
  }
  .new {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    width: var(--tile);
    padding: 4px;
    color: var(--text-dim);
  }
  .new.list {
    flex-direction: row;
    align-items: center;
    width: auto;
    height: 20px;
    gap: 6px;
    padding: 0 8px 0 6px;
  }
  .rename {
    user-select: text;
    width: 100%;
    height: 18px;
    min-width: 0;
    padding: 0 4px;
    font: inherit;
    font-size: var(--fs-12);
    color: var(--text);
    background: var(--bg-2);
    border: 1px solid var(--accent);
    border-radius: var(--r-ctl);
    outline: none;
  }
  .new.list .rename {
    width: 200px;
  }
  .empty {
    width: 100%;
    margin: 0;
    padding: 16px 10px;
    font-size: var(--fs-12);
    line-height: 1.5;
    color: var(--text-faint);
  }
</style>
