<script lang="ts">
  // Painel Projeto (AST-001): árvore de pastas à esquerda, grade de assets à direita.
  // Os arquivos ficam no disco e o backend é a fonte da verdade. Aqui só mostramos e disparamos ações.
  import { onMount } from 'svelte';
  import { assets, messageOf, parentOf, SHAPES_DIR, type DirEntry } from '$lib/state/assets.svelte';
  import '$lib/state/textures.svelte';
  import { isTauri } from '$lib/platform';
  import { app } from '$lib/state/app.svelte';
  import { focus, focusPanel } from '$lib/state/focus.svelte';
  import { log } from '$lib/state/log.svelte';
  import { showContextMenu, type MenuItem } from '$lib/state/contextmenu.svelte';
  import FolderTree from './project/FolderTree.svelte';
  import AssetGrid from './project/AssetGrid.svelte';
  import ShapeGrid from './project/ShapeGrid.svelte';
  import ProjectBar from './project/ProjectBar.svelte';
  import ProjectFooter from './project/ProjectFooter.svelte';
  import { dragState, startDrag } from './project/drag.svelte';
  import { createMenu, itemMenu } from './project/menus';
  import { importFilesTo, pickImportFiles } from './project/imports';
  import { baseName, editableStem, invalidName, normalize, scriptNameOf, uniqueName, type NewItem } from './project/names';
  import { logic } from '$lib/logic/store.svelte';

  const SIZE_KEY = 'nexo.project.v1';

  function loadSize(): number {
    try {
      const v = Number(JSON.parse(localStorage.getItem(SIZE_KEY) ?? '{}').size);
      return Number.isFinite(v) ? Math.min(100, Math.max(0, v)) : 50;
    } catch {
      // Sem armazenamento (janela privada, dados bloqueados): usa o padrão.
      return 50;
    }
  }
  function saveSize(v: number) {
    try {
      localStorage.setItem(SIZE_KEY, JSON.stringify({ size: v }));
    } catch {
      // Sem armazenamento: a preferência vale só nesta sessão.
    }
  }

  // Tamanho 0 é a lista. Acima de 0 é a grade, de 48 a 160 px por item.
  let size = $state(loadSize());
  $effect(() => saveSize(size));
  const list = $derived(size === 0);
  const tile = $derived(Math.round(48 + size * 1.12));

  let rootEl = $state<HTMLElement>();
  let search = $state('');
  let selected = $state<string | null>(null);
  let renaming = $state<string | null>(null);
  let creating = $state<NewItem | null>(null);
  let confirming = $state<string | null>(null);
  let dropHover = $state(false);

  const onShapes = $derived(assets.dir === SHAPES_DIR);
  // Criar, importar e renomear só dentro de uma pasta do projeto, no app desktop.
  const writable = $derived(isTauri && !!assets.root && !onShapes);
  const filtering = $derived(search.trim() !== '');
  const shown = $derived(
    filtering ? assets.entries.filter((e) => normalize(e.name).includes(normalize(search.trim()))) : assets.entries,
  );
  const selectedEntry = $derived(selected ? (assets.entries.find((e) => e.rel === selected) ?? null) : null);
  const loaded = $derived(assets.listingOf(assets.dir) !== undefined);
  const focused = $derived(focus.panel === 'project');
  const projectName = $derived(app.project?.name ?? 'Projeto');
  const countLabel = $derived(`${shown.length} ${shown.length === 1 ? 'item' : 'itens'}`);

  const crumbs = $derived.by(() => {
    if (onShapes) return [{ label: 'Formas básicas', rel: SHAPES_DIR }];
    const out = [{ label: projectName, rel: '' }];
    const parts = assets.dir ? assets.dir.split('/') : [];
    parts.forEach((p, i) => out.push({ label: p, rel: parts.slice(0, i + 1).join('/') }));
    return out;
  });

  const join = (dir: string, name: string) => (dir ? `${dir}/${name}` : name);

  function clearTransient() {
    renaming = null;
    creating = null;
    confirming = null;
    selected = null;
  }

  function open(rel: string) {
    clearTransient();
    assets.open(rel);
  }

  function up() {
    clearTransient();
    assets.up();
  }

  // ---------- Mover por arraste (ponteiro) ----------

  /** Mover `from` para `to` é válido: não para dentro de si mesmo nem para a pasta onde já está. */
  function canDrop(from: string, to: string): boolean {
    if (to.startsWith('@') || to === from || to.startsWith(from + '/')) return false;
    // Script se acha pelo nome, dentro de `scripts/`: arrastá-lo para outra pasta quebraria os nós que o usam.
    if (entryOf(from)?.kind === 'script') return false;
    return parentOf(from) !== to;
  }

  function entryOf(rel: string): DirEntry | undefined {
    return assets.entries.find((e) => e.rel === rel);
  }

  async function moveTo(from: string, to: string) {
    const dest = join(to, baseName(from));
    try {
      await assets.rename(from, dest);
      selected = dest;
    } catch (e) {
      log.toast('error', 'Não foi possível mover', messageOf(e));
    }
  }

  function press(e: PointerEvent, rel: string) {
    // A raiz é só destino de soltura: não se arrasta.
    if (rel === '') return;
    selected = rel;
    startDrag(
      e,
      rel,
      (to) => writable && canDrop(rel, to),
      (to) => void moveTo(rel, to),
    );
  }

  // ---------- Renomear, excluir, criar ----------

  function startRename(rel: string) {
    if (!writable) return;
    selected = rel;
    creating = null;
    confirming = null;
    renaming = rel;
  }

  async function commitRename(entry: DirEntry, typed: string) {
    renaming = null;
    const name = typed.trim();
    if (!name || name === entry.name) return;
    const bad = invalidName(name);
    if (bad) {
      log.toast('error', 'Nome inválido', bad);
      return;
    }
    const dest = join(parentOf(entry.rel), name);
    try {
      if (entry.kind === 'script') {
        // O nome do script vai pela Lógica: ela ajusta os nós que o usam. Falha já aparece como aviso.
        if (!(await logic.rename(scriptNameOf(entry.name), scriptNameOf(name)))) return;
        await assets.refresh();
      } else {
        await assets.rename(entry.rel, dest);
      }
      selected = dest;
    } catch (e) {
      log.toast('error', 'Não foi possível renomear', messageOf(e));
    }
  }

  function askTrash(rel: string) {
    if (!writable) return;
    selected = rel;
    renaming = null;
    creating = null;
    confirming = rel;
  }

  async function confirmTrash() {
    const rel = confirming;
    if (!rel) return;
    confirming = null;
    const entry = entryOf(rel);
    if (entry?.kind === 'script') {
      // A Lógica manda o arquivo para a lixeira e tira o script dos nós que o usavam.
      await logic.remove(scriptNameOf(entry.name));
      await assets.refresh();
      if (selected === rel) selected = null;
      return;
    }
    try {
      await assets.trash(rel);
      if (selected === rel) selected = null;
      log.toast('success', 'Movido para a lixeira', baseName(rel), 2400);
    } catch (e) {
      log.toast('error', 'Não foi possível excluir', messageOf(e));
    }
  }

  function startCreate(kind: 'folder' | 'scene') {
    if (!writable) return;
    renaming = null;
    confirming = null;
    selected = null;
    const base = kind === 'folder' ? 'Nova pasta' : 'Nova cena';
    creating = { kind, name: uniqueName(base, new Set(assets.entries.map((e) => editableStem(e)))) };
  }

  async function commitCreate() {
    const c = creating;
    if (!c) return;
    const name = c.name.trim();
    creating = null;
    if (!name) return;
    const bad = invalidName(name);
    if (bad) {
      log.toast('error', 'Nome inválido', bad);
      return;
    }
    const rel = join(assets.dir, name);
    try {
      if (c.kind === 'folder') {
        await assets.createFolder(rel);
        selected = rel;
      } else {
        selected = await assets.createScene(rel, name);
      }
    } catch (e) {
      log.toast('error', c.kind === 'folder' ? 'Não foi possível criar a pasta' : 'Não foi possível criar a cena', messageOf(e));
    }
  }

  // ---------- Importar ----------

  async function importPaths(paths: string[]) {
    if (!writable) {
      log.toast('warn', 'Abra uma pasta do projeto para importar', 'A importação precisa do app desktop e de uma pasta do projeto');
      return;
    }
    await importFilesTo(paths, assets.dir);
  }

  async function importDialog() {
    if (!writable) return;
    try {
      const paths = await pickImportFiles();
      if (paths.length > 0) await importPaths(paths);
    } catch (e) {
      log.toast('error', 'Não foi possível abrir a janela de importação', messageOf(e));
    }
  }

  // ---------- Menus ----------

  function createItems(): MenuItem[] {
    return createMenu({ enabled: writable, folder: () => startCreate('folder'), scene: () => startCreate('scene') });
  }

  /** Botão Criar: o menu abre logo abaixo do botão. */
  function openCreateMenu(btn: HTMLElement) {
    const r = btn.getBoundingClientRect();
    const ev = new MouseEvent('contextmenu', { clientX: r.left, clientY: r.bottom + 2, bubbles: true, cancelable: true });
    showContextMenu(ev, createItems());
  }

  function onEmptyContext(e: MouseEvent) {
    if (onShapes) return;
    showContextMenu(e, [
      ...createItems(),
      'sep',
      { label: 'Importar arquivos…', enabled: writable, run: () => void importDialog() },
      { label: 'Atualizar', enabled: isTauri, run: () => void assets.refresh() },
    ]);
  }

  function onItemContext(e: MouseEvent, entry: DirEntry) {
    selected = entry.rel;
    showContextMenu(
      e,
      itemMenu({
        enabled: writable,
        rename: () => startRename(entry.rel),
        trash: () => askTrash(entry.rel),
        reveal: () => void reveal(entry.rel),
        canReveal: isTauri && !!assets.root,
        open: entry.kind === 'script' ? () => void logic.openIn(scriptNameOf(entry.name)) : undefined,
      }),
    );
  }

  /** Abre o explorador do sistema com o arquivo selecionado. */
  async function reveal(rel: string) {
    if (!assets.root) return;
    // Usa o separador do próprio caminho do projeto (\ no Windows).
    const sep = assets.root.includes('\\') ? '\\' : '/';
    const abs = [assets.root, ...rel.split('/')].join(sep);
    try {
      const { revealItemInDir } = await import('@tauri-apps/plugin-opener');
      await revealItemInDir(abs);
    } catch (e) {
      log.toast('error', 'Não foi possível mostrar no explorador', messageOf(e));
    }
  }

  // ---------- Teclado ----------

  function onKey(e: KeyboardEvent) {
    // Campos de texto (busca, nome em edição) cuidam das próprias teclas.
    if ((e.target as HTMLElement).closest('input, textarea')) return;
    const handled = (fn: () => void) => {
      e.preventDefault();
      e.stopPropagation();
      fn();
    };
    if (e.key === 'F2' && selectedEntry && writable) handled(() => startRename(selectedEntry.rel));
    else if (e.key === 'Delete' && selectedEntry && writable) handled(() => askTrash(selectedEntry.rel));
    else if (e.key === 'Enter' && selectedEntry?.isDir) handled(() => open(selectedEntry.rel));
    else if (e.key === 'Enter' && selectedEntry?.kind === 'script') handled(() => void logic.openIn(scriptNameOf(selectedEntry.name)));
    else if (e.key === 'Escape' && (confirming || selected || creating)) {
      handled(() => {
        confirming = null;
        selected = null;
        creating = null;
      });
    }
  }
