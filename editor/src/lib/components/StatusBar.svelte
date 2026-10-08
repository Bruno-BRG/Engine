<script lang="ts">
  import { app } from '$lib/state/app.svelte';
  import { scene } from '$lib/state/scene.svelte';
  import { isTauri } from '$lib/platform';
  import { log } from '$lib/state/log.svelte';
  import { tasks } from '$lib/state/tasks.svelte';
  import { Cpu, Gauge, ZoomIn, History, Info, CircleAlert, CircleCheck, TriangleAlert, LoaderCircle } from '@lucide/svelte';

  const last = $derived(log.entries.at(-1));
  // Tarefa em segundo plano mostrada: a mais recente em execução (EDT-013).
  const task = $derived(tasks.running.at(-1) ?? null);

  // Clicar na última mensagem abre o Console com o histórico completo
  function showConsole() {
    app.bottomTab = 'console';
    app.panels.bottom = true;
    app.persistUi();
  }

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
    <span>{scene.data.nodes.length} objetos</span>
    {#if scene.lastAction}<span class="dim"><History size={11} /> {scene.lastAction}</span>{/if}
    {#if task}
      {@const p = task.progress}
      <span class="task" title={p == null ? task.label : `${task.label} (${Math.round(p * 100)}%)`}>
        <LoaderCircle size={11} class="spin" />
        <span class="tlabel">{task.label}</span>
        <span class="meter" class:indet={p == null} aria-hidden="true"><i style:width={p == null ? '' : `${p * 100}%`}></i></span>
        {#if tasks.running.length > 1}<span class="more">+{tasks.running.length - 1}</span>{/if}
      </span>
    {/if}
    {#if last}
      <button class="last {last.level}" onclick={showConsole} title="Abrir Console">
        {#if last.level === 'error'}<CircleAlert size={11} />{:else if last.level === 'warn'}<TriangleAlert size={11} />{:else if last.level === 'success'}<CircleCheck size={11} />{:else}<Info size={11} />{/if}
        <span class="msg">{last.text}</span>
      </button>
    {/if}
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
    padding: 0 8px;
    font-size: var(--fs-11);
    color: var(--text-dim);
    background: var(--bg-toolbar);
    border-top: 1px solid var(--line);
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
  .task {
    flex: 0 1 auto;
    min-width: 0;
    max-width: 260px;
    color: var(--text);
  }
  .task :global(.spin) {
    flex: none;
    color: var(--accent-hi);
    animation: spin 900ms linear infinite;
  }
  .tlabel {
    display: block;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meter {
    position: relative;
    flex: none;
    width: 56px;
    height: 3px;
    overflow: hidden;
    background: var(--bg-3);
  }
  .meter i {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: 0;
    background: var(--accent-hi);
    transition: width var(--t-med) var(--ease);
  }
  /* Sem progresso conhecido: uma faixa percorre a barra */
  .meter.indet i {
    width: 35%;
    animation: slide 1.2s linear infinite;
  }
  .more {
    flex: none;
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @keyframes slide {
    from {
      left: -35%;
    }
    to {
      left: 100%;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .task :global(.spin),
    .meter.indet i {
      animation: none;
    }
    .meter.indet i {
      left: 30%;
    }
  }
  .last {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    max-width: 420px;
    height: 16px;
    padding: 0 4px;
    border-radius: var(--r-ctl);
    color: var(--text-dim);
    transition: background var(--t-fast) var(--ease);
  }
  .last:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .last .msg {
    display: block;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .last.error {
    color: var(--danger);
  }
  .last.warn {
    color: var(--warning);
  }
  .last.success {
    color: var(--success);
  }
  .last.info {
    color: var(--text-dim);
  }
</style>
