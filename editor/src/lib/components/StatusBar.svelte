<script lang="ts">
  import { app } from '$lib/state/app.svelte';
  import { scene } from '$lib/state/scene.svelte';
  import { isTauri } from '$lib/platform';
  import { Cpu, Gauge, ZoomIn, History } from '@lucide/svelte';

  let now = $state(Date.now());
  $effect(() => {
    const t = setInterval(() => (now = Date.now()), 15000);
    return () => clearInterval(t);
  });
  const saved = $derived.by(() => {
    if (app.saving) return 'Salvando…';
    if (app.dirty) return 'Alterações não salvas';
    if (!app.lastSaved) return 'Salvo';
    const s = (now - app.lastSaved.getTime()) / 1000;
    return s < 60 ? 'Salvo agora' : 'Salvo há ' + Math.floor(s / 60) + ' min';
  });
</script>

<footer>
  <div class="group">
    <span class="save" class:dirty={app.dirty || app.saving}><i></i>{saved}</span>
    <span>{scene.data.objects.length} objetos</span>
    {#if scene.lastAction}<span class="dim"><History size={11} /> {scene.lastAction}</span>{/if}
  </div>
  <div class="group">
    {#if app.playing}<span class="fps"><Gauge size={11} /> {app.fps} fps</span>{/if}
    {#if app.gpu}<span class="dim" title="Backend gráfico do viewport"><Cpu size={11} /> {app.gpu}</span>{/if}
    <span class="dim"><ZoomIn size={11} /> {Math.round(app.zoom * 100)}%</span>
    <span class="dim">Nexo 0.1.0 · {isTauri ? 'desktop' : 'navegador'}</span>
  </div>
</footer>

<style>
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 0 12px;
    font-size: var(--fs-11);
    color: var(--text-dim);
    background: var(--bg-1);
    border-top: 1px solid var(--line-soft);
    white-space: nowrap;
    overflow: hidden;
  }
  .group {
    display: flex;
    align-items: center;
    gap: 16px;
    min-width: 0;
  }
  span {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .dim {
    color: var(--text-faint);
  }
  .save i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--success);
  }
  .save.dirty i {
    background: var(--warning);
  }
  .fps {
    color: var(--success);
    font-variant-numeric: tabular-nums;
  }
</style>
