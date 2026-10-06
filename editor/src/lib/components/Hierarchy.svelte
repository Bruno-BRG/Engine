<script lang="ts">
  import { tick } from 'svelte';
  import { scene, type SceneObject } from '$lib/state/scene.svelte';
  import { app } from '$lib/state/app.svelte';
  import { viewApi } from '$lib/state/view';
  import * as act from '$lib/actions';
  import PanelHeader from './PanelHeader.svelte';
  import ShapeIcon from './ShapeIcon.svelte';
  import { Plus, Eye, EyeOff, Lock, LockOpen, Search, Clapperboard } from '@lucide/svelte';

  let filter = $state('');
  let renaming = $state<string | null>(null);
  let renameText = $state('');
  let addOpen = $state(false);
  let dragId = $state<string | null>(null);
  let dropAt = $state<number | null>(null);

  const norm = (s: string) => s.normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLowerCase();
  // Itens no topo da lista são desenhados na frente.
  const rows = $derived(
    [...scene.data.objects].reverse().filter((o) => !filter || norm(o.name).includes(norm(filter))),
  );

  async function startRename(o: SceneObject) {
    renaming = o.id;
    renameText = o.name;
    await tick();
    const el = document.getElementById('rename-' + o.id) as HTMLInputElement | null;
    el?.focus();
    el?.select();
  }
  function finishRename(o: SceneObject) {
    const v = renameText.trim();
    if (renaming && v && v !== o.name) scene.update(o.id, { name: v }, 'Renomear');
    renaming = null;
  }

  function keyNav(e: KeyboardEvent) {
    if (renaming) return;
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp' && e.key !== 'F2' && e.key !== 'Enter') return;
    e.preventDefault();
    const i = rows.findIndex((r) => r.id === scene.selectedId);
    if (e.key === 'F2' || e.key === 'Enter') {
      const o = rows[i];
      if (o) startRename(o);
      return;
    }
    const j = Math.max(0, Math.min(rows.length - 1, i + (e.key === 'ArrowDown' ? 1 : -1)));
    if (rows[j]) scene.selectedId = rows[j].id;
  }

  function drop(targetRow: number) {
    if (!dragId) return;
    // Converte a posição da lista (invertida) para o índice na cena.
    const n = scene.data.objects.length;
    const from = scene.data.objects.findIndex((o) => o.id === dragId);
    let to = n - targetRow;
    if (from < to) to -= 1;
    to = Math.max(0, Math.min(n - 1, to));
    if (from !== to)
      scene.mutate('Reordenar', (d) => {
        const [o] = d.objects.splice(from, 1);
        d.objects.splice(to, 0, o);
      });
    dragId = null;
    dropAt = null;
  }
</script>

