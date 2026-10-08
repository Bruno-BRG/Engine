<script lang="ts">
  import { onMount } from 'svelte';
  import { app } from '$lib/state/app.svelte';
  import { platform, isTauri } from '$lib/platform';
  import * as act from '$lib/actions';
  import Logo from './Logo.svelte';
  import MenuBar from './MenuBar.svelte';
  import { Play, Square, Pause, SkipForward, Search, Minus, Copy, X, LayoutGrid } from '@lucide/svelte';

  let maximized = $state(false);

  onMount(() => {
    let unlisten: (() => void) | undefined;
    platform.window().then(async (w) => {
      if (!w) return;
      maximized = await w.isMaximized();
      unlisten = await w.onResized(async () => (maximized = await w.isMaximized()));
    });
    return () => unlisten?.();
  });

  const win = async (fn: 'minimize' | 'toggleMaximize' | 'close') => (await platform.window())?.[fn]();
</script>

<header class="bar" data-tauri-drag-region>
  <div class="left" data-tauri-drag-region>
    <span class="brand"><Logo size={16} /></span>
    {#if app.project}
      <MenuBar />
      <span class="proj" title={app.project.path}>{app.project.name}</span>
    {:else}
      <span class="title" data-tauri-drag-region>Nexo</span>
    {/if}
  </div>

  <div class="transport" data-tauri-drag-region>
    {#if app.project}
      <button class="ctl play" class:playing={app.playing} onclick={act.togglePlay} title={app.playing ? 'Parar (F5)' : 'Jogar (F5)'}>
        {#if app.playing}
          <Square size={11} fill="currentColor" /> Parar
        {:else}
          <Play size={12} fill="currentColor" /> Jogar
        {/if}
      </button>
      <button
        class="ctl icon"
        class:on={app.paused}
        onclick={act.togglePause}
        disabled={!app.playing}
        title={app.paused ? 'Retomar (F6)' : 'Pausar (F6)'}
        aria-label={app.paused ? 'Retomar' : 'Pausar'}
        aria-pressed={app.paused}
      >
        <Pause size={12} fill="currentColor" />
      </button>
      <button class="ctl icon" onclick={act.stepFrame} title="Passo a passo (F10)" aria-label="Passo a passo">
        <SkipForward size={12} fill="currentColor" />
      </button>
    {/if}
  </div>

  <div class="right" data-tauri-drag-region>
    {#if app.project}
      <label class="layout" title="Layout dos painéis (EDT-011)">
        <LayoutGrid size={12} />
        <select value={app.layout} onchange={(e) => act.applyLayout(e.currentTarget.value as act.LayoutId)} aria-label="Layout">
          {#each Object.entries(act.LAYOUTS) as [id, l] (id)}
            <option value={id}>{l.label}</option>
          {/each}
        </select>
      </label>
      <button class="search" onclick={() => (app.paletteOpen = true)}>
        <Search size={12} />
        <span>Buscar comandos</span>
        <kbd>Ctrl</kbd><kbd>K</kbd>
      </button>
    {/if}
    {#if isTauri}
      <div class="winctl">
        <button onclick={() => win('minimize')} aria-label="Minimizar"><Minus size={15} /></button>
        <button onclick={() => win('toggleMaximize')} aria-label="Maximizar">
          {#if maximized}<Copy size={12} />{:else}<Square size={12} />{/if}
        </button>
        <button class="close" onclick={() => win('close')} aria-label="Fechar"><X size={16} /></button>
      </div>
    {/if}
  </div>
</header>

<style>
  .bar {
    position: relative;
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    height: 30px;
    padding-left: 6px;
    background: var(--bg-toolbar);
    border-bottom: 1px solid var(--line);
    z-index: 50;
  }
  .left,
  .right,
  .transport {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 100%;
    min-width: 0;
  }
  .left {
    justify-content: flex-start;
  }
  .right {
    justify-content: flex-end;
    padding-right: 0;
  }
  .transport {
    justify-content: center;
    gap: 2px;
  }
  .brand {
    display: grid;
    place-items: center;
    width: 22px;
    flex: none;
  }
  .title {
    font-weight: 400;
  }
  .proj {
    min-width: 0;
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-dim);
    padding-left: 6px;
    border-left: 1px solid var(--line);
    margin-left: 2px;
  }

  /* Controles de transporte: cinza como no Unity; azul quando o jogo roda */
  .ctl {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 22px;
    padding: 0 10px;
    border-radius: var(--r-ctl);
    font-size: var(--fs-12);
    color: var(--text);
    background: var(--bg-3);
    border: 1px solid var(--line);
    transition: background var(--t-fast) var(--ease), color var(--t-fast) var(--ease);
  }
  .ctl.icon {
    width: 26px;
    padding: 0;
  }
  .ctl:hover:not(:disabled) {
    background: var(--bg-active);
  }
  .ctl:disabled {
    opacity: 0.45;
    pointer-events: none;
  }
  .ctl.play.playing,
  .ctl.icon.on {
    color: var(--sel-text);
    background: var(--selection);
    border-color: var(--selection);
  }
  .layout {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 22px;
    padding: 0 4px 0 6px;
    margin-right: 6px;
    border-radius: var(--r-ctl);
    color: var(--text-faint);
    background: var(--bg-2);
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: var(--fs-11);
  }
  .layout select {
    height: 20px;
    background: none;
    color: var(--text-dim);
    font-size: var(--fs-11);
    outline: none;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    width: 220px;
    padding: 0 4px 0 8px;
    border-radius: var(--r-ctl);
    color: var(--text-faint);
    background: var(--bg-2);
    border: 1px solid var(--line);
    font-size: var(--fs-12);
    transition: border-color var(--t-fast) var(--ease), color var(--t-fast) var(--ease);
  }
  .search span {
    flex: 1;
    text-align: left;
  }
  .search:hover {
    border-color: var(--text-faint);
    color: var(--text-dim);
  }

  .winctl {
    display: flex;
    height: 100%;
    margin-left: 4px;
  }
  .winctl button {
    width: 46px;
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--text-dim);
    transition: background var(--t-fast), color var(--t-fast);
  }
  .winctl button:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .winctl .close:hover {
    background: var(--danger);
    color: white;
  }
  @media (max-width: 1180px) {
    .search span {
      display: none;
    }
    .search {
      width: auto;
    }
    .proj {
      display: none;
    }
  }
</style>
