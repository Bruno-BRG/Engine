<script lang="ts">
  import { app } from '$lib/state/app.svelte';
  import { scene } from '$lib/state/scene.svelte';
  import * as act from '$lib/actions';
  import Splitter from './Splitter.svelte';
  import Hierarchy from './Hierarchy.svelte';
  import Inspector from './Inspector.svelte';
  import Viewport from './Viewport.svelte';
  import BottomDock from './BottomDock.svelte';
  import StatusBar from './StatusBar.svelte';
  import WorkspacePreview from './WorkspacePreview.svelte';

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
</script>

<div class="editor">
  <div class="body">
    {#if app.panels.left}
      <div class="side" style="width: {app.sizes.left}px">
        <Hierarchy />
      </div>
      <Splitter dir="x" value={app.sizes.left} min={180} max={420} onchange={(v) => (app.sizes.left = v)} onend={persist} />
    {/if}

    <div class="center">
      <div class="stage">
        <div class="layer" class:hidden={app.workspace !== 'cena'}>
          <Viewport />
        </div>
        {#if app.workspace !== 'cena'}
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
        <Inspector />
      </div>
    {/if}
  </div>
  <StatusBar />
</div>

<style>
  .editor {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-rows: 1fr 26px;
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
