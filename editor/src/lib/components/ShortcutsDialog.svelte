<script lang="ts">
  import { onMount } from 'svelte';
  import { COMMANDS, keysOf, ownersOf, type Command } from '$lib/commands';
  import { keymap, shortcutsOpen } from '$lib/state/keymap.svelte';
  import { X, RotateCcw, TriangleAlert } from '@lucide/svelte';

  // Seções na ordem em que aparecem em commands.ts.
  const sections = [...new Set(COMMANDS.map((c) => c.group))].map((group) => ({
    group,
    items: COMMANDS.filter((c) => c.group === group),
  }));

  // Comando que espera uma nova tecla.
  let capturing = $state<string | null>(null);
  // Combinação já usada por outro comando, aguardando confirmação.
  let conflict = $state<{ id: string; keys: string; owners: Command[] } | null>(null);
  let dialog = $state<HTMLDivElement>();
  let returnFocus: HTMLElement | null = null;

  const MODIFIER_KEYS = ['Control', 'Shift', 'Alt', 'Meta'];
  const hasOverride = (id: string) => keymap.overrides[id] !== undefined;

  function close() {
    shortcutsOpen.open = false;
  }

  function startCapture(id: string) {
    conflict = null;
    capturing = id;
  }

  // Monta o texto do atalho no formato de commands.ts: Ctrl, Alt, Shift e a tecla.
  function comboOf(e: KeyboardEvent): string | null {
    if (MODIFIER_KEYS.includes(e.key)) return null;
    let key: string;
    if (/^Digit\d$/.test(e.code)) key = e.code.slice(5);
    else if (e.key === ' ') key = 'Space';
    else if (e.key.length === 1) key = e.key.toUpperCase();
    else key = e.key;
    const parts: string[] = [];
    if (e.ctrlKey || e.metaKey) parts.push('Ctrl');
    if (e.altKey) parts.push('Alt');
    if (e.shiftKey) parts.push('Shift');
    parts.push(key);
    return parts.join('+');
  }

  // Captura em fase de captura: o diálogo é modal, então nenhuma tecla chega aos atalhos do editor.
  function onkey(e: KeyboardEvent) {
    if (e.key === 'Tab') return;
    e.stopPropagation();

    if (capturing) {
      e.preventDefault();
      const bare = !e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey;
      if (e.key === 'Escape' && bare) {
        capturing = null;
        return;
      }
      const keys = comboOf(e);
      if (!keys) return;
      const id = capturing;
      capturing = null;
      const owners = ownersOf(keys, id);
      if (owners.length) conflict = { id, keys, owners };
      else keymap.set(id, keys);
      return;
    }

    if (e.key === 'Escape') {
      e.preventDefault();
      if (conflict) conflict = null;
      else close();
    }
  }

  function replace() {
    if (!conflict) return;
    const { id, keys, owners } = conflict;
    for (const o of owners) keymap.set(o.id, null);
    keymap.set(id, keys);
    conflict = null;
  }

  onMount(() => {
    returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    window.addEventListener('keydown', onkey, true);
    dialog?.focus();
    return () => {
      window.removeEventListener('keydown', onkey, true);
      returnFocus?.focus();
    };
  });
</script>

