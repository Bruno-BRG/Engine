<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { logic } from '$lib/logic/store.svelte';
  import { logicDebug, podeAcionar } from '$lib/logic/debug.svelte';
  import { app } from '$lib/state/app.svelte';
  import { runtime } from '$lib/state/runtime.svelte';
  import { scene } from '$lib/state/scene.svelte';
  import { Bug, Play, StepForward, X } from '@lucide/svelte';

  // Seção de lógica visual do Depurador (VIS-031, DBG-003): a instância que o Depurador acompanha, os pontos de
  // parada de todos os scripts, a pausa dela com Continuar e Próxima operação, e quantos nós rodaram no último
  // trecho com rastro. Cada nó com script é uma instância, e o jogo segue quando uma delas pausa.
  const pontos = $derived(Object.entries(logicDebug.breaks).filter(([, ids]) => ids.length > 0));
  const escolhida = $derived(logicDebug.escolhida);
  // The banner shows any pause; the buttons also need a pause at an operation and a running game (see podeAcionar).
  const pausada = $derived(escolhida?.pausada ? escolhida : null);
  const acionavel = $derived(podeAcionar(escolhida, app.paused) ? escolhida : null);
  const rodaram = $derived(escolhida ? (logicDebug.executados[escolhida.uid] ?? []).length : 0);

  // The runtime observes the debugger only while this panel is open (or while a breakpoint exists).
  onMount(() => runtime.definirPainelDepurador(true));
  onDestroy(() => runtime.definirPainelDepurador(false));

  /** Nome do nó que roda a instância, para a lista. */
  function nomeDaInstancia(uid: string): string {
    return scene.byUid.get(uid)?.name ?? uid;
  }

  /** Script que a instância roda. */
  function scriptDe(uid: string): string {
    const nome = scene.byUid.get(uid)?.props.script;
    return typeof nome === 'string' ? nome : '';
  }

  /** Nome do nó de um ponto de parada: com o script aberto, o rótulo; nos outros, o id. */
  function nomeDoPonto(script: string, id: number): string {
    if (script !== logic.active) return `nó ${id}`;
    const n = logic.snap.nodes.find((x) => x.id === id);
    return n ? logic.labelOf(n) : `nó ${id}`;
  }

  /** Troca a instância acompanhada. Se ela roda outro script, o workspace abre esse script. */
  function escolher(uid: string) {
    runtime.escolherInstancia(uid || null);
    const script = uid ? scriptDe(uid) : '';
    if (script && script !== logic.active) void logic.open(script);
  }
</script>

<section class="dbg-logic">
  <header><Bug size={12} /> <span>Lógica visual</span></header>

  <label class="instancia">
    <span>Instância</span>
    <select
      value={logicDebug.selecionada ?? ''}
      disabled={logicDebug.instancias.length === 0}
      aria-label="Instância depurada"
      onchange={(e) => escolher(e.currentTarget.value)}
    >
      {#each logicDebug.instancias as inst (inst.uid)}
        <option value={inst.uid}>
          {nomeDaInstancia(inst.uid)} · {scriptDe(inst.uid)}{inst.pausada ? ' (pausado)' : ''}
        </option>
      {:else}
        <option value="">Nenhuma (jogue com F5)</option>
      {/each}
    </select>
  </label>

  {#if pausada}
    <p class="pausa">
      Pausado em <b class="mono">{scriptDe(pausada.uid)}</b>{#if pausada.node !== null}, {nomeDoPonto(scriptDe(pausada.uid), pausada.node)} ({pausada.evento}){:else}, início do quadro{/if}
    </p>
  {/if}
  <div class="botoes">
    <button class="btn" disabled={!acionavel} onclick={() => acionavel && runtime.depurar('continuar', acionavel.uid)}>
      <Play size={12} fill="currentColor" /> Continuar
    </button>
    <button
      class="btn"
      disabled={!acionavel}
      title="Roda uma operação desta instância e para na seguinte. F10 avança um quadro do jogo, sem liberar esta instância pausada (use Continuar)."
      onclick={() => acionavel && runtime.depurar('proxima', acionavel.uid)}
    >
      <StepForward size={12} /> Próxima operação
    </button>
  </div>
  {#if logicDebug.instancias.length === 0}
    <p class="nota">Pontos de parada e pausas valem durante o jogo (F5). Marque um nó com F9.</p>
  {/if}

  <h4>Pontos de parada</h4>
  <ul>
    {#each pontos as [script, ids] (script)}
      {#each ids as id (id)}
        <li>
          <span class="mono script">{script}</span>
          <span class="no">{nomeDoPonto(script, id)}</span>
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
    {#if escolhida}
      Último rastro: {rodaram} nó{rodaram === 1 ? '' : 's'} executado{rodaram === 1 ? '' : 's'} em {scriptDe(escolhida.uid)}.
    {:else}
      Escolha uma instância para ver o rastro dela.
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
  .instancia {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-11);
    color: var(--text-dim);
  }
  .instancia select {
    flex: 1;
    min-width: 0;
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
