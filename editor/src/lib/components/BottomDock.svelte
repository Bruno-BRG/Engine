<script lang="ts">
  import { app, type BottomTab } from '$lib/state/app.svelte';
  import { log } from '$lib/state/log.svelte';
  import * as act from '$lib/actions';
  import AssetsPanel from './AssetsPanel.svelte';
  import ConsolePanel from './ConsolePanel.svelte';
  import DebuggerPanel from './DebuggerPanel.svelte';
  import CSharpPanel from './CSharpPanel.svelte';
  import AudioMixer from './AudioMixer.svelte';
  import { Shapes, Terminal, Bug, Braces, ChevronDown, SlidersHorizontal } from '@lucide/svelte';

  const tabs: { id: BottomTab; label: string; icon: typeof Shapes }[] = [
    { id: 'assets', label: 'Assets', icon: Shapes },
    { id: 'console', label: 'Console', icon: Terminal },
    { id: 'depurador', label: 'Depurador', icon: Bug },
    { id: 'csharp', label: 'C#', icon: Braces },
    { id: 'mixer', label: 'Mixer', icon: SlidersHorizontal },
  ];
  const errors = $derived(log.entries.filter((e) => e.level === 'error').length);
</script>

<div class="tabs">
  {#each tabs as t (t.id)}
    {@const Icon = t.icon}
    <button class:active={app.bottomTab === t.id} onclick={() => { app.bottomTab = t.id; app.persistUi(); }}>
      <Icon size={13} />
      {t.label}
      {#if t.id === 'console' && errors > 0}<span class="count err">{errors}</span>{/if}
      {#if t.id === 'depurador' && app.playing}<span class="live"></span>{/if}
    </button>
  {/each}
  <button class="icon-btn hide" title="Ocultar painel (Ctrl+J)" onclick={() => act.togglePanel('bottom')}><ChevronDown size={15} /></button>
</div>
<div class="content">
  {#if app.bottomTab === 'assets'}
    <AssetsPanel />
  {:else if app.bottomTab === 'console'}
    <ConsolePanel />
  {:else if app.bottomTab === 'depurador'}
    <DebuggerPanel />
  {:else if app.bottomTab === 'mixer'}
    <AudioMixer />
  {:else}
    <CSharpPanel />
  {/if}
</div>

<style>
  .tabs {
    flex: none;
    display: flex;
    align-items: stretch;
    gap: 0;
    height: 22px;
    padding: 0 2px;
    background: var(--bg-toolbar);
    border-bottom: 1px solid var(--line);
  }
  .tabs > button:not(.hide) {
    position: relative;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    font-size: var(--fs-12);
    color: var(--text-dim);
    transition: color var(--t-fast), background var(--t-fast);
  }
  .tabs > button:not(.hide):hover {
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
  .hide {
    margin-left: auto;
  }
  .count {
    font-size: var(--fs-11);
    font-weight: 700;
    padding: 0 5px;
    border-radius: var(--r-ctl);
    line-height: 15px;
  }
  .count.err {
    color: white;
    background: var(--danger);
  }
  .live {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--success);
    box-shadow: 0 0 8px var(--success);
  }
  .content {
    flex: 1;
    min-height: 0;
    display: flex;
  }
</style>
