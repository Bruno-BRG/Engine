<script lang="ts">
  import { scene, type SignalDef } from '$lib/state/scene.svelte';
  import PanelHeader from './PanelHeader.svelte';
  import NodeIcon from './NodeIcon.svelte';
  import ConnectSignalDialog from './ConnectSignalDialog.svelte';
  import { Plus, Unlink, TriangleAlert } from '@lucide/svelte';

  // Node tab (SCN-034): the signals of the selected node, with each signal's connections.
  const node = $derived(scene.selected);
  const multi = $derived(scene.selection.length > 1);

  /** Signal the dialog is connecting, with the emitter's UID. null when the dialog is closed. */
  let dialog = $state<{ emitter: string; signal: SignalDef } | null>(null);
  /** The dialog only applies to the node that opened it: changing the selection effectively closes it. */
  const connecting = $derived(dialog && node && dialog.emitter === node.uid ? dialog : null);
</script>

<section class="node-tab" aria-label="Nó">
  <PanelHeader title="Nó" />

  {#if !node}
    <div class="notice">
      <strong>Nada selecionado</strong>
      <span>Escolha um nó na hierarquia ou no viewport para ver os sinais.</span>
    </div>
  {:else if multi}
    <div class="notice">
      <strong>{scene.selection.length} nós selecionados</strong>
      <span>Selecione um só nó para ver e editar as ligações de sinais.</span>
    </div>
  {:else}
    <div class="head">
      <NodeIcon kind={node.kind} size={12} />
      <span class="name">{node.name}</span>
      <span class="mono kind">{node.kind}</span>
    </div>

    {#if node.signals.length === 0}
      <p class="empty-all">{node.kind} não emite sinais.</p>
    {:else}
      <div class="body">
        {#each node.signals as signal (signal.name)}
          {@const links = node.connections.filter((c) => c.signal === signal.name)}
          <section class="signal">
            <div class="signal-head">
              <span class="label">{signal.label}</span>
              <span class="mono code">{signal.name}</span>
              <button
                class="add"
                aria-label="Conectar {signal.label}"
                title="Conectar sinal"
                onclick={() => (dialog = { emitter: node.uid, signal })}
              >
                <Plus size={12} />
              </button>
            </div>

            {#if links.length === 0}
              <p class="empty">Sem ligações</p>
            {:else}
              <ul class="list">
                {#each links as link (link.target + '|' + link.method)}
                  {@const target = scene.node(link.target)}
                  <li class="link" class:missing={!target}>
                    {#if target}
                      <NodeIcon kind={target.kind} size={11} />
                      <span class="target">{target.name}</span>
                    {:else}
                      <TriangleAlert size={11} />
                      <span class="target">Alvo ausente</span>
                    {/if}
                    <span class="mono method">{link.method}</span>
                    <button
                      class="remove"
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

{#if connecting && node}
  <ConnectSignalDialog emitter={node} signal={connecting.signal} onclose={() => (dialog = null)} />
{/if}

<style>
  .node-tab {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--bg-1);
  }
  .notice {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 16px 14px;
    font-size: var(--fs-12);
    color: var(--text-faint);
  }
  .notice strong {
    color: var(--text-dim);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 10px;
    font-size: var(--fs-12);
    color: var(--text);
    border-bottom: 1px solid var(--line-soft);
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }
  .kind {
    margin-left: auto;
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .empty-all {
    margin: 0;
    padding: 12px 10px;
    font-size: var(--fs-12);
    color: var(--text-faint);
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding-bottom: 8px;
  }
  .signal {
    border-bottom: 1px solid var(--line-soft);
  }
  .signal-head {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 4px 0 8px;
    background: var(--bg-toolbar);
  }
  .label {
    font-size: var(--fs-12);
    font-weight: 600;
    color: var(--text);
  }
  .code {
    font-size: var(--fs-11);
    color: var(--text-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .add,
  .remove {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: var(--r-ctl);
    color: var(--text-faint);
  }
  .signal-head .add {
    margin-left: auto;
  }
  .add:hover,
  .remove:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
  .empty {
    margin: 0;
    padding: 4px 10px 6px;
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 4px 6px 6px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .link {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 24px;
    padding: 0 2px 0 6px;
    border-radius: var(--r-ctl);
    font-size: var(--fs-12);
    color: var(--text);
  }
  .link:hover {
    background: var(--bg-hover);
  }
  .target {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .method {
    margin-left: auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-11);
    color: var(--text-dim);
  }
  .link.missing {
    color: var(--warning);
  }
  .link.missing .method {
    color: var(--warning);
  }
</style>
