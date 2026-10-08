<script lang="ts">
  import { onMount } from 'svelte';
  import {
    settingsUi,
    settingsStatus,
    cancelCapture,
    closeProjectSettings,
    finishCapture,
    type SettingsTab,
  } from '$lib/state/settings.svelte';
  import { watchGamepad } from '$lib/runtime/devices';
  import GeneralTab from './GeneralTab.svelte';
  import InputTab from './InputTab.svelte';
  import PhysicsTab from './PhysicsTab.svelte';
  import AudioTab from './AudioTab.svelte';
  import TagsTab from './TagsTab.svelte';
  import { X, TriangleAlert } from '@lucide/svelte';

  const TABS: { id: SettingsTab; label: string }[] = [
    { id: 'geral', label: 'Geral' },
    { id: 'entrada', label: 'Entrada' },
    { id: 'fisica', label: 'Física 2D' },
    { id: 'audio', label: 'Áudio' },
    { id: 'camadas', label: 'Camadas e Tags' },
  ];

  let dialog = $state<HTMLDivElement>();
  let returnFocus: HTMLElement | null = null;

  // Modal: em fase de captura, nenhuma tecla chega aos atalhos do editor. Quando há uma captura
  // pendente (Entrada), a próxima tecla vai para a ação; Esc cancela. Tab segue para a navegação.
  // Tecla que acabou de ser capturada: o keyup dela não deve acionar um botão com foco.
  let swallowUp: string | null = null;

  function onkey(e: KeyboardEvent) {
    if (e.key === 'Tab') return;
    e.stopPropagation();
    if (e.type === 'keyup') {
      if (swallowUp !== null && e.code === swallowUp) {
        e.preventDefault();
        swallowUp = null;
      }
      return;
    }
    if (settingsUi.capture) {
      e.preventDefault();
      if (e.key === 'Escape') {
        cancelCapture();
      } else if (e.code) {
        swallowUp = e.code;
        finishCapture(e.code);
      }
      return;
    }
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
    }
  }

  function close() {
    closeProjectSettings();
  }

  onMount(() => {
    returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    window.addEventListener('keydown', onkey, true);
    window.addEventListener('keyup', onkey, true);
    dialog?.focus();
    return () => {
      window.removeEventListener('keydown', onkey, true);
      window.removeEventListener('keyup', onkey, true);
      returnFocus?.focus();
    };
  });

  // Gamepad capture (INP-001): while a capture is pending, wait for a button, trigger or stick. The keyboard
  // path is still `onkey`. Whichever arrives first wins, and finishCapture clears the capture, which stops the wait.
  $effect(() => {
    if (!settingsUi.capture) return;
    return watchGamepad((code) => finishCapture(code));
  });
</script>

<div class="scrim" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && close()}>
  <div class="dlg" role="dialog" aria-modal="true" aria-labelledby="ps-titulo" tabindex="-1" bind:this={dialog}>
    <header>
      <h2 id="ps-titulo">Configurações do projeto</h2>
      <button class="icon-btn" title="Fechar (Esc)" aria-label="Fechar" onclick={close}><X size={15} /></button>
    </header>

    {#if settingsStatus.error}
      <div class="alert" role="alert">
        <TriangleAlert size={14} />
        <span>
          Não foi possível ler as configurações: {settingsStatus.error}. Os padrões estão em uso e nada é gravado
          até o arquivo ser corrigido ou apagado.
        </span>
      </div>
    {/if}

    <div class="split">
      <div class="tabs" role="tablist" aria-orientation="vertical" aria-label="Seções">
        {#each TABS as t (t.id)}
          <button
            role="tab"
            id="ps-tab-{t.id}"
            aria-selected={settingsUi.tab === t.id}
            aria-controls="ps-panel"
            class:on={settingsUi.tab === t.id}
            onclick={() => (settingsUi.tab = t.id)}
          >
            {t.label}
          </button>
        {/each}
      </div>

      <div class="ps-body" role="tabpanel" id="ps-panel" aria-labelledby="ps-tab-{settingsUi.tab}">
        {#if settingsUi.tab === 'geral'}
          <GeneralTab />
        {:else if settingsUi.tab === 'entrada'}
          <InputTab />
        {:else if settingsUi.tab === 'fisica'}
          <PhysicsTab />
        {:else if settingsUi.tab === 'audio'}
          <AudioTab />
        {:else}
          <TagsTab />
        {/if}
      </div>
    </div>

    <footer>
      <span class="saved">Alterações são gravadas automaticamente.</span>
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
    width: min(860px, calc(100vw - 32px));
    height: min(640px, calc(100vh - 48px));
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
  .alert {
    flex: none;
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 16px;
    font-size: var(--fs-12);
    color: var(--text);
    background: var(--bg-toolbar);
    border-bottom: 1px solid var(--line-soft);
  }
  .alert :global(svg) {
    flex: none;
    margin-top: 2px;
    color: var(--warning);
  }
  .split {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .tabs {
    flex: none;
    width: 160px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px;
    border-right: 1px solid var(--line-soft);
    background: var(--bg-1);
  }
  .tabs button {
    height: 26px;
    padding: 0 10px;
    border-radius: var(--r-ctl);
    text-align: left;
    font-size: var(--fs-12);
    color: var(--text-dim);
  }
  .tabs button:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .tabs button.on {
    background: var(--selection);
    color: var(--sel-text);
  }
  .ps-body {
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: auto;
    padding: 12px 16px;
  }
  footer {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    height: 44px;
    padding: 0 12px 0 16px;
    border-top: 1px solid var(--line);
  }
  .saved {
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .ps-body :global(h3) {
    margin: 14px 0 6px;
    font-size: var(--fs-12);
    font-weight: 600;
    color: var(--text);
  }
  .ps-body :global(h3:first-child) {
    margin-top: 0;
  }
  .ps-body :global(.hint) {
    margin: 0 0 10px;
    font-size: var(--fs-11);
    line-height: 1.45;
    color: var(--text-dim);
  }
  .ps-body :global(.mono) {
    font-family: var(--font-mono);
  }
  .ps-body :global(.txt),
  .ps-body :global(.num) {
    min-width: 0;
    height: 22px;
    padding: 0 6px;
    border-radius: var(--r-ctl);
    background: var(--bg-0);
    color: var(--text);
    font-size: var(--fs-12);
    box-shadow: inset 0 0 0 1px var(--line);
    outline: none;
  }
  .ps-body :global(.txt:focus) {
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .ps-body :global(.err) {
    margin: 4px 0 0;
    font-size: var(--fs-11);
    color: var(--danger);
  }
</style>
