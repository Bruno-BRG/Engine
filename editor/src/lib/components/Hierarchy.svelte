<script lang="ts">
  import { tick } from 'svelte';
  import { scene, type NodeView } from '$lib/state/scene.svelte';
  import { focus, focusPanel } from '$lib/state/focus.svelte';
  import { showContextMenu, type MenuItem } from '$lib/state/contextmenu.svelte';
  import { deleteSelected, duplicateSelected, copySelected, pasteClipboard } from '$lib/actions';
  import { log } from '$lib/state/log.svelte';
  import { app } from '$lib/state/app.svelte';
  import { assets, messageOf } from '$lib/state/assets.svelte';
  import { platform } from '$lib/platform';
  import PanelHeader from './PanelHeader.svelte';
  import NodeIcon from './NodeIcon.svelte';
  import { Plus, Eye, EyeOff, Lock, LockOpen, Search, ChevronRight, ChevronDown, Clapperboard } from '@lucide/svelte';

  const norm = (s: string) => s.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase();

  let filter = $state('');
  let collapsed = $state<Set<string>>(new Set());
  let renaming = $state<string | null>(null);
  let renameText = $state('');
  let dragUid = $state<string | null>(null);
  let drop = $state<{ uid: string | null; pos: 'before' | 'inside' | 'after' } | null>(null);
  let anchor = $state<string | null>(null);

  /** Filtro: `t:Sprite2D` filtra pelo tipo; o resto procura pelo nome. */
  const query = $derived.by(() => {
    const type = filter.match(/t:([A-Za-z0-9]+)/i)?.[1] ?? null;
    const text = norm(filter.replace(/t:[A-Za-z0-9]+/gi, '').trim());
    return { type, text, active: !!type || !!text };
  });

  /** Linhas visíveis, em ordem de árvore. Nós de grupos recolhidos somem; com filtro, a lista é plana. */
  const rows = $derived.by(() => {
    const out: NodeView[] = [];
    if (query.active) {
      for (const n of scene.data.nodes) {
        if (query.type && n.kind !== query.type) continue;
        if (query.text && !norm(n.name).includes(query.text)) continue;
        out.push(n);
      }
      return out;
    }
    let hideBelow = -1;
    for (const n of scene.data.nodes) {
      if (hideBelow >= 0 && n.depth > hideBelow) continue;
      hideBelow = -1;
      out.push(n);
      if (collapsed.has(n.uid) && n.children.length) hideBelow = n.depth;
    }
    return out;
  });

  function toggleCollapsed(uid: string) {
    const next = new Set(collapsed);
    if (next.has(uid)) next.delete(uid);
    else next.add(uid);
    collapsed = next;
  }

  function onRowClick(e: MouseEvent, n: NodeView) {
    if (e.shiftKey && anchor) {
      // Seleção por intervalo entre a âncora e o nó clicado.
      const ids = rows.map((r) => r.uid);
      const a = ids.indexOf(anchor);
      const b = ids.indexOf(n.uid);
      if (a >= 0 && b >= 0) {
        const [lo, hi] = a < b ? [a, b] : [b, a];
        scene.selection = ids.slice(lo, hi + 1);
        return;
      }
    }
    scene.select(n.uid, e.ctrlKey || e.metaKey);
    anchor = n.uid;
  }

  async function startRename(uid: string) {
    const n = scene.node(uid);
    if (!n) return;
    renaming = uid;
    renameText = n.name;
    await tick();
    const el = document.getElementById('rename-' + uid) as HTMLInputElement | null;
    el?.focus();
    el?.select();
  }

  function commitRename() {
    if (renaming) {
      const name = renameText.trim();
      if (name && name !== scene.node(renaming)?.name) scene.rename(renaming, name);
    }
    renaming = null;
  }

  function toggleProp(e: MouseEvent, uid: string, key: 'visible' | 'locked') {
    e.stopPropagation();
    const n = scene.node(uid);
    if (!n) return;
    scene.setProp(uid, key, key === 'visible' ? !n.visible : !n.props.locked);
  }

  // ---------- Arrastar para reordenar e reparentar (SCN-021) ----------

  function dropPos(e: DragEvent, el: HTMLElement): 'before' | 'inside' | 'after' {
    const r = el.getBoundingClientRect();
    const y = (e.clientY - r.top) / r.height;
    return y < 0.25 ? 'before' : y > 0.75 ? 'after' : 'inside';
  }

  function onDragStart(e: DragEvent, uid: string) {
    dragUid = uid;
    e.dataTransfer?.setData('application/x-nexo-node', uid);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
  }

  function onDragOver(e: DragEvent, uid: string | null, el: HTMLElement) {
    if (!dragUid && !e.dataTransfer?.types.includes('application/x-nexo-node')) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    drop = { uid, pos: uid == null ? 'inside' : dropPos(e, el) };
  }

  function onDrop(e: DragEvent) {
    e.preventDefault();
    const uid = dragUid ?? e.dataTransfer?.getData('application/x-nexo-node') ?? null;
    const target = drop;
    dragUid = null;
    drop = null;
    if (!uid || !target) return;
    moveNode(uid, target.uid, target.pos);
  }

  /** Move `uid` para antes, dentro ou depois de `targetUid`; `null` é a raiz. */
  function moveNode(uid: string, targetUid: string | null, pos: 'before' | 'inside' | 'after') {
    if (targetUid === uid) return;
    if (targetUid == null || pos === 'inside') {
      const parent = targetUid;
      const count = scene.children(parent).filter((c) => c.uid !== uid).length;
      scene.move(uid, parent, count);
      return;
    }
    const target = scene.node(targetUid);
    if (!target) return;
    const siblings = scene.children(target.parent).map((c) => c.uid).filter((u) => u !== uid);
    const i = siblings.indexOf(targetUid);
    scene.move(uid, target.parent, pos === 'before' ? i : i + 1);
  }

  function onDragEnd() {
    dragUid = null;
    drop = null;
  }

  // ---------- Menus e teclado ----------

  function contextMenu(e: MouseEvent, uid: string | null) {
    // A linha e a lista ouvem o mesmo clique; só o primeiro (o da linha, com UID) pode abrir o menu.
    e.preventDefault();
    e.stopPropagation();
    if (uid && !scene.isSelected(uid)) scene.select(uid);
    const parent = uid;
    const creatable = scene.types.filter((t) => t.name !== 'Node');
    const items: MenuItem[] = [
      ...creatable.map((t) => ({
        label: 'Criar ' + t.name,
        keys: undefined,
        enabled: t.name !== 'Behavior' || !!uid,
        run: () => {
          const created = scene.add(t.name, parent);
          if (created) scene.select(created);
        },
      })),
      'sep',
      { label: 'Renomear', keys: 'F2', enabled: !!uid, run: () => uid && startRename(uid) },
      { label: 'Duplicar', keys: 'Ctrl+D', enabled: scene.selection.length > 0, run: duplicateSelected },
      { label: 'Excluir', keys: 'Delete', enabled: scene.selection.length > 0, run: deleteSelected },
      { label: 'Salvar ramo como cena…', enabled: !!uid && !!app.project, run: () => uid && saveBranch(uid) },
      'sep',
      { label: 'Copiar', keys: 'Ctrl+C', enabled: scene.selection.length > 0, run: copySelected },
      { label: 'Colar', keys: 'Ctrl+V', enabled: scene.hasClipboard, run: pasteClipboard },
      'sep',
      {
        label: scene.selected ? 'Trocar tipo: ' + scene.selected.kind : 'Trocar tipo',
        enabled: false,
        run: () => {},
      },
      ...(scene.selected
        ? scene.types
            .filter((t) => t.name !== scene.selected?.kind && t.name !== 'Node')
            .map((t) => ({ label: '   Para ' + t.name, run: () => scene.selectedId && scene.changeKind(scene.selectedId, t.name) }))
        : []),
    ];
    showContextMenu(e, items);
  }

  function onKey(e: KeyboardEvent) {
    if (renaming) return;
    const ids = rows.map((r) => r.uid);
    const cur = scene.selectedId;
    const i = cur ? ids.indexOf(cur) : -1;
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const next = ids[Math.max(0, Math.min(ids.length - 1, i + (e.key === 'ArrowDown' ? 1 : -1)))];
      if (next) {
        scene.select(next, false);
        anchor = next;
      }
    } else if (e.key === 'F2' && cur) {
      e.preventDefault();
      startRename(cur);
    } else if (e.key === 'ArrowRight' && cur) {
      if (collapsed.has(cur)) toggleCollapsed(cur);
    } else if (e.key === 'ArrowLeft' && cur) {
      if (!collapsed.has(cur)) toggleCollapsed(cur);
    }
  }

  /** Grava a subárvore como uma cena nova em scenes/ (SCN-022). Nunca sobrescreve uma cena existente. */
  async function saveBranch(uid: string) {
    const n = scene.node(uid);
    const project = app.project?.path;
    if (!n || !project) return;
    try {
      const text = scene.branchText(uid);
      await assets.load('scenes');
      const taken = new Set((assets.listingOf('scenes') ?? []).map((e) => e.name));
      const base = n.name.replace(/[\/:*?"<>|]+/g, ' ').trim() || 'Ramo';
      let name = base;
      for (let i = 2; taken.has(name + '.scene.ron'); i++) name = `${base} ${i}`;
      const rel = `scenes/${name}.scene.ron`;
      await platform.writeProjectText(project, rel, text);
      await assets.refresh();
      log.toast('success', 'Ramo salvo como cena', rel);
    } catch (e) {
      log.toast('error', 'Não foi possível salvar o ramo', messageOf(e));
    }
  }

  function countSprites() {
    return scene.data.nodes.filter((n) => n.kind === 'Sprite2D').length;
  }
</script>

<section class="hier" use:focusPanel={'hierarchy'} class:focused={focus.panel === 'hierarchy'} aria-label="Hierarquia da cena">
  <PanelHeader title="Hierarquia">
    <button class="icon-btn" title="Criar" aria-label="Criar nó" onclick={(e) => contextMenu(e, scene.selectedId)}>
      <Plus size={13} />
    </button>
  </PanelHeader>

  <div class="search">
    <Search size={12} />
    <input bind:value={filter} placeholder="Buscar ou t:Sprite2D" aria-label="Buscar na hierarquia" />
  </div>

  <!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_no_noninteractive_element_to_interactive_role -->
  <div
    class="list"
    role="tree"
    tabindex="0"
    onkeydown={onKey}
    oncontextmenu={(e) => contextMenu(e, null)}
  >
    <div
      class="scene-row"
      class:drop-inside={drop?.uid == null && drop}
      role="treeitem"
      tabindex="-1"
      aria-selected="false"
      ondragover={(e) => onDragOver(e, null, e.currentTarget as HTMLElement)}
      ondrop={onDrop}
    >
      <Clapperboard size={12} />
      <span class="scene-name">{scene.data.name}</span>
      <em class="mono">{scene.data.nodes.length} nós · {countSprites()} sprites</em>
    </div>

    {#each rows as n (n.uid)}
      {@const hasKids = n.children.length > 0}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div
        class="row"
        class:selected={scene.isSelected(n.uid)}
        class:hidden={!n.visible}
        class:hovered={scene.hoveredId === n.uid}
        class:drop-before={drop?.uid === n.uid && drop.pos === 'before'}
        class:drop-after={drop?.uid === n.uid && drop.pos === 'after'}
        class:drop-inside={drop?.uid === n.uid && drop.pos === 'inside'}
        class:dragging={dragUid === n.uid}
        role="treeitem"
        tabindex="-1"
        aria-selected={scene.isSelected(n.uid)}
        aria-expanded={hasKids ? !collapsed.has(n.uid) : undefined}
        draggable={renaming !== n.uid}
        style="--depth: {n.depth}"
        onclick={(e) => onRowClick(e, n)}
        ondblclick={() => startRename(n.uid)}
        oncontextmenu={(e) => contextMenu(e, n.uid)}
        onpointerenter={() => (scene.hoveredId = n.uid)}
        onpointerleave={() => (scene.hoveredId = null)}
        ondragstart={(e) => onDragStart(e, n.uid)}
        ondragover={(e) => onDragOver(e, n.uid, e.currentTarget as HTMLElement)}
        ondrop={onDrop}
        ondragend={onDragEnd}
      >
        <span class="indent" aria-hidden="true"></span>
        {#if hasKids}
          <button
            class="twisty"
            aria-label={collapsed.has(n.uid) ? 'Expandir' : 'Recolher'}
            onclick={(e) => {
              e.stopPropagation();
              toggleCollapsed(n.uid);
            }}
          >
            {#if collapsed.has(n.uid)}<ChevronRight size={11} />{:else}<ChevronDown size={11} />{/if}
          </button>
        {:else}
          <span class="twisty-gap"></span>
        {/if}
        <NodeIcon kind={n.kind} size={12} />
        {#if renaming === n.uid}
          <input
            id={'rename-' + n.uid}
            class="rename"
            bind:value={renameText}
            onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => {
              if (e.key === 'Enter') commitRename();
              if (e.key === 'Escape') renaming = null;
              e.stopPropagation();
            }}
            onblur={commitRename}
          />
        {:else}
          <span class="name">{n.name}</span>
          <span class="kind mono">{n.kind}</span>
        {/if}
        <button
          class="mini"
          class:on={!n.visible}
          title={n.visible ? 'Ocultar' : 'Mostrar'}
          aria-label={n.visible ? 'Ocultar nó' : 'Mostrar nó'}
          onclick={(e) => toggleProp(e, n.uid, 'visible')}
        >
          {#if n.visible}<Eye size={11} />{:else}<EyeOff size={11} />{/if}
        </button>
        <button
          class="mini"
          class:on={!!n.props.locked}
          title={n.props.locked ? 'Destravar' : 'Travar no editor'}
          aria-label={n.props.locked ? 'Destravar nó' : 'Travar nó'}
          onclick={(e) => toggleProp(e, n.uid, 'locked')}
        >
          {#if n.props.locked}<Lock size={11} />{:else}<LockOpen size={11} />{/if}
        </button>
      </div>
    {/each}

    {#if rows.length === 0 && query.active}
      <div class="empty">Nenhum nó corresponde a “{filter}”.</div>
    {/if}
  </div>

  {#if scene.data.nodes.length === 0}
    <div class="empty-scene">
      <strong>Cena sem nós</strong>
      <span>Clique com o botão direito aqui para criar o primeiro.</span>
    </div>
  {/if}
</section>

<style>
  .hier {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--bg-1);
  }
  .icon-btn {
    width: 18px;
    height: 18px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--r-ctl);
    color: var(--text-dim);
  }
  .icon-btn:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
  .search {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 8px;
    color: var(--text-faint);
    border-bottom: 1px solid var(--line-soft);
  }
  .search input {
    flex: 1;
    min-width: 0;
    background: none;
    border: 0;
    outline: none;
    color: var(--text);
    font-size: var(--fs-12);
  }
  .search input::placeholder {
    color: var(--text-faint);
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 2px 0 8px;
    outline: none;
  }
  .scene-row {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    padding: 0 8px;
    color: var(--text-dim);
    font-size: var(--fs-12);
  }
  .scene-name {
    color: var(--text);
    font-weight: 600;
  }
  .scene-row em {
    margin-left: auto;
    font-style: normal;
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 5px;
    height: 20px;
    padding-right: 4px;
    padding-left: calc(6px + var(--depth) * 14px);
    font-size: var(--fs-12);
    color: var(--text-dim);
    user-select: none;
    cursor: default;
  }
  .row:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .row.selected {
    background: var(--selection-unfocused);
    color: var(--text);
  }
  .list:focus-within .row.selected,
  :global(.hier.focused) .row.selected {
    background: var(--selection);
  }
  .row.hidden .name,
  .row.hidden :global(svg:not(.hide-off)) {
    opacity: 0.5;
  }
  .row.dragging {
    opacity: 0.45;
  }
  .row.drop-before::before,
  .row.drop-after::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--accent-hi);
  }
  .row.drop-before::before {
    top: 0;
  }
  .row.drop-after::after {
    bottom: 0;
  }
  .row.drop-inside,
  .scene-row.drop-inside {
    box-shadow: inset 0 0 0 1px var(--accent-hi);
  }
  .indent {
    display: none;
  }
  .twisty,
  .twisty-gap {
    width: 12px;
    height: 16px;
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-faint);
  }
  .twisty:hover {
    color: var(--text);
  }
  .name {
    flex: none;
    max-width: 60%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .kind {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .row.selected .kind {
    color: var(--text-dim);
  }
  .rename {
    flex: 1;
    min-width: 0;
    height: 18px;
    padding: 0 4px;
    border-radius: var(--r-ctl);
    background: var(--bg-2);
    color: var(--text);
    box-shadow: inset 0 0 0 1px var(--accent);
    outline: none;
    font-size: var(--fs-12);
  }
  .mini {
    flex: none;
    width: 16px;
    height: 16px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--r-ctl);
    color: var(--text-faint);
    opacity: 0;
  }
  .row:hover .mini,
  .row.selected .mini,
  .mini.on {
    opacity: 1;
  }
  .mini:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
  .mini.on {
    color: var(--text-dim);
  }
  .empty,
  .empty-scene {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px;
    font-size: var(--fs-12);
    color: var(--text-faint);
  }
  .empty-scene {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 12px;
    text-align: center;
    pointer-events: none;
  }
  .empty-scene strong {
    color: var(--text-dim);
  }
</style>