</script>

<!-- O painel é o contêiner do teclado (F2, Delete, Enter): por isso recebe foco e as teclas. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<section
  class="project"
  class:focused
  class:drop-hover={dropHover}
  bind:this={rootEl}
  tabindex="0"
  aria-label="Projeto"
  use:focusPanel={'project'}
  onkeydown={onKey}
>
  <ProjectBar
    bind:search
    {crumbs}
    canUp={assets.dir !== ''}
    {writable}
    oncreate={openCreateMenu}
    onimport={() => void importDialog()}
    onup={up}
    onopen={open}
  />

  {#if confirming}
    <div class="confirm">
      <span>Mover “{baseName(confirming)}” para a lixeira?</span>
      <button class="btn" onclick={() => void confirmTrash()}>Mover para a lixeira</button>
      <button class="btn ghost" onclick={() => (confirming = null)}>Cancelar</button>
    </div>
  {/if}

  <div class="body">
    <aside class="tree-col">
      <FolderTree label={projectName} current={assets.dir} onopen={open} oncontext={onItemContext} onpress={press} />
    </aside>

    <div class="content">
      {#if onShapes}
        <ShapeGrid />
      {:else if !assets.root}
        <p class="empty">Nenhum projeto aberto. Abra ou crie um projeto para ver os assets.</p>
      {:else if !isTauri}
        <p class="empty">Os assets ficam no disco. Abra o projeto no app desktop para ver e importar arquivos.</p>
      {:else if !loaded}
        <p class="empty">Carregando…</p>
      {:else}
        <AssetGrid
          entries={shown}
          {tile}
          {list}
          {focused}
          {selected}
          {renaming}
          bind:creating
          {filtering}
          onpress={(e, entry) => press(e, entry.rel)}
          onopen={(entry) => {
            if (entry.isDir) open(entry.rel);
            else if (entry.kind === 'script') void logic.openIn(scriptNameOf(entry.name));
            else selected = entry.rel;
          }}
          oncontext={onItemContext}
          onempty={onEmptyContext}
          onclear={() => (selected = null)}
          oncommit={(entry, typed) => void commitRename(entry, typed)}
          oncancelrename={() => (renaming = null)}
          oncreatecommit={() => void commitCreate()}
          oncreatecancel={() => (creating = null)}
        />
      {/if}
    </div>
  </div>

  <ProjectFooter label={!onShapes && assets.root ? countLabel : ''} bind:size />
</section>

<style>
  .project {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-1);
    outline: none;
  }
  .project.drop-hover {
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .btn {
    display: inline-flex;
    align-items: center;
    height: 20px;
    padding: 0 7px;
    font-size: var(--fs-12);
    color: var(--text-dim);
    border-radius: var(--r-ctl);
  }
  .btn.ghost:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .confirm {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 26px;
    padding: 0 8px;
    font-size: var(--fs-12);
    color: var(--text);
    background: var(--bg-2);
    border-bottom: 1px solid var(--line);
  }
  .confirm .btn {
    color: var(--text);
    background: var(--bg-3);
    border: 1px solid var(--line);
  }
  .confirm .btn.ghost {
    background: none;
    border-color: transparent;
    color: var(--text-dim);
  }
  .confirm span {
    margin-right: auto;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .tree-col {
    flex: none;
    width: 180px;
    overflow: auto;
    border-right: 1px solid var(--line-soft);
  }
  .content {
    flex: 1;
    min-width: 0;
    overflow: auto;
  }
  .empty {
    margin: 0;
    padding: 16px 12px;
    font-size: var(--fs-12);
    line-height: 1.5;
    color: var(--text-faint);
  }
</style>
