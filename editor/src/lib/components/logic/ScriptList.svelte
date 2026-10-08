<script lang="ts">
  import { logic } from '$lib/logic/store.svelte';
  import { showContextMenu } from '$lib/state/contextmenu.svelte';
  import { log } from '$lib/state/log.svelte';
  import { invalidName } from '$lib/components/project/names';
  import { Plus, Workflow } from '@lucide/svelte';

  // Lista de scripts do projeto (VIS-001): criar, abrir, renomear, duplicar e excluir.
  let renomeando = $state<string | null>(null);
  let rascunho = $state('');

  function comecaRenomear(nome: string) {
    renomeando = nome;
    rascunho = nome;
  }

  async function confirma() {
    const de = renomeando;
    const para = rascunho.trim();
    renomeando = null;
    if (!de || !para || para === de) return;
    const erro = invalidName(para);
    if (erro) {
      log.toast('error', 'Nome inválido', erro);
      return;
    }
    if (logic.names.some((n) => n !== de && n.toLowerCase() === para.toLowerCase())) {
      log.toast('error', 'Já existe um script com esse nome', para);
      return;
    }
    await logic.rename(de, para);
  }

  function menu(e: MouseEvent, nome: string) {
    showContextMenu(e, [
      { label: 'Abrir', run: () => void logic.open(nome) },
      { label: 'Renomear', run: () => comecaRenomear(nome) },
      { label: 'Duplicar', run: () => void logic.duplicate(nome) },
      'sep',
      { label: 'Excluir', run: () => void logic.remove(nome) },
    ]);
  }

  function foco(node: HTMLElement) {
    node.focus();
    (node as HTMLInputElement).select();
  }
</script>

<section class="lista" aria-label="Scripts do projeto">
  <header>
    <span>Scripts</span>
    <button class="icon-btn" title="Novo script" aria-label="Novo script" disabled={!logic.project} onclick={() => logic.create()}>
      <Plus size={13} />
    </button>
  </header>
  <ul>
    {#each logic.names as nome (nome)}
      <li>
        {#if renomeando === nome}
          <input
            class="ren"
            bind:value={rascunho}
            use:foco
            aria-label="Novo nome do script"
            onblur={confirma}
            onkeydown={(e) => {
              if (e.key === 'Enter') (e.currentTarget as HTMLInputElement).blur();
              if (e.key === 'Escape') {
                renomeando = null;
                (e.currentTarget as HTMLInputElement).blur();
              }
            }}
          />
        {:else}
          <button
            class="item"
            class:ativo={logic.active === nome}
            title="{nome}: duplo clique para renomear"
            onclick={() => logic.open(nome)}
            ondblclick={() => comecaRenomear(nome)}
            oncontextmenu={(e) => menu(e, nome)}
          >
            <Workflow size={12} />
            <span>{nome}</span>
          </button>
        {/if}
      </li>
    {:else}
      <li class="vazio">Nenhum script ainda. Crie um com +.</li>
    {/each}
  </ul>
</section>

<style>
  .lista {
    display: flex;
    flex-direction: column;
    min-height: 0;
    height: 100%;
    background: var(--bg-1);
  }
  header {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 22px;
    padding: 0 3px 0 8px;
    background: var(--bg-toolbar);
    border-bottom: 1px solid var(--line);
    font-size: var(--fs-12);
    color: var(--text);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 4px;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    height: 22px;
    padding: 0 6px;
    border-radius: var(--r-ctl);
    font-size: var(--fs-12);
    color: var(--text-dim);
    text-align: left;
  }
  .item span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .item:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .item.ativo {
    background: var(--accent-soft);
    color: var(--text);
  }
  .ren {
    width: 100%;
    height: 22px;
    padding: 0 6px;
    border-radius: var(--r-ctl);
    background: var(--bg-2);
    border: 1px solid var(--accent);
    outline: none;
    font-size: var(--fs-12);
    user-select: text;
  }
  .vazio {
    padding: 8px 6px;
    font-size: var(--fs-12);
    color: var(--text-faint);
  }
</style>
