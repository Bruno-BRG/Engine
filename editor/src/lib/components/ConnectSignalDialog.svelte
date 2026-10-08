<script lang="ts">
  import { onMount } from 'svelte';
  import { scene, type NodeView, type SignalDef } from '$lib/state/scene.svelte';
  import { X, TriangleAlert } from '@lucide/svelte';

  let { emitter, signal, onclose }: { emitter: NodeView; signal: SignalDef; onclose: () => void } = $props();

  /** Accepted method name: starts with a letter or _, then unaccented letters, digits or _ (up to 64). */
  const METHOD_PATTERN = /^[A-Za-z_][A-Za-z0-9_]{0,63}$/;

  let target = $state('');
  let method = $state('');
  /** Error returned by the last connect attempt. Cleared as soon as the user edits a field. */
  let error = $state('');
  let input: HTMLInputElement | undefined;

  const trimmed = $derived(method.trim());
  const duplicate = $derived(
    (scene.node(emitter.uid)?.connections ?? []).some(
      (c) => c.signal === signal.name && c.target === target && c.method === trimmed,
    ),
  );
  /** Reason the connection is not allowed yet. Empty while the field is blank, so no warning shows before typing. */
  const problem = $derived(
    trimmed === ''
      ? ''
      : !METHOD_PATTERN.test(trimmed)
        ? 'Use letras sem acento, números e _, começando por letra ou _ (até 64 caracteres).'
        : duplicate
          ? 'Essa ligação já existe.'
          : '',
  );
  const canConnect = $derived(trimmed !== '' && problem === '' && target !== '');

  function edited() {
    error = '';
  }

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!canConnect) return;
    const reason = scene.connect(emitter.uid, signal.name, target, trimmed);
    if (reason) error = reason;
    else onclose();
  }

  onMount(() => {
    // Initial target: the first node that is not the emitter. Without another node, the emitter itself.
    target = scene.data.nodes.find((n) => n.uid !== emitter.uid)?.uid ?? emitter.uid;
    method = `on_${signal.name}`;
    input?.focus();
    input?.select();
  });
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="scrim" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && onclose()}>
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="connect-title">
    <form class="form" onsubmit={submit}>
      <header>
        <h3 id="connect-title">Conectar {signal.label}</h3>
        <button type="button" class="icon-btn" aria-label="Fechar" onclick={onclose}><X size={16} /></button>
      </header>
      <p class="from">De <strong>{emitter.name}</strong> · <span class="mono">{signal.name}</span></p>

      <label class="row">
        <span>Nó alvo</span>
        <select class="field" bind:value={target} onchange={edited}>
          {#each scene.data.nodes as n (n.uid)}
            <option value={n.uid}>{'  '.repeat(n.depth)}{n.name} · {n.kind}</option>
          {/each}
        </select>
      </label>

      <label class="row">
        <span>Método do script do alvo</span>
        <input
          class="field mono"
          bind:this={input}
          bind:value={method}
          oninput={edited}
          spellcheck="false"
          autocomplete="off"
          maxlength="64"
          aria-invalid={problem !== '' ? 'true' : undefined}
        />
        <small>Nome do evento no script do alvo que recebe o sinal.</small>
      </label>

      <p class="note">A ligação fica salva no nó. Por enquanto o script do alvo ainda não recebe o sinal.</p>

      {#if problem || error}
        <p class="error" role="alert"><TriangleAlert size={12} /> {problem || error}</p>
      {/if}

      <footer>
        <button type="button" class="btn ghost" onclick={onclose}>Cancelar</button>
        <button type="submit" class="btn primary" disabled={!canConnect}>Conectar</button>
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
  .dialog {
    width: min(380px, calc(100vw - 32px));
    padding: 16px 18px 14px;
    border-radius: var(--r-dlg);
    background: var(--bg-2);
    box-shadow: var(--shadow-pop);
  }
  .form {
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
  .from {
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
  .note {
    margin: 0;
    font-size: var(--fs-11);
    color: var(--text-faint);
    line-height: 1.5;
  }
  .error {
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
