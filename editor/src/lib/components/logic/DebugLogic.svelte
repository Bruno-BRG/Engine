<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { logic } from '$lib/logic/store.svelte';
  import { logicDebug, podeAcionar, type Frame } from '$lib/logic/debug.svelte';
  import { app } from '$lib/state/app.svelte';
  import { runtime } from '$lib/state/runtime.svelte';
  import { scene } from '$lib/state/scene.svelte';
  import { Bug, Play, StepForward, X } from '@lucide/svelte';

  // Visual logic section of the Depurador (VIS-031, DBG-003): the instance the debugger follows, the breakpoints of
  // all scripts, its pause with Continuar and Next operation, the call stack of the pause, and how many nodes ran in
  // the last traced segment. Each node with a script is an instance, and the game keeps running while one is paused.
  const pontos = $derived(Object.entries(logicDebug.breaks).filter(([, ids]) => ids.length > 0));
  const escolhida = $derived(logicDebug.escolhida);
  // The banner shows any pause; the buttons also need a pause at an operation and a running game (see podeAcionar).
  const pausada = $derived(escolhida?.pausada ? escolhida : null);
  const acionavel = $derived(podeAcionar(escolhida, app.paused) ? escolhida : null);
  const rodaram = $derived(escolhida ? (logicDebug.executados[escolhida.uid] ?? []).length : 0);
  // The call stack is listed innermost first: the frame on top is the one the paused operation belongs to.
  const stack = $derived(escolhida ? [...escolhida.stack].reverse() : []);

  // The runtime observes the debugger only while this panel is open (or while a breakpoint exists).
  onMount(() => runtime.definirPainelDepurador(true));
  onDestroy(() => runtime.definirPainelDepurador(false));

  /** Name of the node that runs the instance, for the list. */
  function nomeDaInstancia(uid: string): string {
    return scene.byUid.get(uid)?.name ?? uid;
  }

  /** Script that the instance runs. */
  function scriptDe(uid: string): string {
    const nome = scene.byUid.get(uid)?.props.script;
    return typeof nome === 'string' ? nome : '';
  }

  /** Name of a breakpoint's node: its label when the script is open, its id otherwise. */
  function nomeDoPonto(script: string, id: number): string {
    if (script !== logic.active) return `nó ${id}`;
    const n = logic.snap.nodes.find((x) => x.id === id);
    return n ? logic.labelOf(n) : `nó ${id}`;
  }

  /** Changes the followed instance. If it runs another script, the workspace opens that script. */
  function escolher(uid: string) {
    runtime.escolherInstancia(uid || null);
    const script = uid ? scriptDe(uid) : '';
    if (script && script !== logic.active) void logic.open(script);
  }

  /** Text of a call stack frame: the event, the function, or the timer. */
  function frameLabel(frame: Frame): string {
    if (frame.kind === 'event') return `Evento: ${logic.defsByKind.get(frame.name)?.label ?? frame.name}`;
    if (frame.kind === 'function') return `Função: ${frame.name}`;
    return 'Temporizador: tempo esgotado';
  }

  /** Double-click on a frame (Enter too): opens the instance's script in the Lógica workspace and selects the node. */
  async function goToFrame(uid: string, frame: Frame) {
    const script = scriptDe(uid);
    if (!script) return;
    await logic.openIn(script);
    // The canvas measures itself when it mounts, so the frame waits for it before framing the selection.
    await tick();
    // The open can fail or be replaced by another open: then the workspace does not show this frame's script.
    if (logic.active !== script) return;
    logic.select([frame.node], [], false);
    logic.frameSelection();
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
  {#if escolhida && stack.length}
    <h4>Pilha de chamadas (mais interno primeiro)</h4>
    <ol>
      {#each stack as frame, i (i)}
        <li>
          <button
            type="button"
            class="frame"
            class:current={i === 0}
            title="Clique duplo para ir ao nó"
            ondblclick={() => void goToFrame(escolhida.uid, frame)}
            onkeydown={(e) => {
              if (e.key === 'Enter') void goToFrame(escolhida.uid, frame);
            }}
          >
            <span class="frame-name">{frameLabel(frame)}</span>
            <span class="frame-node mono">{nomeDoPonto(scriptDe(escolhida.uid), frame.node)}</span>
          </button>
        </li>
      {/each}
    </ol>
  {/if}
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
  .frame {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 6px;
    border: 0;
    border-radius: var(--r-ctl);
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: default;
  }
  .frame:hover {
    background: var(--bg-hover);
  }
  .frame.current {
    background: var(--accent-soft);
  }
  .frame-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .frame-node {
    flex: none;
    max-width: 45%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-11);
    color: var(--text-dim);
  }
  ol {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
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
