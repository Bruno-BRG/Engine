<script lang="ts">
  import { tick } from 'svelte';
  import { contextMenu, closeContextMenu, type MenuItem } from '$lib/state/contextmenu.svelte';
  import { Check } from '@lucide/svelte';

  // Distância mínima da borda da janela.
  const EDGE = 4;

  let menuEl = $state<HTMLDivElement>();
  let pos = $state({ left: 0, top: 0 });
  let active = $state(-1);
  // Elemento que tinha o foco antes do menu. Volta a ter o foco ao fechar pelo teclado.
  let returnFocus: HTMLElement | null = null;

  const enabled = (it: MenuItem) => it !== 'sep' && it.enabled !== false;

  // Ao abrir: posiciona dentro da janela, foca o menu e ativa o primeiro item habilitado.
  $effect(() => {
    const m = contextMenu.open;
    if (!m) return;
    returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    active = m.items.findIndex(enabled);
    pos = { left: m.x, top: m.y };
    tick().then(() => {
      if (contextMenu.open !== m || !menuEl) return;
      const w = menuEl.offsetWidth;
      const h = menuEl.offsetHeight;
      pos = {
        left: Math.max(EDGE, Math.min(m.x, window.innerWidth - w - EDGE)),
        top: Math.max(EDGE, Math.min(m.y, window.innerHeight - h - EDGE)),
      };
      menuEl.focus();
    });
  });

  // Fecha ao clicar fora, ao rolar (fora do próprio menu) ou ao a janela perder o foco.
  $effect(() => {
    if (!contextMenu.open) return;
    const outside = (e: Event) => {
      if (e.target instanceof Node && menuEl?.contains(e.target)) return;
      closeContextMenu();
    };
    const blur = () => closeContextMenu();
    window.addEventListener('pointerdown', outside, true);
    window.addEventListener('scroll', outside, true);
    window.addEventListener('blur', blur);
    return () => {
      window.removeEventListener('pointerdown', outside, true);
      window.removeEventListener('scroll', outside, true);
      window.removeEventListener('blur', blur);
    };
  });

  function restoreFocus() {
    returnFocus?.focus();
  }

  function close() {
    closeContextMenu();
    restoreFocus();
  }

  function pick(it: Extract<MenuItem, object>) {
    if (it.enabled === false) return;
    closeContextMenu();
    restoreFocus();
    it.run();
  }

  // Move a seleção entre os itens habilitados, dando a volta no fim da lista.
  function step(dir: 1 | -1) {
    const items = contextMenu.open?.items ?? [];
    const n = items.length;
    if (!n) return;
    let i = active < 0 ? (dir === 1 ? n - 1 : 0) : active;
    for (let k = 0; k < n; k++) {
      i = (i + dir + n) % n;
      if (enabled(items[i])) {
        active = i;
        return;
      }
    }
  }

  function onkey(e: KeyboardEvent) {
    const m = contextMenu.open;
    if (!m) return;
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      e.stopPropagation();
      step(e.key === 'ArrowDown' ? 1 : -1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      e.stopPropagation();
      const it = m.items[active];
      if (it && it !== 'sep') pick(it);
    } else if (e.key === 'Escape') {
      // Não chega aos atalhos globais: Escape fecha só o menu.
      e.preventDefault();
      e.stopPropagation();
      close();
    } else if (e.key === 'Tab') {
      close();
    }
  }
</script>

{#if contextMenu.open}
  {@const m = contextMenu.open}
  <div
    bind:this={menuEl}
    class="ctx"
    role="menu"
    tabindex="-1"
    aria-label="Menu de contexto"
    style:left="{pos.left}px"
    style:top="{pos.top}px"
    onkeydown={onkey}
  >
    {#each m.items as it, i (i)}
      {#if it === 'sep'}
        <div class="sep" role="separator"></div>
      {:else}
        <button
          type="button"
          role={it.checked !== undefined ? 'menuitemcheckbox' : 'menuitem'}
          aria-checked={it.checked !== undefined ? it.checked : undefined}
          aria-disabled={it.enabled === false}
          class:active={i === active}
          disabled={it.enabled === false}
          tabindex="-1"
          onpointerenter={() => enabled(it) && (active = i)}
          onclick={() => pick(it)}
        >
          <span class="mark">{#if it.checked}<Check size={12} />{/if}</span>
          <span class="label">{it.label}</span>
          {#if it.keys}<span class="keys">{it.keys}</span>{/if}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .ctx {
    position: fixed;
    z-index: 160;
    min-width: 180px;
    max-width: 320px;
    max-height: calc(100vh - 8px);
    overflow-y: auto;
    padding: 3px;
    background: var(--bg-2);
    border-radius: var(--r-dlg);
    box-shadow: var(--shadow-pop);
    outline: none;
    animation: pop var(--t-fast) var(--ease);
  }
  button {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    height: 24px;
    padding: 0 8px 0 4px;
    border-radius: var(--r-ctl);
    color: var(--text);
    text-align: left;
    font-size: var(--fs-12);
  }
  button.active:not(:disabled) {
    background: var(--selection);
    color: var(--sel-text);
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  button.active .keys {
    color: var(--sel-text);
  }
  .mark {
    flex: none;
    width: 14px;
    display: grid;
    place-items: center;
  }
  .label {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .keys {
    flex: none;
    margin-left: 16px;
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .sep {
    height: 1px;
    margin: 3px 4px;
    background: var(--line);
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-3px) scale(0.98);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .ctx {
      animation: none;
    }
  }
</style>
