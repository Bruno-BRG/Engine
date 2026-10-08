<script lang="ts">
  import { logic } from '$lib/logic/store.svelte';
  import { scene, type NodeView } from '$lib/state/scene.svelte';
  import { TIPO_NOME, textOf, type Variable } from '$lib/logic/model';
  import { Workflow, ExternalLink, Plus } from '@lucide/svelte';

  // Slot de script do Inspector (INS-005): anexa um script existente ou novo, abre no workspace de Lógica e
  // lista as variáveis exportadas dele. Só este bloco mexe com a propriedade `script`.
  let { node }: { node: NodeView } = $props();

  const atual = $derived(String(node.props.script ?? ''));
  let exportadas = $state<Variable[]>([]);

  $effect(() => {
    const nome = atual;
    // Com o script aberto, a lista acompanha as edições; os demais são lidos do disco.
    if (nome && logic.active === nome) void logic.snap;
    if (!nome) {
      exportadas = [];
      return;
    }
    let cancelado = false;
    logic
      .exportedOf(nome)
      .then((v) => {
        if (!cancelado) exportadas = v;
      })
      .catch(() => {
        if (!cancelado) exportadas = [];
      });
    return () => {
      cancelado = true;
    };
  });

  function anexa(nome: string) {
    scene.setProp(node.uid, 'script', nome);
  }

  async function novo() {
    const nome = await logic.create();
    if (nome) anexa(nome);
  }
</script>

<div class="slot">
  <label class="campo">
    <span class="rotulo">Script</span>
    <select class="sel" value={atual} onchange={(e) => anexa(e.currentTarget.value)} aria-label="Script do nó">
      <option value="">Nenhum</option>
      {#each logic.names as nome (nome)}
        <option value={nome}>{nome}</option>
      {/each}
      {#if atual && !logic.names.includes(atual)}
        <option value={atual}>{atual} (arquivo ausente)</option>
      {/if}
    </select>
  </label>

  <div class="acoes">
    <button class="btn" disabled={!logic.project} onclick={novo} title="Cria um script vazio e anexa a este nó">
      <Plus size={12} /> Novo
    </button>
    <button class="btn" disabled={!atual || !logic.project} onclick={() => logic.openIn(atual)} title="Abre o script no workspace de Lógica">
      <ExternalLink size={12} /> Abrir no Lógica
    </button>
  </div>

  {#if atual}
    <div class="exportadas">
      <div class="cab"><Workflow size={11} /> Variáveis exportadas</div>
      {#if exportadas.length}
        <ul>
          {#each exportadas as v (v.name)}
            <li>
              <span class="mono nome">{v.name}</span>
              <span class="tipo">{TIPO_NOME[v.ty]}</span>
              <span class="mono valor">{textOf(v.default)}</span>
            </li>
          {/each}
        </ul>
      {:else}
        <p>Nenhuma variável exportada neste script.</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .slot {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 6px 12px 8px;
    border-bottom: 1px solid var(--line-soft);
  }
  .campo {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .rotulo {
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .sel {
    height: 22px;
    padding: 0 4px;
    border-radius: var(--r-ctl);
    background: var(--bg-2);
    border: 1px solid var(--line);
    font-size: var(--fs-12);
    color: var(--text);
    outline: none;
  }
  .sel:focus {
    border-color: var(--accent);
  }
  .acoes {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }
  .acoes .btn {
    height: 22px;
    padding: 0 7px;
  }
  .exportadas {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .cab {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: var(--fs-11);
    color: var(--text-dim);
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
    align-items: baseline;
    gap: 6px;
    font-size: var(--fs-12);
  }
  .nome {
    color: var(--text);
  }
  .tipo {
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .valor {
    margin-left: auto;
    font-size: var(--fs-11);
    color: var(--text-dim);
  }
  p {
    margin: 0;
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
</style>
