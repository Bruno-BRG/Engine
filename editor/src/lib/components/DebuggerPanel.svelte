<script lang="ts">
  import { app } from '$lib/state/app.svelte';
  import { runtime } from '$lib/state/runtime.svelte';
  import { scene } from '$lib/state/scene.svelte';
  import * as act from '$lib/actions';
  import NodeIcon from './NodeIcon.svelte';
  import { Play } from '@lucide/svelte';
  const n = (v: number, p = 1) => v.toFixed(p).replace('.', ',');
</script>

<div class="dbg">
  {#if app.playing}
    <table class="mono">
      <thead>
        <tr><th>Nó</th><th>X</th><th>Y</th><th>Rotação</th><th>Escala</th><th>Visível</th></tr>
      </thead>
      <tbody>
        {#each runtime.live as o (o.uid)}
          <tr class:sel={scene.isSelected(o.uid)} onclick={(e) => scene.select(o.uid, e.ctrlKey || e.shiftKey)}>
            <td class="name"><NodeIcon kind={o.kind} size={12} /> {o.name}</td>
            <td>{n(o.world.x)}</td>
            <td>{n(o.world.y)}</td>
            <td>{n(o.world.rot)}°</td>
            <td>{n(o.world.sx, 2)}×{n(o.world.sy, 2)}</td>
            <td class="beh">{o.visible ? 'sim' : 'não'}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <div class="idle">
      <strong>Valores ao vivo aparecem aqui durante o jogo</strong>
      <span>Na Fase 2, o depurador também vai mostrar o fluxo da lógica visual, breakpoints e o histórico por frame.</span>
      <button class="btn" onclick={act.togglePlay}><Play size={13} fill="currentColor" /> Jogar agora</button>
    </div>
  {/if}
</div>

<style>
  .dbg {
    flex: 1;
    overflow: auto;
    min-width: 0;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--fs-11);
  }
  th {
    position: sticky;
    top: 0;
    background: var(--bg-1);
    text-align: left;
    font-family: var(--font-ui);
    font-size: var(--fs-11);
    font-weight: 600;
    color: var(--text-faint);
    padding: 7px 12px;
    border-bottom: 1px solid var(--line-soft);
  }
  td {
    padding: 4px 12px;
    color: var(--text);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  tr:hover td {
    background: var(--bg-hover);
  }
  tr.sel td {
    background: var(--accent-soft);
  }
  .name {
    font-family: var(--font-ui);
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .beh {
    color: var(--text-dim);
  }
  .idle {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 16px;
    text-align: center;
    color: var(--text-faint);
  }
  .idle strong {
    color: var(--text-dim);
  }
  .idle span {
    max-width: 480px;
    font-size: var(--fs-12);
    line-height: 1.5;
  }
  .idle .btn {
    margin-top: 6px;
  }
</style>
