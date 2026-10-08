<script lang="ts">
  import {
    projectSettings,
    settingsUi,
    addAction,
    cancelCapture,
    removeAction,
    startCapture,
    unbindKey,
  } from '$lib/state/settings.svelte';
  import { X, Keyboard } from '@lucide/svelte';

  let newName = $state('');
  let error = $state<string | null>(null);

  const actions = $derived(Object.keys(projectSettings.input.actions).sort());

  function add(e: SubmitEvent) {
    e.preventDefault();
    const err = addAction(newName);
    error = err;
    if (!err) newName = '';
  }
</script>

<h3>Mapa de entrada</h3>
<p class="hint">
  Ações são os nomes que o jogo consulta (ex.: <span class="mono">ui_left</span>). Teclas são os códigos
  físicos do teclado (<span class="mono">KeyA</span>, <span class="mono">Space</span>,
  <span class="mono">ArrowLeft</span>), então não mudam com o layout. Uma tecla pode servir a mais de uma ação.
  Esc cancela a captura.
</p>

<table class="inputs">
  <thead>
    <tr>
      <th scope="col">Ação</th>
      <th scope="col">Teclas</th>
      <th scope="col"><span class="sr">Opções</span></th>
    </tr>
  </thead>
  <tbody>
    {#each actions as name (name)}
      {@const keys = projectSettings.input.actions[name] ?? []}
      {@const capturing = settingsUi.capture?.action === name}
      <tr>
        <td class="name mono">{name}</td>
        <td>
          <div class="keys">
            {#each keys as key (key)}
              <span class="chip">
                <kbd>{key}</kbd>
                <button class="x" title="Desligar {key}" aria-label="Desligar {key} de {name}" onclick={() => unbindKey(name, key)}>
                  <X size={10} />
                </button>
              </span>
            {:else}
              <span class="none">sem teclas</span>
            {/each}
            {#if capturing}
              <span class="wait" role="status">Pressione uma tecla…</span>
            {/if}
          </div>
        </td>
        <td class="acts">
          <button class="btn" disabled={capturing} title="Ligar mais uma tecla a esta ação" onclick={() => startCapture(name, 'add')}>
            + Tecla
          </button>
          <button class="btn ghost" disabled={capturing} title="Trocar todas as teclas por uma só" onclick={() => startCapture(name, 'only')}>
            Substituir
          </button>
          <button class="icon-btn" title="Remover ação" aria-label="Remover ação {name}" onclick={() => removeAction(name)}>
            <X size={13} />
          </button>
        </td>
      </tr>
    {:else}
      <tr><td colspan="3" class="none empty">Nenhuma ação. Crie uma abaixo.</td></tr>
    {/each}
  </tbody>
</table>

{#if settingsUi.capture}
  <div class="capture" role="status">
    <Keyboard size={13} />
    <span>Pressione a tecla para <span class="mono">{settingsUi.capture.action}</span>.</span>
    <button class="btn" onclick={cancelCapture}>Cancelar</button>
  </div>
{/if}

<form class="new" onsubmit={add}>
  <input class="txt mono" placeholder="nome_da_acao" aria-label="Nome da nova ação" bind:value={newName} />
  <button class="btn" type="submit">Criar ação</button>
</form>
{#if error}<p class="err" role="alert">{error}</p>{/if}

<style>
  .mono {
    font-family: var(--font-mono);
  }
  .inputs {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--fs-12);
  }
  th {
    height: 22px;
    padding: 0 6px;
    text-align: left;
    font-weight: 600;
    color: var(--text-dim);
    border-bottom: 1px solid var(--line);
  }
  td {
    padding: 4px 6px;
    border-bottom: 1px solid var(--line-soft);
    vertical-align: middle;
  }
  .name {
    width: 150px;
    color: var(--text);
  }
  .keys {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    min-height: 22px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    height: 20px;
    padding: 0 2px 0 6px;
    border-radius: var(--r-ctl);
    background: var(--bg-0);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  kbd {
    font-family: var(--font-mono);
    font-size: var(--fs-11);
    color: var(--text);
  }
  .x {
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    border-radius: var(--r-ctl);
    color: var(--text-faint);
  }
  .x:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .none {
    color: var(--text-faint);
  }
  .empty {
    padding: 10px 6px;
  }
  .wait {
    color: var(--accent-hi);
  }
  .acts {
    width: 1%;
    white-space: nowrap;
    text-align: right;
  }
  .acts .btn {
    margin-left: 4px;
  }
  .capture {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 10px;
    padding: 6px 8px;
    border-radius: var(--r-ctl);
    background: var(--accent-soft);
    color: var(--text);
    font-size: var(--fs-12);
  }
  .capture :global(svg) {
    flex: none;
    color: var(--accent-hi);
  }
  .capture > span {
    flex: 1;
  }
  .new {
    display: flex;
    gap: 6px;
    margin-top: 12px;
  }
  .new .txt {
    flex: 1;
  }
  .err {
    margin: 4px 0 0;
  }
</style>
