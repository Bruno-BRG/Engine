<script lang="ts">
  import type { DirEntry } from '$lib/state/assets.svelte';
  import { dragState } from './drag.svelte';
  import { thumbGen, thumbUrl } from './thumbs.svelte';
  import { editableStem, formatSize, kindLabel, withExtension } from './names';
  import AudioPreview from '../AudioPreview.svelte';
  import { Folder, Image as ImageIcon, Music, Type, Clapperboard, Workflow, File } from '@lucide/svelte';

  let {
    entry,
    tile,
    list = false,
    selected = false,
    focused = false,
    renaming = false,
    onpress,
    onopen,
    oncontext,
    oncommit,
    oncancel,
  }: {
    entry: DirEntry;
    tile: number;
    list?: boolean;
    selected?: boolean;
    focused?: boolean;
    renaming?: boolean;
    onpress: (e: PointerEvent) => void;
    onopen: () => void;
    oncontext: (e: MouseEvent) => void;
    oncommit: (typed: string) => void;
    oncancel: () => void;
  } = $props();

  const ICONS = {
    folder: Folder,
    texture: ImageIcon,
    audio: Music,
    font: Type,
    scene: Clapperboard,
    script: Workflow,
    other: File,
  };
  const Icon = $derived(ICONS[entry.kind] ?? File);

  let visible = $state(false);
  let url = $state<string | null>(null);
  // Só as miniaturas do que está (quase) na tela são lidas do disco.
  function watchVisible(node: HTMLElement) {
    const io = new IntersectionObserver(
      (es) => {
        if (es.some((x) => x.isIntersecting)) {
          visible = true;
          io.disconnect();
        }
      },
      { rootMargin: '160px' },
    );
    io.observe(node);
    return { destroy: () => io.disconnect() };
  }

  $effect(() => {
    if (!visible || entry.kind !== 'texture') return;
    const rel = entry.rel;
    void thumbGen[rel];
    let alive = true;
    thumbUrl(rel).then((u) => {
      if (alive) url = u;
    });
    return () => {
      alive = false;
    };
  });

  // Campo de renomear: mostra o nome sem a extensão e seleciona só o nome.
  let settled = false;
  function editing(node: HTMLInputElement) {
    settled = false;
    const stem = editableStem(entry);
    node.value = stem;
    node.focus();
    node.setSelectionRange(0, stem.length);
  }
  function finish(node: HTMLInputElement, commit: boolean) {
    if (settled) return;
    settled = true;
    const typed = node.value.trim();
    // Nome vazio cancela sem aviso. Sem extensão digitada, mantém a original.
    if (commit && typed) oncommit(withExtension(typed, entry));
    else oncancel();
  }

  const dragging = $derived(dragState.from === entry.rel);
</script>

