<script lang="ts">
  import { onMount } from 'svelte';
  import { scene, type NodeView, type SignalDef } from '$lib/state/scene.svelte';
  import { X, TriangleAlert } from '@lucide/svelte';

  let { emitter, signal, onclose }: { emitter: NodeView; signal: SignalDef; onclose: () => void } = $props();

  /** Nome de método aceito: começa por letra ou _, segue com letras sem acento, números ou _ (até 64). */
  const METODO = /^[A-Za-z_][A-Za-z0-9_]{0,63}$/;

  let alvo = $state('');
  let metodo = $state('');
  let erro = $state('');
  let campo: HTMLInputElement | undefined;

  const limpo = $derived(metodo.trim());
  const repetida = $derived(
    (scene.node(emitter.uid)?.connections ?? []).some(
      (c) => c.signal === signal.name && c.target === alvo && c.method === limpo,
    ),
  );
  /** Motivo para ainda não ligar. Vazio enquanto o campo está em branco, sem mostrar aviso antes de digitar. */
  const problema = $derived(
    limpo === ''
      ? ''
      : !METODO.test(limpo)
        ? 'Use letras sem acento, números e _, começando por letra ou _ (até 64 caracteres).'
        : repetida
          ? 'Essa ligação já existe.'
          : '',
  );
  const podeLigar = $derived(limpo !== '' && problema === '' && alvo !== '');

  function enviar(e: SubmitEvent) {
    e.preventDefault();
    if (!podeLigar) return;
    const motivo = scene.connect(emitter.uid, signal.name, alvo, limpo);
    if (motivo) erro = motivo;
    else onclose();
  }

  onMount(() => {
    // Alvo inicial: o primeiro nó que não é o emissor. Sem outro nó, o próprio emissor.
    alvo = scene.data.nodes.find((n) => n.uid !== emitter.uid)?.uid ?? emitter.uid;
    metodo = `on_${signal.name}`;
    campo?.focus();
    campo?.select();
  });
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="scrim" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && onclose()}>
  <div class="dlg" role="dialog" aria-modal="true" aria-labelledby="conectar-titulo">
    <form class="miolo" onsubmit={enviar}>
      <header>
        <h3 id="conectar-titulo">Conectar {signal.label}</h3>
        <button type="button" class="icon-btn" aria-label="Fechar" onclick={onclose}><X size={16} /></button>
      </header>
      <p class="de">De <strong>{emitter.name}</strong> · <span class="mono">{signal.name}</span></p>

      <label class="row">
        <span>Nó alvo</span>
        <select class="field" bind:value={alvo}>
          {#each scene.data.nodes as n (n.uid)}
            <option value={n.uid}>{'  '.repeat(n.depth)}{n.name} · {n.kind}</option>
          {/each}
        </select>
      </label>

      <label class="row">
        <span>Método do script do alvo</span>
        <input
          class="field mono"
          bind:this={campo}
          bind:value={metodo}
          spellcheck="false"
          autocomplete="off"
          maxlength="64"
          aria-invalid={problema !== '' ? 'true' : undefined}
        />
        <small>Nome do evento no script do alvo que recebe o sinal.</small>
      </label>

      <p class="aviso">A ligação fica salva no nó. Por enquanto o script do alvo ainda não recebe o sinal.</p>

      {#if problema || erro}
        <p class="erro" role="alert"><TriangleAlert size={12} /> {problema || erro}</p>
      {/if}

      <footer>
        <button type="button" class="btn ghost" onclick={onclose}>Cancelar</button>
        <button type="submit" class="btn primary" disabled={!podeLigar}>Conectar</button>
      </footer>
    </form>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 120;
    display: grid;
    place-items: center;
    background: var(--scrim);
    backdrop-filter: blur(4px);
  }
  .dlg {
    width: min(380px, calc(100vw - 32px));
    padding: 16px 18px 14px;
    border-radius: var(--r-dlg);
    background: var(--bg-2);
    box-shadow: var(--shadow-pop);
  }
  .miolo {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h3 {
    margin: 0;
    font-size: var(--fs-15);
    font-weight: 650;
  }
  .de {
    margin: 0;
    font-size: var(--fs-12);
    color: var(--text-dim);
  }
  .row {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: var(--fs-12);
    color: var(--text-dim);
  }
  .row .field {
    height: 28px;
    font-size: var(--fs-12);
  }
  small {
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .aviso {
    margin: 0;
    font-size: var(--fs-11);
    color: var(--text-faint);
    line-height: 1.5;
  }
  .erro {
    margin: 0;
    display: flex;
    align-items: flex-start;
    gap: 6px;
    font-size: var(--fs-12);
    color: var(--danger);
    line-height: 1.4;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