<PanelHeader title="Hierarquia">
  <div class="add-wrap">
    <button class="icon-btn" title="Adicionar objeto" onclick={() => (addOpen = !addOpen)} disabled={app.playing}><Plus size={15} /></button>
    {#if addOpen}
      <button class="scrim" aria-label="Fechar" onclick={() => (addOpen = false)}></button>
      <div class="menu">
        {#each [0, 1, 2] as const as s}
          <button onclick={() => { addOpen = false; act.addShape(s); }}>
            <ShapeIcon shape={s} color="var(--text-dim)" />
            {['Retângulo', 'Círculo', 'Losango'][s]}
          </button>
        {/each}
      </div>
    {/if}
  </div>
</PanelHeader>

<div class="search">
  <Search size={13} />
  <input placeholder="Filtrar objetos" bind:value={filter} spellcheck="false" />
</div>

<div class="scene-row">
  <Clapperboard size={14} />
  <span>{scene.data.name}</span>
  <em>{scene.data.objects.length}</em>
</div>

<div class="list" role="tree" tabindex="0" onkeydown={keyNav} ondragleave={() => (dropAt = null)}>
  {#each rows as o, i (o.id)}
    <!-- svelte-ignore a11y_click_events_have_key_events (a navegação por teclado fica na lista) -->
    <div
      class="row"
      class:selected={scene.selectedId === o.id}
      class:hovered={scene.hoveredId === o.id}
      class:hidden={!o.visible}
      class:drop-before={dropAt === i}
      class:drop-after={dropAt === rows.length && i === rows.length - 1}
      role="treeitem"
      aria-selected={scene.selectedId === o.id}
      tabindex="-1"
      draggable={!filter && !renaming}
      ondragstart={(e) => { dragId = o.id; e.dataTransfer?.setData('text/plain', o.id); }}
      ondragover={(e) => { if (!dragId) return; e.preventDefault(); const r = (e.currentTarget as HTMLElement).getBoundingClientRect(); dropAt = e.clientY > r.top + r.height / 2 ? i + 1 : i; }}
      ondrop={(e) => { e.preventDefault(); if (dropAt != null) drop(dropAt); }}
      ondragend={() => { dragId = null; dropAt = null; }}
      onclick={() => (scene.selectedId = o.id)}
      ondblclick={() => startRename(o)}
      onpointerenter={() => (scene.hoveredId = o.id)}
      onpointerleave={() => scene.hoveredId === o.id && (scene.hoveredId = null)}
    >
      <ShapeIcon shape={o.shape} color={o.color} />
      {#if renaming === o.id}
        <input
          id={'rename-' + o.id}
          class="rename"
          bind:value={renameText}
          onblur={() => finishRename(o)}
          onkeydown={(e) => { e.stopPropagation(); if (e.key === 'Enter') finishRename(o); if (e.key === 'Escape') renaming = null; }}
          onclick={(e) => e.stopPropagation()}
        />
      {:else}
        <span class="name">{o.name}</span>
      {/if}
      {#if o.behaviors.length}<span class="badge" title="Behaviors">{o.behaviors.length}</span>{/if}
      <button class="mini" class:on={o.locked} title={o.locked ? 'Destravar' : 'Travar'} onclick={(e) => { e.stopPropagation(); scene.update(o.id, { locked: !o.locked }, o.locked ? 'Destravar' : 'Travar'); }}>
        {#if o.locked}<Lock size={12} />{:else}<LockOpen size={12} />{/if}
      </button>
      <button class="mini" class:on={!o.visible} title={o.visible ? 'Ocultar' : 'Mostrar'} onclick={(e) => { e.stopPropagation(); scene.update(o.id, { visible: !o.visible }, o.visible ? 'Ocultar' : 'Mostrar'); }}>
        {#if o.visible}<Eye size={12} />{:else}<EyeOff size={12} />{/if}
      </button>
    </div>
  {:else}
    <div class="empty">
      {#if filter}Nada encontrado para “{filter}”.{:else}A cena está vazia. Use <strong>+</strong> ou arraste uma forma do painel Assets.{/if}
    </div>
  {/each}
</div>

<style>
  .add-wrap {
    position: relative;
  }
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 10;
    cursor: default;
  }
  .menu {
    position: absolute;
    right: 0;
    top: 32px;
    z-index: 11;
    min-width: 170px;
    padding: 5px;
    border-radius: var(--r-panel);
    background: var(--bg-2);
    box-shadow: var(--shadow-pop);
    animation: pop var(--t-med) var(--ease);
  }
  .menu button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 30px;
    padding: 0 10px;
    border-radius: var(--r-ctl);
  }
  .menu button:hover {
    background: var(--accent-soft);
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
  .search {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 8px 8px 4px;
    padding: 0 9px;
    height: 28px;
    border-radius: var(--r-ctl);
    background: var(--bg-0);
    border: 1px solid var(--line-soft);
    color: var(--text-faint);
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
  }
  .scene-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 14px;
    color: var(--text-dim);
    font-size: var(--fs-12);
    font-weight: 600;
  }
  .scene-row em {
    margin-left: auto;
    font-style: normal;
    color: var(--text-faint);
    font-weight: 500;
  }
  .list {
    flex: 1;
    overflow-y: auto;
    padding: 0 6px 10px;
    outline: none;
  }
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 9px;
    height: 30px;
    padding: 0 6px 0 16px;
    border-radius: 7px;
    color: var(--text);
    transition: background 80ms;
  }
  .row:hover,
  .row.hovered {
    background: var(--bg-hover);
  }
  .row.selected {
    background: var(--accent-soft);
  }
  .row.selected::before {
    content: '';
    position: absolute;
    left: 5px;
    top: 8px;
    bottom: 8px;
    width: 3px;
    border-radius: 3px;
    background: var(--accent);
  }
  .row.hidden .name,
  .row.hidden :global(svg:first-child) {
    opacity: 0.4;
  }
  .row.drop-before {
    box-shadow: 0 -2px 0 var(--accent);
  }
  .row.drop-after {
    box-shadow: 0 2px 0 var(--accent);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rename {
    flex: 1;
    min-width: 0;
    height: 22px;
    padding: 0 6px;
    border-radius: 4px;
    background: var(--bg-0);
    border: 1px solid var(--accent);
    outline: none;
  }
  .badge {
    font-size: 10px;
    font-weight: 600;
    color: var(--accent-hi);
    background: var(--accent-soft);
    padding: 0 6px;
    border-radius: 99px;
    line-height: 16px;
  }
  .mini {
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    border-radius: 5px;
    color: var(--text-faint);
    opacity: 0;
    transition: opacity var(--t-fast), background var(--t-fast);
  }
  .row:hover .mini,
  .mini.on {
    opacity: 1;
  }
  .mini:hover {
    background: var(--bg-active);
    color: var(--text);
  }
  .empty {
    padding: 18px 12px;
    color: var(--text-faint);
    font-size: var(--fs-12);
    line-height: 1.5;
  }
  .empty strong {
    color: var(--text-dim);
  }
</style>
