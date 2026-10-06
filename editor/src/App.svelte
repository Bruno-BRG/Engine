<script lang="ts">
  import { app } from '$lib/state/app.svelte';
  import { handleShortcut } from '$lib/commands';
  import TitleBar from '$lib/components/TitleBar.svelte';
  import Home from '$lib/components/Home.svelte';
  import Editor from '$lib/components/Editor.svelte';
  import CommandPalette from '$lib/components/CommandPalette.svelte';
  import Toasts from '$lib/components/Toasts.svelte';

  $effect(() => {
    document.documentElement.dataset.theme = app.theme;
  });
</script>

<svelte:window onkeydown={handleShortcut} oncontextmenu={(e) => e.preventDefault()} />

<div class="shell">
  <TitleBar />
  <main>
    {#if app.project}
      <Editor />
    {:else}
      <Home />
    {/if}
  </main>
</div>

{#if app.paletteOpen}
  <CommandPalette />
{/if}
<Toasts />

<style>
  .shell {
    height: 100%;
    display: grid;
    grid-template-rows: 40px 1fr;
    background: var(--bg-0);
  }
  main {
    min-height: 0;
    position: relative;
  }
</style>
