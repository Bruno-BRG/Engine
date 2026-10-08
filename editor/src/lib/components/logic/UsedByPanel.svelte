<script lang="ts">
  import { logic } from '$lib/logic/store.svelte';
  import { scene } from '$lib/state/scene.svelte';
  import { Link, Unlink } from '@lucide/svelte';

  // "Usado por" (VIS-003): os nós da cena com este script anexado. Anexa o nó selecionado na hierarquia.
  const selecionado = $derived(scene.selected);
  const podeAnexar = $derived(
    !!logic.active && !!selecionado && selecionado.props.script !== logic.active,
  );
</script>

<section class="painel">
  <header><span>Usado por</span><span class="contagem mono">{logic.usedBy.length}</span></header>
  <div class="corpo">
    {#if !logic.active}
      <p class="vazio">Abra um script para ver quais nós o usam.</p>
    {:else}
      <ul>
        {#each logic.usedBy as n (n.uid)}
          <li>
            <button class="no" title="Selecionar na cena" onclick={() => scene.select(n.uid)}>
              <span class="nome">{n.name}</span>
              <span class="tipo mono">{n.kind}</span>
            </button>
            <button class="icon-btn" title="Tirar o script deste nó" aria-label="Tirar o script de {n.name}" onclick={() => logic.detach(n.uid)}>
              <Unlink size={12} />
            </button>
          </li>
        {:else}
          <li class="vazio">Nenhum nó usa este script.</li>
        {/each}
      </ul>
      <button class="btn" disabled={!podeAnexar} onclick={() => logic.attachToSelected()}>
        <Link size={12} />
        {selecionado ? `Anexar a ${selecionado.name}` : 'Anexar ao nó selecionado'}
      </button>
      {#if !selecionado}
        <p class="vazio">Escolha um nó na hierarquia ou no viewport para anexar.</p>
      {/if}
    {/if}
  </div>
</section>

<style>
  .painel {
    display: flex;
    flex-direction: column;
    border-bottom: 1px solid var(--line);
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
    gap: 4px;
  }
  .no {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    padding: 0 6px;
    border-radius: var(--r-ctl);
    font-size: var(--fs-12);
    color: var(--text);
    text-align: left;
  }
  .no:hover {
    background: var(--bg-hover);
  }
  .nome {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tipo {
    font-size: var(--fs-11);
    color: var(--text-faint);
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