<div class="scrim" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && close()}>
  <div class="dlg" role="dialog" aria-modal="true" aria-labelledby="atalhos-titulo" tabindex="-1" bind:this={dialog}>
    <header>
      <h2 id="atalhos-titulo">Atalhos de teclado</h2>
      <button class="icon-btn" title="Fechar (Esc)" aria-label="Fechar" onclick={close}><X size={15} /></button>
    </header>
    <p class="hint">Clique num atalho e pressione a nova combinação. Esc cancela.</p>

    <div class="body">
      <table>
        {#each sections as sec (sec.group)}
          <tbody>
            <tr class="band"><th colspan="3" scope="colgroup">{sec.group}</th></tr>
            {#each sec.items as c (c.id)}
              {@const k = keysOf(c)}
              <tr>
                <td class="name">{c.title}</td>
                <td class="key">
                  <button
                    class="cap"
                    class:capturing={capturing === c.id}
                    aria-label="Atalho de {c.title}: {k ?? 'sem atalho'}. Clique para trocar."
                    onclick={() => startCapture(c.id)}
                    onblur={() => capturing === c.id && (capturing = null)}
                  >
                    {#if capturing === c.id}
                      <span class="wait">Pressione uma tecla…</span>
                    {:else if k}
                      {#each k.split('+') as part}<kbd>{part}</kbd>{/each}
                    {:else}
                      <span class="none">Sem atalho</span>
                    {/if}
                  </button>
                </td>
                <td class="act">
                  <button
                    class="icon-btn"
                    title="Remover atalho"
                    aria-label="Remover atalho de {c.title}"
                    disabled={!k}
                    onclick={() => keymap.set(c.id, null)}
                  >
                    <X size={13} />
                  </button>
                  <button
                    class="icon-btn"
                    title="Restaurar atalho padrão"
                    aria-label="Restaurar atalho padrão de {c.title}"
                    disabled={!hasOverride(c.id)}
                    onclick={() => keymap.reset(c.id)}
                  >
                    <RotateCcw size={13} />
                  </button>
                </td>
              </tr>
              {#if conflict && conflict.id === c.id}
                <tr class="conflict">
                  <td colspan="3">
                    <div class="cf" role="alert">
                      <TriangleAlert size={14} />
                      <span>
                        {conflict.keys} já é usado por {conflict.owners.map((o) => '“' + o.title + '”').join(', ')}. Substituir remove o atalho dele.
                      </span>
                      <button class="btn primary" onclick={replace}>Substituir</button>
                      <button class="btn" onclick={() => (conflict = null)}>Cancelar</button>
                    </div>
                  </td>
                </tr>
              {/if}
            {/each}
          </tbody>
        {/each}
      </table>
    </div>

    <footer>
      <button class="btn" onclick={() => keymap.resetAll()} disabled={Object.keys(keymap.overrides).length === 0}>
        <RotateCcw size={13} /> Restaurar padrões
      </button>
      <button class="btn primary" onclick={close}>Fechar</button>
    </footer>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 170;
    display: grid;
    place-items: center;
    background: var(--scrim);
    animation: fade var(--t-med) var(--ease);
  }
  .dlg {
    width: min(720px, calc(100vw - 32px));
    max-height: min(640px, calc(100vh - 48px));
    display: flex;
    flex-direction: column;
    background: var(--bg-2);
    border-radius: var(--r-dlg);
    box-shadow: var(--shadow-pop);
    overflow: hidden;
    outline: none;
    animation: rise var(--t-med) var(--ease);
  }
  header {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 40px;
    padding: 0 8px 0 16px;
    border-bottom: 1px solid var(--line);
  }
  h2 {
    margin: 0;
    font-size: var(--fs-15);
    font-weight: 600;
    color: var(--text);
  }
  .hint {
    flex: none;
    margin: 0;
    padding: 8px 16px;
    font-size: var(--fs-12);
    color: var(--text-dim);
    border-bottom: 1px solid var(--line-soft);
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 8px 8px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--fs-12);
  }
  .band th {
    position: sticky;
    top: 0;
    z-index: 1;
    height: 22px;
    padding: 0 8px;
    text-align: left;
    font-weight: 600;
    color: var(--text);
    background: var(--bg-toolbar);
  }
  td {
    height: 28px;
    padding: 0 4px;
    border-bottom: 1px solid var(--line-soft);
    vertical-align: middle;
  }
  .name {
    padding-left: 8px;
    color: var(--text);
  }
  .key {
    width: 220px;
  }
  .act {
    width: 60px;
    white-space: nowrap;
    text-align: right;
  }
  .cap {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    min-width: 180px;
    height: 22px;
    padding: 0 6px;
    border-radius: var(--r-ctl);
    border: 1px solid var(--line);
    background: var(--bg-0);
    color: var(--text);
    transition: border-color var(--t-fast) var(--ease), background var(--t-fast) var(--ease);
  }
  .cap:hover {
    border-color: var(--text-faint);
  }
  .cap.capturing {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .cap kbd {
    color: var(--text);
  }
  .wait {
    color: var(--text);
  }
  .none {
    color: var(--text-faint);
  }
  .act .icon-btn:disabled {
    opacity: 0.45;
    pointer-events: none;
  }
  .conflict td {
    height: auto;
    padding: 6px 8px;
    background: var(--bg-toolbar);
  }
  .cf {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    color: var(--text);
  }
  .cf :global(svg) {
    flex: none;
    color: var(--warning);
  }
  .cf > span {
    flex: 1;
    min-width: 200px;
  }
  footer {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 12px;
    border-top: 1px solid var(--line);
  }
  footer .btn {
    gap: 6px;
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(-8px) scale(0.985);
    }
  }
</style>
