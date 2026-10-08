<script lang="ts">
  import { logic } from '$lib/logic/store.svelte';
  import { scene, type NodeView } from '$lib/state/scene.svelte';
  import { TIPO_NOME, textOf, type Variable } from '$lib/logic/model';
  import {
    asBool,
    asNode,
    asNumber,
    asText,
    asVec2,
    defaultOf,
    isOverridden,
    scriptVarsOf,
    valueOf,
    withValue,
    type StoredValue,
  } from '$lib/logic/exported';
  import NumberField from '../NumberField.svelte';
  import NodeRefField from '../inspector/NodeRefField.svelte';
  import { Workflow, ExternalLink, Plus, RotateCcw } from '@lucide/svelte';

  // Slot de script do Inspector (INS-005): anexa um script existente ou novo, abre no workspace de Lógica e
  // lista as variáveis exportadas dele. Cada variável pode ter um valor neste nó (VIS-022). Só este bloco mexe
  // com as propriedades `script` e `script_vars`.
  let { node }: { node: NodeView } = $props();

  const atual = $derived(String(node.props.script ?? ''));
  const valores = $derived(scriptVarsOf(node.props));
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

  /**
   * Troca o script do nó. Os valores eram das variáveis do script antigo, então saem junto: um só passo de
   * desfazer.
   */
  function anexa(nome: string) {
    if (nome === atual) return;
    scene.setProps(node.uid, { script: nome, script_vars: {} }, 'attach:' + node.uid);
    scene.endGroup();
  }

  /**
   * Grava o valor de uma variável neste nó. Com `coalesce`, alterações seguidas (arrastar um número) viram um passo.
   */
  function setVar(variable: Variable, value: StoredValue, coalesce = false) {
    scene.setProp(
      node.uid,
      'script_vars',
      withValue(valores, variable, value),
      coalesce ? 'vars:' + node.uid + ':' + variable.name : undefined,
    );
  }

  function endEdit() {
    scene.endGroup();
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
            {@const valor = valueOf(v, valores)}
            {@const mudou = isOverridden(v, valores)}
            <li class="var" class:mudou title="{TIPO_NOME[v.ty]}, padrão do script: {textOf(v.default)}">
              <div class="corpo">
                {#if v.ty === 'Float' || v.ty === 'Int'}
                  <NumberField
                    label={v.name}
                    value={asNumber(valor)}
                    step={v.ty === 'Int' ? 1 : 0.1}
                    precision={v.ty === 'Int' ? 0 : 2}
                    onchange={(x) => setVar(v, v.ty === 'Int' ? { Int: Math.round(x) } : { Float: x }, true)}
                    onend={endEdit}
                  />
                {:else if v.ty === 'Bool'}
                  <label class="check">
                    <input
                      type="checkbox"
                      checked={asBool(valor)}
                      onchange={(e) => setVar(v, { Bool: e.currentTarget.checked })}
                    />
                    <span>{v.name}</span>
                  </label>
                {:else if v.ty === 'String'}
                  <label class="linha">
                    <span class="flabel">{v.name}</span>
                    <input
                      class="txt"
                      value={asText(valor)}
                      onchange={(e) => setVar(v, { String: e.currentTarget.value })}
                    />
                  </label>
                {:else if v.ty === 'Vec2'}
                  {@const par = asVec2(valor)}
                  <div class="vec">
                    <span class="flabel">{v.name}</span>
                    <NumberField
                      label="X"
                      value={par[0]}
                      step={1}
                      precision={0}
                      onchange={(x) => setVar(v, { Vec2: [x, par[1]] }, true)}
                      onend={endEdit}
                    />
                    <NumberField
                      label="Y"
                      value={par[1]}
                      step={1}
                      precision={0}
                      onchange={(y) => setVar(v, { Vec2: [par[0], y] }, true)}
                      onend={endEdit}
                    />
                  </div>
                {:else if v.ty === 'Node'}
                  <NodeRefField
                    label={v.name}
                    value={asNode(valor)}
                    anyNode
                    emptyLabel="Nenhum"
                    onchange={(uid) => setVar(v, { Node: uid })}
                  />
                {/if}
              </div>
              {#if mudou}
                <button
                  class="reset"
                  title="Voltar ao padrão do script ({textOf(v.default)})"
                  aria-label="Voltar {v.name} ao padrão do script"
                  onclick={() => setVar(v, defaultOf(v))}
                >
                  <RotateCcw size={11} />
                </button>
              {/if}
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
    gap: 4px;
  }
  .var {
    display: flex;
    align-items: flex-start;
    gap: 4px;
    min-width: 0;
  }
  .corpo {
    flex: 1;
    min-width: 0;
  }
  .var.mudou :global(.lbl),
  .var.mudou .flabel {
    font-weight: 600;
    color: var(--text);
  }
  /* Campos de texto e vetor usam a mesma coluna de rótulo que os campos numéricos do Inspector. */
  .linha {
    display: grid;
    grid-template-columns: 84px minmax(0, 1fr);
    align-items: center;
    gap: 6px;
  }
  .vec {
    display: grid;
    grid-template-columns: 84px minmax(0, 1fr) minmax(0, 1fr);
    gap: 4px;
    align-items: center;
  }
  .flabel {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-11);
    color: var(--text-dim);
  }
  .check {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-11);
    color: var(--text-dim);
    padding-left: 90px;
  }
  .txt {
    min-width: 0;
    height: 20px;
    padding: 0 4px;
    border-radius: var(--r-ctl);
    background: var(--bg-2);
    color: var(--text);
    font-size: var(--fs-12);
    box-shadow: inset 0 0 0 1px var(--line);
    outline: none;
  }
  .txt:focus {
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .reset {
    flex: none;
    margin-top: 2px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: var(--r-ctl);
    color: var(--text-faint);
  }
  .reset:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
  p {
    margin: 0;
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
</style>
