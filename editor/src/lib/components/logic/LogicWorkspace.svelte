<script lang="ts">
  import { logic } from '$lib/logic/store.svelte';
  import { focusPanel } from '$lib/state/focus.svelte';
  import { log } from '$lib/state/log.svelte';
  import * as act from '$lib/actions';
  import ScriptList from './ScriptList.svelte';
  import GraphCanvas from './GraphCanvas.svelte';
  import EventsPanel from './EventsPanel.svelte';
  import UsedByPanel from './UsedByPanel.svelte';
  import VariablesPanel from './VariablesPanel.svelte';
  import ScriptCSharp from './ScriptCSharp.svelte';
  import { Undo2, Redo2, Maximize, Focus, Plus, MessageSquare } from '@lucide/svelte';

  let nomeFuncao = $state('');
  const erros = $derived(logic.errors.length);

  /** Colapsa a seleção em função (VIS-015). Só o que funções expressam; macros ficam pendentes. */
  function colapsa() {
    const nome = nomeFuncao.trim();
    if (!nome) {
      log.toast('warn', 'Dê um nome à função', 'O nome aparece no Chamar função que substitui a seleção.');
      return;
    }
    if (logic.collapseSelection(nome)) nomeFuncao = '';
  }
</script>

<div class="logic" use:focusPanel={'logic'} role="region" aria-label="Lógica visual">
  <aside class="esquerda">
    <ScriptList />
  </aside>

  <section class="centro">
    <div class="barra">
      <span class="nome mono" title={logic.active ?? ''}>{logic.active ?? 'Nenhum script aberto'}</span>
      <span class="sep"></span>
      <button class="icon-btn" title="Desfazer (Ctrl+Z)" aria-label="Desfazer" disabled={!logic.canUndo} onclick={act.undo}>
        <Undo2 size={13} />
      </button>
      <button class="icon-btn" title="Refazer (Ctrl+Shift+Z)" aria-label="Refazer" disabled={!logic.canRedo} onclick={act.redo}>
        <Redo2 size={13} />
      </button>
      <span class="sep"></span>
      <button class="icon-btn" title="Enquadrar tudo (Home)" aria-label="Enquadrar tudo" disabled={!logic.active} onclick={() => logic.frameAll()}>
        <Maximize size={13} />
      </button>
      <button class="icon-btn" title="Enquadrar seleção (F)" aria-label="Enquadrar seleção" disabled={!logic.active} onclick={() => logic.frameSelection()}>
        <Focus size={13} />
      </button>
      <button class="icon-btn" title="Caixa de comentário no centro da tela" aria-label="Adicionar caixa de comentário" disabled={!logic.active} onclick={() => logic.addCommentAtCenter()}>
        <MessageSquare size={12} />
      </button>
      <span class="sep"></span>
      <input
        class="campo"
        placeholder="Nome da função"
        aria-label="Nome da função"
        bind:value={nomeFuncao}
        disabled={!logic.active}
        spellcheck="false"
      />
      <button class="btn ghost" title="Colapsa a seleção num nó Função" disabled={!logic.selNodes.length} onclick={colapsa}>
        <Plus size={12} /> Colapsar em função
      </button>
      <span class="status" class:erro={erros > 0}>{erros ? `${erros} erro${erros > 1 ? 's' : ''}` : 'Sem erros'}</span>
    </div>

    {#if logic.active}
      <GraphCanvas />
    {:else}
      <div class="sem">
        <strong>Nenhum script aberto</strong>
        <span>Crie um script com +, ou escolha um da lista. Cada script vira o arquivo <span class="mono">scripts/nome.logic.ron</span> do projeto.</span>
        {#if logic.project}
          <button class="btn primary" onclick={() => logic.create()}><Plus size={12} /> Novo script</button>
        {/if}
      </div>
    {/if}
  </section>

  <aside class="direita">
    <EventsPanel />
    <UsedByPanel />
    <VariablesPanel />
    <ScriptCSharp />
  </aside>
</div>

<style>
  .logic {
    position: absolute;
    inset: 0;
    display: flex;
    background: var(--bg-0);
    min-width: 0;
  }
  .esquerda {
    flex: none;
    width: 180px;
    border-right: 1px solid var(--line);
    background: var(--bg-1);
    min-height: 0;
  }
  .centro {
    flex: 1;
    min-width: 0;
    position: relative;
    display: flex;
    flex-direction: column;
  }
  .direita {
    flex: none;
    width: 240px;
    border-left: 1px solid var(--line);
    background: var(--bg-1);
    overflow: auto;
    min-height: 0;
  }
  .barra {
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
    height: 26px;
    padding: 0 6px;
    background: var(--bg-toolbar);
    border-bottom: 1px solid var(--line);
    font-size: var(--fs-12);
    min-width: 0;
  }
  .nome {
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
  }
  .sep {
    width: 1px;
    height: 16px;
    margin: 0 4px;
    background: var(--line);
  }
  .campo {
    width: 130px;
    height: 20px;
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
  .status {
    margin-left: auto;
    font-size: var(--fs-11);
    color: var(--text-faint);
    white-space: nowrap;
  }
  .status.erro {
    color: var(--type-bool);
  }
  .sem {
    position: absolute;
    inset: 26px 0 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 16px;
    text-align: center;
    color: var(--text-faint);
    background-image: radial-gradient(circle, var(--canvas-dot) 1px, transparent 1px);
    background-size: 22px 22px;
  }
  .sem strong {
    color: var(--text-dim);
    font-size: var(--fs-15);
  }
  .sem span {
    max-width: 380px;
    line-height: 1.5;
    font-size: var(--fs-12);
  }
</style>
