<script lang="ts">
  import { log, type LogEntry, type LogLevel } from '$lib/state/log.svelte';
  import { scene } from '$lib/state/scene.svelte';
  import { focus, focusPanel } from '$lib/state/focus.svelte';
  import { Trash, ListCollapse, Play, CirclePause, Search, Info, CircleCheck, TriangleAlert, CircleAlert } from '@lucide/svelte';

  // Filtros por tipo. "Log" reúne Log e Sucesso, como antes.
  type Group = 'log' | 'warn' | 'error';
  const GROUP: Record<LogLevel, Group> = { info: 'log', success: 'log', warn: 'warn', error: 'error' };
  const LEVEL_NAME: Record<LogLevel, string> = { info: 'Log', success: 'Sucesso', warn: 'Aviso', error: 'Erro' };
  const ICON = { info: Info, success: CircleCheck, warn: TriangleAlert, error: CircleAlert };

  let show = $state<Record<Group, boolean>>({ log: true, warn: true, error: true });
  let collapse = $state(false);
  let query = $state('');
  // Chave da linha selecionada: id da primeira mensagem do grupo.
  let selected = $state<number | null>(null);
  let list = $state<HTMLDivElement>();
  // Enquanto o usuário não sobe a lista, novas mensagens levam a barra para o fim.
  let stick = true;

  interface Row {
    key: number;
    entry: LogEntry;
    count: number;
  }

  const norm = (s: string) => s.normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLowerCase();

  const counts = $derived.by(() => {
    const c: Record<Group, number> = { log: 0, warn: 0, error: 0 };
    for (const e of log.entries) c[GROUP[e.level]]++;
    return c;
  });

  // Linhas exibidas: filtros por tipo e busca. Com "Colapsar", mensagens iguais seguidas viram uma linha.
  const rows = $derived.by(() => {
    const q = norm(query.trim());
    const out: Row[] = [];
    for (const e of log.entries) {
      if (!show[GROUP[e.level]]) continue;
      if (q && !norm(e.text).includes(q)) continue;
      const prev = out.at(-1);
      if (collapse && prev && prev.entry.level === e.level && prev.entry.text === e.text && prev.entry.nodeUid === e.nodeUid) {
        prev.count++;
        prev.entry = e;
      } else {
        out.push({ key: e.id, entry: e, count: 1 });
      }
    }
    return out;
  });

  const selectedRow = $derived(rows.find((r) => r.key === selected) ?? null);
  const newest = $derived(rows.at(-1)?.entry.id ?? null);

  const time = (d: Date) => d.toLocaleTimeString('pt-BR', { hour12: false });
  const fullTime = (d: Date) => d.toLocaleString('pt-BR', { hour12: false });
  const objectName = (uid: string) => scene.node(uid)?.name ?? null;

  // Duplo clique (ou Enter) numa mensagem com origem: seleciona o objeto na cena.
  function jump(e: LogEntry) {
    if (e.nodeUid && objectName(e.nodeUid) !== null) scene.select(e.nodeUid);
  }

  function onscroll() {
    if (!list) return;
    stick = list.scrollHeight - list.scrollTop - list.clientHeight < 24;
  }

  $effect(() => {
    if (newest === null || !stick || !list) return;
    list.scrollTop = list.scrollHeight;
  });

  $effect(() => {
    if (selected === null || !list) return;
    list.querySelector(`[data-key="${selected}"]`)?.scrollIntoView({ block: 'nearest' });
  });

  // Clique e duplo clique são tratados na lista (delegação): cada linha tem data-key.
  function rowAt(e: Event): Row | undefined {
    const el = (e.target as HTMLElement).closest<HTMLElement>('[data-key]');
    if (!el) return undefined;
    return rows.find((r) => r.key === Number(el.dataset.key));
  }
  function onclick(e: MouseEvent) {
    const r = rowAt(e);
    if (r) selected = r.key;
  }
  function ondblclick(e: MouseEvent) {
    const r = rowAt(e);
    if (r) jump(r.entry);
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      if (selectedRow) jump(selectedRow.entry);
      return;
    }
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp' && e.key !== 'Home' && e.key !== 'End') return;
    e.preventDefault();
    if (!rows.length) return;
    const i = rows.findIndex((r) => r.key === selected);
    let j: number;
    if (e.key === 'Home') j = 0;
    else if (e.key === 'End') j = rows.length - 1;
    else j = Math.max(0, Math.min(rows.length - 1, i + (e.key === 'ArrowDown' ? 1 : -1)));
    selected = rows[j].key;
  }

  function clear() {
    log.clear();
    selected = null;
  }
</script>

