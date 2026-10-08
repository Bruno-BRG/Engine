<script lang="ts">
  import { scene, type SignalDef } from '$lib/state/scene.svelte';
  import PanelHeader from './PanelHeader.svelte';
  import NodeIcon from './NodeIcon.svelte';
  import ConnectSignalDialog from './ConnectSignalDialog.svelte';
  import { Plus, Unlink, TriangleAlert } from '@lucide/svelte';

  // Aba Nó (SCN-034): os sinais do nó selecionado, com as ligações de cada um.
  const node = $derived(scene.selected);
  const multi = $derived(scene.selection.length > 1);

  /** Sinal que o diálogo está ligando, com o uid do emissor. Fechado quando é null. */
  let dialog = $state<{ emitter: string; signal: SignalDef } | null>(null);
  /** O diálogo só vale para o nó que o abriu: trocar a seleção fecha ele na prática. */
  const aberto = $derived(dialog && node && dialog.emitter === node.uid ? dialog : null);
</script>

<section class="no" aria-label="Nó">
  <PanelHeader title="Nó" />

  {#if !node}
    <div class="avisos">
      <strong>Nada selecionado</strong>
      <span>Escolha um nó na hierarquia ou no viewport para ver os sinais.</span>
    </div>
  {:else if multi}
    <div class="avisos">
      <strong>{scene.selection.length} nós selecionados</strong>
      <span>Selecione um só nó para ver e editar as ligações de sinais.</span>
    </div>
  {:else}
    <div class="cabeca">
      <NodeIcon kind={node.kind} size={12} />
      <span class="nome">{node.name}</span>
      <span class="mono tipo">{node.kind}</span>
    </div>

    {#if node.signals.length === 0}
      <p class="vazio-geral">{node.kind} não emite sinais.</p>
    {:else}
      <div class="corpo">
        {#each node.signals as sinal (sinal.name)}
          {@const ligacoes = node.connections.filter((c) => c.signal === sinal.name)}
          <section class="sinal">
            <div class="sinal-cabeca">
              <span class="rotulo">{sinal.label}</span>
              <span class="mono codigo">{sinal.name}</span>
              <button
                class="add"
                aria-label="Conectar {sinal.label}"
                title="Conectar sinal"
                onclick={() => (dialog = { emitter: node.uid, signal: sinal })}
              >
                <Plus size={12} />
              </button>
            </div>

            {#if ligacoes.length === 0}
              <p class="vazio">Sem ligações</p>
            {:else}
              <ul class="lista">
                {#each ligacoes as link (link.target + '|' + link.method)}
                  {@const alvo = scene.node(link.target)}
                  <li class="ligacao" class:ausente={!alvo}>
                    {#if alvo}
                      <NodeIcon kind={alvo.kind} size={11} />
                      <span class="alvo">{alvo.name}</span>
                    {:else}
                      <TriangleAlert size={11} />
                      <span class="alvo">Alvo ausente</span>
                    {/if}
                    <span class="mono metodo">{link.method}</span>
                    <button
                      class="remover"
                      aria-label="Remover ligação com {link.method}"
                      title="Remover ligação"
                      onclick={() => scene.disconnect(node.uid, link)}
                    >
                      <Unlink size={11} />
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
          </section>
        {/each}
      </div>
    {/if}
  {/if}
</section>

{#if aberto && node}
  <ConnectSignalDialog emitter={node} signal={aberto.signal} onclose={() => (dialog = null)} />
{/if}

<style>
  .no {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--bg-1);
  }
  .avisos {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 16px 14px;
    font-size: var(--fs-12);
    color: var(--text-faint);
  }
  .avisos strong {
    color: var(--text-dim);
  }
  .cabeca {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 10px;
    font-size: var(--fs-12);
    color: var(--text);
    border-bottom: 1px solid var(--line-soft);
  }
  .nome {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }
  .tipo {
    margin-left: auto;
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .vazio-geral {
    margin: 0;
    padding: 12px 10px;
    font-size: var(--fs-12);
    color: var(--text-faint);
  }
  .corpo {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding-bottom: 8px;
  }
  .sinal {
    border-bottom: 1px solid var(--line-soft);
  }
  .sinal-cabeca {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 4px 0 8px;
    background: var(--bg-toolbar);
  }
  .rotulo {
    font-size: var(--fs-12);
    font-weight: 600;
    color: var(--text);
  }
  .codigo {
    font-size: var(--fs-11);
    color: var(--text-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .add,
  .remover {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: var(--r-ctl);
    color: var(--text-faint);
  }
  .sinal-cabeca .add {
    margin-left: auto;
  }
  .add:hover,
  .remover:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
  .vazio {
    margin: 0;
    padding: 4px 10px 6px;
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .lista {
    list-style: none;
    margin: 0;
    padding: 4px 6px 6px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .ligacao {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 24px;
    padding: 0 2px 0 6px;
    border-radius: var(--r-ctl);
    font-size: var(--fs-12);
    color: var(--text);
  }
  .ligacao:hover {
    background: var(--bg-hover);
  }
  .alvo {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .metodo {
    margin-left: auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-11);
    color: var(--text-dim);
  }
  .ligacao.ausente {
    color: var(--warning);
  }
  .ligacao.ausente .metodo {
    color: var(--warning);
  }
</style>
