<script lang="ts">
  import { onDestroy } from 'svelte';
  import { app } from '$lib/state/app.svelte';
  import { scene } from '$lib/state/scene.svelte';
  import { shortcutsOpen } from '$lib/state/keymap.svelte';
  import * as act from '$lib/actions';
  import ContextMenu from './ContextMenu.svelte';
  import ShortcutsDialog from './ShortcutsDialog.svelte';
  import Splitter from './Splitter.svelte';
  import Hierarchy from './Hierarchy.svelte';
  import RightDock from './RightDock.svelte';
  import Viewport from './Viewport.svelte';
  import BottomDock from './BottomDock.svelte';
  import StatusBar from './StatusBar.svelte';
  import WorkspacePreview from './WorkspacePreview.svelte';
  import WorkspaceTabs from './WorkspaceTabs.svelte';
  import LogicWorkspace from './logic/LogicWorkspace.svelte';

  // Salvamento automático 1,5 s depois da última alteração.
  let first = true;
  $effect(() => {
    scene.revision;
    if (first) {
      first = false;
      return;
    }
    app.dirty = true;
    const t = setTimeout(() => act.save(true), 1500);
    return () => clearTimeout(t);
  });

  const persist = () => app.persistUi();

  // Fechar o projeto desmonta o editor: o diálogo Atalhos não deve reaparecer no próximo projeto.
  onDestroy(() => (shortcutsOpen.open = false));
</script>

<div class="editor" class:playing={app.playing}>
  <div class="body">
    {#if app.panels.left}
      <div class="side" style="width: {app.sizes.left}px">
        <Hierarchy />
      </div>
      <Splitter dir="x" value={app.sizes.left} min={180} max={420} onchange={(v) => (app.sizes.left = v)} onend={persist} />
    {/if}

    <div class="center">
      <WorkspaceTabs />
      <div class="stage">
        <div class="layer" class:hidden={app.workspace !== 'cena'}>
          <Viewport mode="cena" active={app.workspace === 'cena'} />
        </div>
        <div class="layer" class:hidden={app.workspace !== 'jogo'}>
          <Viewport mode="jogo" active={app.workspace === 'jogo'} />
        </div>
        {#if app.workspace === 'logica'}
          <LogicWorkspace />
        {:else if app.workspace !== 'cena' && app.workspace !== 'jogo'}
          <WorkspacePreview workspace={app.workspace} />
        {/if}
      </div>
      {#if app.panels.bottom}
        <Splitter dir="y" value={app.sizes.bottom} min={120} max={520} invert onchange={(v) => (app.sizes.bottom = v)} onend={persist} />
        <div class="bottom" style="height: {app.sizes.bottom}px">
          <BottomDock />
        </div>
      {/if}
    </div>

    {#if app.panels.right}
      <Splitter dir="x" value={app.sizes.right} min={240} max={480} invert onchange={(v) => (app.sizes.right = v)} onend={persist} />
      <div class="side" style="width: {app.sizes.right}px">
        <RightDock />
      </div>
    {/if}
  </div>
  <StatusBar />
  <ContextMenu />
  {#if shortcutsOpen.open}
    <ShortcutsDialog />
  {/if}
</div>

<style>
  .editor {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-rows: 1fr 22px;
    background: var(--bg-0);
  }
  /* Unity tinge os painéis enquanto o jogo roda */
  .editor.playing .side,
  .editor.playing .bottom {
    background: var(--play-tint);
  }
  .body {
    display: flex;
    min-height: 0;
  }
  .side {
    flex: none;
    min-width: 0;
    background: var(--bg-1);
    display: flex;
    flex-direction: column;
  }
  .center {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .stage {
    position: relative;
    flex: 1;
    min-height: 0;
  }
  .layer {
    position: absolute;
    inset: 0;
  }
  .layer.hidden {
    visibility: hidden;
  }
  .bottom {
    flex: none;
    background: var(--bg-1);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
</style>
