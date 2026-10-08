<script lang="ts">
  import { logic } from '$lib/logic/store.svelte';
  import {
    TIPO_NOME,
    TIPOS_VARIAVEL,
    textOf,
    valueFromText,
    zeroOf,
    type PinType,
    type Variable,
  } from '$lib/logic/model';
  import { X } from '@lucide/svelte';

  // Variáveis do script (VIS-022): declarar, marcar como exportada (aparece no Inspector, INS-005) e remover.
  let nome = $state('');
  let ty = $state<PinType>('Float');
  let padrao = $state('0');
  let exporta = $state(false);

  function adiciona() {
    const n = nome.trim();
    if (!n) return;
    const valor = valueFromText(ty, padrao) ?? zeroOf(ty);
    if (logic.setVariable(n, ty, valor, exporta)) {
      nome = '';
      padrao = '0';
    }
  }

  function alternaExportada(v: Variable) {
    logic.setVariable(v.name, v.ty, v.default, !v.exported);
  }
</script>

<section class="painel">
  <header><span>Variáveis</span><span class="contagem mono">{logic.snap.variables.length}</span></header>
  {#if logic.active}
    <div class="corpo">
      <ul>
        {#each logic.snap.variables as v (v.name)}
          <li>
            <label class="linha">
              <input type="checkbox" checked={v.exported} onchange={() => alternaExportada(v)} />
              <span class="nome mono" title={v.exported ? 'Exportada: aparece no Inspector' : 'Local ao script'}>{v.name}</span>
            </label>
            <span class="tipo">{TIPO_NOME[v.ty]}</span>
            <span class="valor mono">{textOf(v.default)}</span>
            <button class="icon-btn" title="Remover variável" aria-label="Remover {v.name}" onclick={() => logic.removeVariable(v.name)}>
              <X size={11} />
            </button>
          </li>
        {:else}
          <li class="vazio">Sem variáveis.</li>
        {/each}
      </ul>

      <form class="novo" onsubmit={(e) => { e.preventDefault(); adiciona(); }}>
        <input class="campo" placeholder="Nome" bind:value={nome} aria-label="Nome da variável" spellcheck="false" />
        <select class="campo" bind:value={ty} aria-label="Tipo da variável">
          {#each TIPOS_VARIAVEL as t (t)}
            <option value={t}>{TIPO_NOME[t]}</option>
          {/each}
        </select>
        <input
          class="campo"
          placeholder={ty === 'Bool' ? 'true ou false' : 'Padrão'}
          bind:value={padrao}
          aria-label="Valor padrão"
        />
        <label class="linha">
          <input type="checkbox" bind:checked={exporta} />
          <span>Exportar para o Inspector</span>
        </label>
        <button class="btn" type="submit" disabled={!nome.trim()}>Adicionar variável</button>
      </form>
    </div>
  {:else}
    <p class="vazio corpo">Abra um script para declarar variáveis.</p>
  {/if}
</section>

<style>
  .painel {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  header {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 22px;
    padding: 0 8px;
    font-size: var(--fs-12);
    color: var(--text);
    background: var(--bg-toolbar);
    border-bottom: 1px solid var(--line);
  }
  .contagem {
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .corpo {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 6px;
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
    font-size: var(--fs-12);
  }
  .linha {
    display: flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
  }
  .nome {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tipo {
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .valor {
    margin-left: auto;
    font-size: var(--fs-11);
    color: var(--text-dim);
    max-width: 80px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .novo {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding-top: 6px;
    border-top: 1px solid var(--line-soft);
  }
  .campo {
    height: 22px;
    padding: 0 6px;
    border-radius: var(--r-ctl);
    background: var(--bg-2);
    border: 1px solid var(--line);
    outline: none;
    font-size: var(--fs-12);
    user-select: text;
  }
  .campo:focus {
    border-color: var(--accent);
  }
  .btn {
    align-self: flex-start;
  }
  .vazio {
    font-size: var(--fs-12);
    color: var(--text-faint);
    line-height: 1.5;
  }
</style>