<div class="console" use:focusPanel={'console'}>
  <div class="bar">
    <button class="tool" title="Limpar console" aria-label="Limpar console" onclick={clear}>
      <Trash size={13} />
    </button>
    <span class="sep"></span>
    <button
      class="tool"
      class:on={collapse}
      aria-pressed={collapse}
      title="Agrupar mensagens iguais seguidas"
      onclick={() => (collapse = !collapse)}
    >
      <ListCollapse size={13} /><span class="tlbl">Colapsar</span>
    </button>
    <button
      class="tool"
      class:on={log.clearOnPlay}
      aria-pressed={log.clearOnPlay}
      title="Limpar o console ao iniciar o jogo"
      onclick={() => (log.clearOnPlay = !log.clearOnPlay)}
    >
      <Play size={12} /><span class="tlbl">Limpar ao jogar</span>
    </button>
    <button
      class="tool"
      class:on={log.errorPause}
      aria-pressed={log.errorPause}
      title="Pausar o jogo quando ocorrer um erro"
      onclick={() => (log.errorPause = !log.errorPause)}
    >
      <CirclePause size={13} /><span class="tlbl">Pausar no erro</span>
    </button>

    <label class="search">
      <Search size={12} />
      <input type="search" bind:value={query} placeholder="Buscar mensagens" aria-label="Buscar mensagens" spellcheck="false" />
    </label>

    <div class="filters">
      <button class="flt info" class:on={show.log} aria-pressed={show.log} onclick={() => (show.log = !show.log)}>
        <Info size={12} />Log <em>{counts.log}</em>
      </button>
      <button class="flt warn" class:on={show.warn} aria-pressed={show.warn} onclick={() => (show.warn = !show.warn)}>
        <TriangleAlert size={12} />Aviso <em>{counts.warn}</em>
      </button>
      <button class="flt error" class:on={show.error} aria-pressed={show.error} onclick={() => (show.error = !show.error)}>
        <CircleAlert size={12} />Erro <em>{counts.error}</em>
      </button>
    </div>
  </div>

  <div
    class="list mono"
    class:focused={focus.panel === 'console'}
    role="listbox"
    aria-label="Mensagens"
    tabindex="0"
    bind:this={list}
    onscroll={onscroll}
    onkeydown={onkey}
    {onclick}
    {ondblclick}
  >
    {#each rows as r (r.key)}
      {@const Icon = ICON[r.entry.level]}
      <div
        class="row {r.entry.level}"
        class:sel={selected === r.key}
        role="option"
        tabindex="-1"
        aria-selected={selected === r.key}
        data-key={r.key}
        title={r.entry.nodeUid ? 'Clique duplo para selecionar o objeto' : undefined}
      >
        <span class="t">{time(r.entry.time)}</span>
        <Icon size={12} />
        <span class="msg">{r.entry.text}</span>
        {#if r.count > 1}<span class="badge">{r.count}</span>{/if}
      </div>
    {:else}
      <div class="empty">{log.entries.length ? 'Nenhuma mensagem com este filtro.' : 'Nenhuma mensagem.'}</div>
    {/each}
  </div>

  <div class="detail">
    {#if selectedRow}
      {@const e = selectedRow.entry}
      {@const name = e.nodeUid ? objectName(e.nodeUid) : null}
      <div class="meta">
        <span class="lvl {e.level}">{LEVEL_NAME[e.level]}</span>
        <span>{fullTime(e.time)}</span>
        {#if selectedRow.count > 1}<span>Repetida {selectedRow.count} vezes</span>{/if}
        {#if e.nodeUid}
          <span>
            Objeto: {name ?? 'não encontrado na cena'}{#if name} · clique duplo na mensagem para selecionar{/if}
          </span>
        {/if}
      </div>
      <pre class="full">{e.text}</pre>
    {:else}
      <p class="none">Selecione uma mensagem para ver os detalhes.</p>
    {/if}
  </div>
</div>

<style>
  .console {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    container-type: inline-size;
  }

  /* Barra de ferramentas */
  .bar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
    height: 28px;
    padding: 0 6px;
    background: var(--bg-toolbar);
    border-bottom: 1px solid var(--line);
  }
  .sep {
    flex: none;
    width: 1px;
    height: 14px;
    margin: 0 3px;
    background: var(--line);
  }
  .tool,
  .flt {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    padding: 0 7px;
    border: 1px solid transparent;
    border-radius: var(--r-ctl);
    font-size: var(--fs-12);
    white-space: nowrap;
    color: var(--text-dim);
    transition: background var(--t-fast) var(--ease), color var(--t-fast) var(--ease);
  }
  .tool:hover,
  .flt:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .tool.on {
    color: var(--text);
    background: var(--accent-soft);
    border-color: var(--accent);
  }
  .tool.on :global(svg) {
    color: var(--accent-hi);
  }

  .search {
    flex: 1 1 120px;
    min-width: 80px;
    max-width: 260px;
    height: 24px;
    display: flex;
    align-items: center;
    gap: 5px;
    margin-left: 6px;
    padding: 0 6px;
    background: var(--bg-2);
    border: 1px solid var(--line);
    border-radius: var(--r-ctl);
    color: var(--text-faint);
    transition: border-color var(--t-fast) var(--ease);
  }
  .search:hover {
    border-color: var(--text-faint);
  }
  .search:focus-within {
    border-color: var(--accent);
  }
  .search input {
    flex: 1;
    min-width: 0;
    height: 100%;
    background: none;
    border: none;
    outline: none;
    font-size: var(--fs-12);
    color: var(--text);
  }
  .search input::placeholder {
    color: var(--text-faint);
  }

  .filters {
    flex: none;
    display: flex;
    gap: 3px;
    margin-left: auto;
  }
  .flt {
    border-color: var(--line-soft);
  }
  .flt em {
    font-style: normal;
    font-variant-numeric: tabular-nums;
    color: var(--text-dim);
  }
  .flt :global(svg) {
    opacity: 0.5;
  }
  .flt.on {
    color: var(--text);
    background: var(--bg-2);
    border-color: var(--line);
  }
  .flt.on :global(svg) {
    opacity: 1;
  }
  .flt.info.on :global(svg) {
    color: var(--info);
  }
  .flt.warn.on :global(svg) {
    color: var(--warning);
  }
  .flt.error.on :global(svg) {
    color: var(--danger);
  }

  @container (max-width: 560px) {
    .tlbl {
      display: none;
    }
  }

  /* Lista de mensagens */
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 2px 0;
    font-size: var(--fs-11);
    background: var(--bg-1);
    outline: none;
  }
  .list:focus-visible {
    outline: 2px solid var(--accent-ring);
    outline-offset: -2px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 20px;
    padding: 0 8px 0 8px;
    user-select: none;
    color: var(--text);
  }
  .row:hover {
    background: var(--bg-hover);
  }
  .row:focus {
    outline: none;
  }
  .row :global(svg) {
    flex: none;
  }
  .t {
    flex: none;
    color: var(--text-faint);
    font-variant-numeric: tabular-nums;
  }
  .msg {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .badge {
    flex: none;
    min-width: 18px;
    height: 14px;
    padding: 0 4px;
    border-radius: var(--r-ctl);
    background: var(--accent-soft);
    color: var(--text);
    font-size: var(--fs-11);
    font-weight: 600;
    line-height: 14px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }
  .row.info :global(svg) {
    color: var(--info);
  }
  .row.success :global(svg) {
    color: var(--success);
  }
  .row.warn :global(svg) {
    color: var(--warning);
  }
  .row.warn .msg {
    color: var(--warning);
  }
  .row.error :global(svg) {
    color: var(--danger);
  }
  .row.error .msg {
    color: var(--danger);
  }
  .row.error {
    background: color-mix(in srgb, var(--danger) 8%, transparent);
  }
  .row.error:hover {
    background: color-mix(in srgb, var(--danger) 14%, transparent);
  }
  /* Seleção: cheia quando a lista tem o foco, apagada quando não tem (EDT-020) */
  .row.sel {
    background: var(--selection-unfocused);
  }
  .row.sel .msg,
  .row.sel .t {
    color: var(--text);
  }
  .list.focused .row.sel {
    background: var(--selection);
  }
  .list.focused .row.sel .msg,
  .list.focused .row.sel .t {
    color: var(--sel-text);
  }
  .list.focused .row.sel :global(svg) {
    color: var(--sel-text);
  }
  .empty {
    padding: 12px;
    color: var(--text-faint);
    font-family: var(--font-ui);
  }

  /* Detalhe da mensagem selecionada */
  .detail {
    flex: none;
    height: 76px;
    overflow-y: auto;
    padding: 6px 10px;
    border-top: 1px solid var(--line);
    background: var(--bg-1);
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px 12px;
    margin-bottom: 4px;
    font-size: var(--fs-11);
    color: var(--text-dim);
  }
  .lvl {
    font-weight: 600;
    color: var(--text);
  }
  .lvl.info {
    color: var(--info);
  }
  .lvl.success {
    color: var(--success);
  }
  .lvl.warn {
    color: var(--warning);
  }
  .lvl.error {
    color: var(--danger);
  }
  .full {
    margin: 0;
    font-family: var(--font-mono);
    font-size: var(--fs-11);
    line-height: 1.45;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    color: var(--text);
    user-select: text;
  }
  .none {
    margin: 0;
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
</style>