{#if list}
  <div
    class="item row"
    class:sel={selected}
    class:focused
    class:dragging
    class:drop={dragState.over === entry.rel}
    class:hidden-item={entry.name.startsWith('.')}
    use:watchVisible
    role="option"
    tabindex="-1"
    aria-selected={selected}
    title={entry.rel}
    data-dir={entry.isDir ? entry.rel : undefined}
    onpointerdown={(e) => { if (!renaming) onpress(e); }}
    ondblclick={() => { if (!renaming && (entry.isDir || entry.kind === 'script')) onopen(); }}
    oncontextmenu={(e) => { e.stopPropagation(); oncontext(e); }}
  >
    <span class="ico"><Icon size={12} /></span>
    {#if renaming}
      <input
        class="rename"
        use:editing
        onpointerdown={(e) => e.stopPropagation()}
        onclick={(e) => e.stopPropagation()}
        ondblclick={(e) => e.stopPropagation()}
        onkeydown={(e) => {
          e.stopPropagation();
          if (e.key === 'Enter') finish(e.currentTarget, true);
          if (e.key === 'Escape') finish(e.currentTarget, false);
        }}
        onblur={(e) => finish(e.currentTarget, true)}
      />
    {:else}
      <span class="name">{entry.name}</span>
    {/if}
    <!-- svelte-ignore a11y_no_static_element_interactions (ouvir não seleciona nem arrasta o item) -->
    <span class="act" onpointerdown={(e) => e.stopPropagation()} ondblclick={(e) => e.stopPropagation()}>
      {#if entry.kind === 'audio' && entry.uid}<AudioPreview uid={entry.uid} compact />{/if}
    </span>
    <span class="meta">{kindLabel(entry.kind)}</span>
    <span class="meta num">{entry.isDir ? '' : formatSize(entry.size)}</span>
  </div>
{:else}
  <div
    class="item tile"
    class:sel={selected}
    class:focused
    class:dragging
    class:hidden-item={entry.name.startsWith('.')}
    class:drop={dragState.over === entry.rel}
    use:watchVisible
    role="option"
    aria-selected={selected}
    title={entry.rel}
    style:--tile="{tile}px"
    tabindex="-1"
    data-dir={entry.isDir ? entry.rel : undefined}
    onpointerdown={(e) => { if (!renaming) onpress(e); }}
    ondblclick={() => { if (!renaming && (entry.isDir || entry.kind === 'script')) onopen(); }}
    oncontextmenu={(e) => { e.stopPropagation(); oncontext(e); }}
  >
    <div class="thumb">
      {#if url}
        <img src={url} alt="" draggable="false" />
      {:else}
        <span class="glyph"><Icon size={Math.max(20, Math.round(tile * 0.32))} /></span>
      {/if}
    </div>
    {#if entry.kind === 'audio' && entry.uid}
      <!-- svelte-ignore a11y_no_static_element_interactions (ouvir não seleciona nem arrasta o item) -->
      <span class="act tile-act" onpointerdown={(e) => e.stopPropagation()} ondblclick={(e) => e.stopPropagation()}>
        <AudioPreview uid={entry.uid} compact />
      </span>
    {/if}
    {#if renaming}
      <input
        class="rename"
        use:editing
        onpointerdown={(e) => e.stopPropagation()}
        onclick={(e) => e.stopPropagation()}
        ondblclick={(e) => e.stopPropagation()}
        onkeydown={(e) => {
          e.stopPropagation();
          if (e.key === 'Enter') finish(e.currentTarget, true);
          if (e.key === 'Escape') finish(e.currentTarget, false);
        }}
        onblur={(e) => finish(e.currentTarget, true)}
      />
    {:else}
      <span class="name">{entry.name}</span>
    {/if}
  </div>
{/if}

<style>
  .item {
    position: relative;
    color: var(--text);
    font-size: var(--fs-12);
    border-radius: var(--r-ctl);
    user-select: none;
    cursor: default;
  }
  .item:hover {
    background: var(--bg-hover);
  }
  .item.sel {
    background: var(--selection-unfocused);
    color: var(--sel-text);
  }
  .item.sel.focused {
    background: var(--selection);
  }
  .item.dragging {
    opacity: 0.45;
  }
  .item.drop {
    box-shadow: inset 0 0 0 1px var(--accent);
    background: var(--accent-soft);
  }
  .hidden-item {
    opacity: 0.6;
  }

  /* Grade: miniatura quadrada com o nome embaixo */
  .tile {
    width: var(--tile);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    padding: 4px;
  }
  .thumb {
    width: calc(var(--tile) - 10px);
    height: calc(var(--tile) - 10px);
    display: grid;
    place-items: center;
    overflow: hidden;
    background: var(--bg-0);
    border: 1px solid var(--line-soft);
    border-radius: var(--r-ctl);
    color: var(--text-dim);
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .glyph {
    display: grid;
    place-items: center;
  }
  .tile .name,
  .tile .rename {
    max-width: 100%;
    text-align: center;
  }
  .name {
    display: block;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Lista: linha de 20 px, como a Hierarquia */
  .row {
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr) 22px 72px 56px;
    align-items: center;
    gap: 6px;
    height: 20px;
    padding: 0 8px 0 6px;
  }
  .act {
    display: inline-flex;
    justify-content: center;
  }
  .tile-act {
    position: absolute;
    top: 6px;
    right: 6px;
  }
  .row .ico {
    display: grid;
    place-items: center;
    color: var(--text-dim);
  }
  .row .name {
    min-width: 0;
  }
  .meta {
    font-size: var(--fs-11);
    color: var(--text-faint);
    white-space: nowrap;
  }
  .meta.num {
    text-align: right;
    font-family: var(--font-mono);
  }
  .item.sel .meta {
    color: var(--sel-text);
  }

  .rename {
    user-select: text;
    height: 18px;
    min-width: 0;
    width: 100%;
    padding: 0 4px;
    font: inherit;
    color: var(--text);
    background: var(--bg-2);
    border: 1px solid var(--accent);
    border-radius: var(--r-ctl);
    outline: none;
  }
</style>
