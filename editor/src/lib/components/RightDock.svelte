<script lang="ts">
  import { app, type RightTab } from '$lib/state/app.svelte';
  import Inspector from './Inspector.svelte';
  import NodeTab from './NodeTab.svelte';
  import { SlidersHorizontal, Waypoints } from '@lucide/svelte';

  // Right-side panel: Inspector (properties) and Node (signals, SCN-034). The chosen tab is saved.
  const tabs: { id: RightTab; label: string; icon: typeof SlidersHorizontal }[] = [
    { id: 'inspector', label: 'Inspector', icon: SlidersHorizontal },
    { id: 'no', label: 'Nó', icon: Waypoints },
  ];

  function choose(id: RightTab) {
    app.rightTab = id;
    app.persistUi();
  }
</script>

<div class="dock">
  <div class="tabs" role="tablist">
    {#each tabs as t (t.id)}
      {@const Icon = t.icon}
      <button
        role="tab"
        aria-selected={app.rightTab === t.id}
        class:active={app.rightTab === t.id}
        onclick={() => choose(t.id)}
      >
        <Icon size={13} />
        {t.label}
      </button>
    {/each}
  </div>
  <div class="content">
    {#if app.rightTab === 'no'}
      <NodeTab />
    {:else}
      <Inspector />
    {/if}
  </div>
</div>

<style>
  .dock {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .tabs {
    flex: none;
    display: flex;
    align-items: stretch;
    height: 22px;
    padding: 0 2px;
    background: var(--bg-toolbar);
    border-bottom: 1px solid var(--line);
  }
  .tabs > button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    font-size: var(--fs-12);
    color: var(--text-dim);
    transition: color var(--t-fast), background var(--t-fast);
  }
  .tabs > button:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
  .tabs > button.active {
    color: var(--text);
    background: var(--bg-1);
  }
  .tabs > button.active :global(svg) {
    color: var(--accent-hi);
  }
  .content {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
</style>
