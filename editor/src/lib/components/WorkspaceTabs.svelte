<script lang="ts">
  import { app, type Workspace } from '$lib/state/app.svelte';
  import * as act from '$lib/actions';
  import { Box, Gamepad2, Workflow, Film, Brush } from '@lucide/svelte';

  const workspaces: { id: Workspace; label: string; icon: typeof Box; key: string }[] = [
    { id: 'cena', label: 'Cena', icon: Box, key: 'Alt+1' },
    { id: 'jogo', label: 'Jogo', icon: Gamepad2, key: 'Alt+2' },
    { id: 'logica', label: 'Lógica', icon: Workflow, key: 'Alt+3' },
    { id: 'animacao', label: 'Animação', icon: Film, key: 'Alt+4' },
    { id: 'sprite', label: 'Sprite', icon: Brush, key: 'Alt+5' },
  ];
</script>

<nav class="tabs">
  {#each workspaces as w (w.id)}
    {@const Icon = w.icon}
    <button class:active={app.workspace === w.id} onclick={() => act.setWorkspace(w.id)} title="{w.label} ({w.key})">
      <Icon size={13} />
      {w.label}
    </button>
  {/each}
</nav>

<style>
  .tabs {
    flex: none;
    display: flex;
    align-items: stretch;
    height: 22px;
    background: var(--bg-toolbar);
    border-bottom: 1px solid var(--line);
  }
  .tabs button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 12px 0 10px;
    color: var(--text-dim);
    transition: color var(--t-fast) var(--ease), background var(--t-fast) var(--ease);
  }
  .tabs button:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
  .tabs button.active {
    color: var(--text);
    background: var(--bg-1);
  }
  .tabs button.active :global(svg) {
    color: var(--accent-hi);
  }
</style>
