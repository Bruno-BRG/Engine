<script lang="ts">
  import { onMount } from 'svelte';
  import { app, type Workspace } from '$lib/state/app.svelte';
  import { platform, isTauri } from '$lib/platform';
  import * as act from '$lib/actions';
  import Logo from './Logo.svelte';
  import { Box, Workflow, Film, Brush, Play, Square, Search, ChevronDown, Minus, Copy, X, Save, FolderOpen, LogOut } from '@lucide/svelte';

  const workspaces: { id: Workspace; label: string; icon: typeof Box }[] = [
    { id: 'cena', label: 'Cena', icon: Box },
    { id: 'logica', label: 'Lógica', icon: Workflow },
    { id: 'animacao', label: 'Animação', icon: Film },
    { id: 'sprite', label: 'Sprite', icon: Brush },
  ];
  const wsIndex = $derived(workspaces.findIndex((w) => w.id === app.workspace));

  let maximized = $state(false);
  let menuOpen = $state(false);

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

  async function revealProject() {
    menuOpen = false;
    if (!app.project || !isTauri) return;
    const { openPath } = await import('@tauri-apps/plugin-opener');
    openPath(app.project.path);
  }
</script>

<header class="bar" data-tauri-drag-region>
  <div class="left" data-tauri-drag-region>
    <span class="brand"><Logo size={18} /></span>
    {#if app.project}
      <div class="menu-wrap">
        <button class="project" onclick={() => (menuOpen = !menuOpen)} class:open={menuOpen}>
          <span class="name">{app.project.name}</span>
          <ChevronDown size={14} />
        </button>
        {#if menuOpen}
          <button class="scrim" aria-label="Fechar menu" onclick={() => (menuOpen = false)}></button>
          <div class="menu">
            <button onclick={() => { menuOpen = false; act.save(); }}><Save size={14} /> Salvar cena <kbd>Ctrl S</kbd></button>
            {#if isTauri}
              <button onclick={revealProject}><FolderOpen size={14} /> Abrir pasta do projeto</button>
            {/if}
            <div class="sep"></div>
            <button onclick={() => { menuOpen = false; act.closeProject(); }}><LogOut size={14} /> Fechar projeto</button>
          </div>
        {/if}
      </div>
    {:else}
      <span class="title" data-tauri-drag-region>Nexo</span>
    {/if}
  </div>

  {#if app.project}
    <nav class="workspaces" style="--i: {wsIndex}">
      <span class="indicator"></span>
      {#each workspaces as w, i (w.id)}
        {@const Icon = w.icon}
        <button class:active={app.workspace === w.id} onclick={() => act.setWorkspace(w.id)} title="{w.label} (Alt+{i + 1})">
          <Icon size={14} />
          {w.label}
        </button>
      {/each}
    </nav>
  {/if}

  <div class="right" data-tauri-drag-region>
    {#if app.project}
      <button class="play" class:playing={app.playing} onclick={act.togglePlay} title={app.playing ? 'Parar (F5)' : 'Jogar (F5)'}>
        {#if app.playing}
          <Square size={12} fill="currentColor" /> Parar
        {:else}
          <Play size={13} fill="currentColor" /> Jogar
        {/if}
      </button>
      <button class="search" onclick={() => (app.paletteOpen = true)}>
        <Search size={13} />
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
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 40px;
    padding-left: 10px;
    background: var(--bg-1);
    border-bottom: 1px solid var(--line-soft);
    z-index: 50;
  }
  .left,
  .right {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 100%;
  }
  .brand {
    display: grid;
    place-items: center;
    width: 26px;
    filter: drop-shadow(0 2px 6px rgb(124 92 255 / 0.4));
  }
  .title {
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .menu-wrap {
    position: relative;
  }
  .project {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 8px 0 10px;
    border-radius: var(--r-ctl);
    font-weight: 600;
    color: var(--text);
    transition: background var(--t-fast) var(--ease);
  }
  .project:hover,
  .project.open {
    background: var(--bg-hover);
  }
  .project .name {
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 1;
    cursor: default;
  }
  .menu {
    position: absolute;
    top: 34px;
    left: 0;
    z-index: 2;
    min-width: 240px;
    padding: 5px;
    background: var(--bg-2);
    border-radius: var(--r-panel);
    box-shadow: var(--shadow-pop);
    animation: pop var(--t-med) var(--ease);
  }
  .menu button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 32px;
    padding: 0 10px;
    border-radius: var(--r-ctl);
    color: var(--text);
  }
  .menu button:hover {
    background: var(--accent-soft);
  }
  .menu kbd {
    margin-left: auto;
  }
  .sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--line);
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.98);
    }
  }

  .workspaces {
    --w: 104px;
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    display: flex;
    padding: 3px;
    background: var(--bg-0);
    border: 1px solid var(--line-soft);
    border-radius: 9px;
  }
  .workspaces .indicator {
    position: absolute;
    top: 3px;
    left: 3px;
    width: var(--w);
    height: calc(100% - 6px);
    border-radius: 6px;
    background: var(--bg-3);
    box-shadow: 0 1px 0 rgb(255 255 255 / 0.05) inset, 0 2px 6px rgb(0 0 0 / 0.25);
    transform: translateX(calc(var(--i) * var(--w)));
    transition: transform 260ms var(--ease);
  }
  .workspaces button {
    position: relative;
    width: var(--w);
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    font-size: var(--fs-12);
    font-weight: 500;
    color: var(--text-dim);
    transition: color var(--t-fast) var(--ease);
  }
  .workspaces button:hover,
  .workspaces button.active {
    color: var(--text);
  }

  .play {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 28px;
    padding: 0 14px 0 12px;
    border-radius: 8px;
    font-size: var(--fs-12);
    font-weight: 600;
    color: #062b1b;
    background: var(--success);
    box-shadow: 0 0 0 1px rgb(255 255 255 / 0.12) inset, 0 4px 14px rgb(61 220 151 / 0.25);
    transition: transform var(--t-fast) var(--ease), background var(--t-fast) var(--ease), box-shadow var(--t-fast) var(--ease);
  }
  .play:hover {
    transform: translateY(-1px);
    box-shadow: 0 0 0 1px rgb(255 255 255 / 0.12) inset, 0 6px 18px rgb(61 220 151 / 0.35);
  }
  .play.playing {
    color: white;
    background: var(--danger);
    box-shadow: 0 0 0 1px rgb(255 255 255 / 0.12) inset, 0 4px 14px rgb(255 92 122 / 0.35);
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 28px;
    padding: 0 6px 0 10px;
    border-radius: 8px;
    color: var(--text-faint);
    background: var(--bg-0);
    border: 1px solid var(--line-soft);
    font-size: var(--fs-12);
    transition: border-color var(--t-fast) var(--ease), color var(--t-fast) var(--ease);
  }
  .search span {
    margin-right: 18px;
  }
  .search kbd {
    margin-left: -4px;
  }
  .search:hover {
    border-color: var(--line);
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
    background: #e81123;
    color: white;
  }
  @media (max-width: 1180px) {
    .search span {
      display: none;
    }
  }
</style>
