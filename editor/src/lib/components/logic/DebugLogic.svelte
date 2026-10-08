<script lang="ts">
  import { logic } from '$lib/logic/store.svelte';
  import { logicDebug } from '$lib/logic/debug.svelte';
  import { Bug, Play, StepForward, X } from '@lucide/svelte';

  // Seção de lógica visual do Depurador (VIS-031, DBG-003): pontos de parada de todos os scripts, a pausa
  // atual com Continuar e Passo, e quantos nós rodaram no último quadro. Os dados chegam da reprodução.
  const pontos = $derived(Object.entries(logicDebug.breaks).filter(([, ids]) => ids.length > 0));
  const controles = $derived(logicDebug.controles);
  const pausa = $derived(logicDebug.pausa);
  const rodaram = $derived(logic.active ? (logicDebug.executados[logic.active] ?? []).length : 0);

  /** Nome do nó para a lista: com o script aberto, o rótulo; nos outros, o id. */
  function nomeDo(script: string, id: number): string {
    if (script !== logic.active) return `nó ${id}`;
    const n = logic.snap.nodes.find((x) => x.id === id);
    return n ? logic.labelOf(n) : `nó ${id}`;
  }
</script>

<section class="dbg-logic">
  <header><Bug size={12} /> <span>Lógica visual</span></header>

  {#if pausa}
    <p class="pausa">Pausado em <b class="mono">{pausa.script}</b>, {nomeDo(pausa.script, pausa.node)} ({pausa.evento})</p>
  {/if}
  <div class="botoes">
    <button class="btn" disabled={!controles || !pausa} onclick={() => controles?.continuar()}>
      <Play size={12} fill="currentColor" /> Continuar
    </button>
    <button class="btn" disabled={!controles || !pausa} onclick={() => controles?.passo()}>
      <StepForward size={12} /> Passo
    </button>
  </div>
  {#if !controles}
    <p class="nota">Pontos de parada e pausas valem durante o jogo (F5). Marque um nó com F9.</p>
  {/if}

  <h4>Pontos de parada</h4>
  <ul>
    {#each pontos as [script, ids] (script)}
      {#each ids as id (id)}
        <li>
          <span class="mono script">{script}</span>
          <span class="no">{nomeDo(script, id)}</span>
          <button class="icon-btn" title="Remover ponto de parada" aria-label="Remover ponto de parada" onclick={() => logicDebug.toggleBreakpoints(script, [id])}>
            <X size={11} />
          </button>
        </li>
      {/each}
    {:else}
      <li class="vazio">Nenhum. Selecione um nó no grafo e aperte F9.</li>
    {/each}
  </ul>

  <p class="nota">
    {#if logic.active}
      Último quadro: {rodaram} nó{rodaram === 1 ? '' : 's'} executado{rodaram === 1 ? '' : 's'} em {logic.active}.
    {:else}
      Abra um script para ver o rastro dele.
    {/if}
  </p>
</section>

<style>
  .dbg-logic {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 12px 10px;
    border-top: 1px solid var(--line-soft);
    font-size: var(--fs-12);
    color: var(--text);
  }
  header {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-dim);
    font-size: var(--fs-11);
    font-weight: 600;
  }
  h4 {
    margin: 4px 0 0;
    font-size: var(--fs-11);
    font-weight: 600;
    color: var(--text-faint);
  }
  .pausa {
    margin: 0;
    padding: 4px 6px;
    border-radius: var(--r-ctl);
    background: var(--accent-soft);
  }
  .botoes {
    display: flex;
    gap: 4px;
  }
  .botoes .btn {
    height: 22px;
    padding: 0 8px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  li {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .script {
    font-size: var(--fs-11);
    color: var(--text-dim);
  }
  .no {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .vazio,
  .nota {
    margin: 0;
    font-size: var(--fs-11);
    color: var(--text-faint);
    line-height: 1.5;
  }
</style>
