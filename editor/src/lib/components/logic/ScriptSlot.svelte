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

  // Script slot of the Inspector (INS-005): attaches an existing or a new script, opens it in the Logic workspace and
  // lists its exported variables. Each variable can have its own value on this node (VIS-022). Only this block writes
  // the `script` and `script_vars` properties.
  let { node }: { node: NodeView } = $props();

  const current = $derived(String(node.props.script ?? ''));
  const stored = $derived(scriptVarsOf(node.props));
  let exported = $state<Variable[]>([]);

  $effect(() => {
    const name = current;
    // Com o script aberto, a lista acompanha as edições; os demais são lidos do disco.
    if (name && logic.active === name) void logic.snap;
    if (!name) {
      exported = [];
      return;
    }
    let cancelled = false;
    logic
      .exportedOf(name)
      .then((v) => {
        if (!cancelled) exported = v;
      })
      .catch(() => {
        if (!cancelled) exported = [];
      });
    return () => {
      cancelled = true;
    };
  });

  /**
   * Switches the node to another script. The values belonged to the old script's variables, so they leave with it,
   * as one undo step. Choosing the script that is already attached does nothing.
   */
  function attach(name: string) {
    if (name === current) return;
    scene.setProps(node.uid, { script: name, script_vars: {} }, 'attach:' + node.uid);
    scene.endGroup();
  }

  /**
   * Stores a variable's value on this node. With `coalesce`, consecutive changes (dragging a number) become one step.
   */
  function setVariable(variable: Variable, value: StoredValue, coalesce = false) {
    scene.setProp(
      node.uid,
      'script_vars',
      withValue(stored, variable, value),
      coalesce ? 'vars:' + node.uid + ':' + variable.name : undefined,
    );
  }

  function endEdit() {
    scene.endGroup();
  }

  /** Default shown in the reset tooltip. A node default is shown from its hex text, which keeps every digit. */
  function defaultText(variable: Variable): string {
    if (variable.ty !== 'Node') return textOf(variable.default);
    return `nó ${variable.default_node || 0}`;
  }

  async function createScript() {
    const name = await logic.create();
    if (name) attach(name);
  }
</script>

<div class="slot">
  <label class="campo">
    <span class="rotulo">Script</span>
    <select class="sel" value={current} onchange={(e) => attach(e.currentTarget.value)} aria-label="Script do nó">
      <option value="">Nenhum</option>
      {#each logic.names as name (name)}
        <option value={name}>{name}</option>
      {/each}
      {#if current && !logic.names.includes(current)}
        <option value={current}>{current} (arquivo ausente)</option>
      {/if}
    </select>
  </label>

  <div class="acoes">
    <button class="btn" disabled={!logic.project} onclick={createScript} title="Cria um script vazio e anexa a este nó">
      <Plus size={12} /> Novo
    </button>
    <button class="btn" disabled={!current || !logic.project} onclick={() => logic.openIn(current)} title="Abre o script no workspace de Lógica">
      <ExternalLink size={12} /> Abrir no Lógica
    </button>
  </div>

  {#if current}
    <div class="exportadas">
      <div class="cab"><Workflow size={11} /> Variáveis exportadas</div>
      {#if exported.length}
        <ul>
          {#each exported as v (v.name)}
            {@const shown = valueOf(v, stored)}
            {@const overridden = isOverridden(v, stored)}
            <li class="var" class:overridden title="{TIPO_NOME[v.ty]}, padrão do script: {defaultText(v)}">
              <div class="corpo">
                {#if v.ty === 'Float' || v.ty === 'Int'}
                  <NumberField
                    label={v.name}
                    value={asNumber(shown)}
                    step={v.ty === 'Int' ? 1 : 0.1}
                    precision={v.ty === 'Int' ? 0 : 2}
                    onchange={(x) => setVariable(v, v.ty === 'Int' ? { Int: Math.round(x) } : { Float: x }, true)}
                    onend={endEdit}
                  />
                {:else if v.ty === 'Bool'}
                  <label class="check">
                    <input
                      type="checkbox"
                      checked={asBool(shown)}
                      onchange={(e) => setVariable(v, { Bool: e.currentTarget.checked })}
                    />
                    <span>{v.name}</span>
                  </label>
                {:else if v.ty === 'String'}
                  <label class="linha">
                    <span class="flabel">{v.name}</span>
                    <input
                      class="txt"
                      value={asText(shown)}
                      onchange={(e) => setVariable(v, { String: e.currentTarget.value })}
                    />
                  </label>
                {:else if v.ty === 'Vec2'}
                  {@const pair = asVec2(shown)}
                  <div class="vec">
                    <span class="flabel">{v.name}</span>
                    <NumberField
                      label="X"
                      value={pair[0]}
                      step={0.1}
                      precision={2}
                      onchange={(x) => setVariable(v, { Vec2: [x, pair[1]] }, true)}
                      onend={endEdit}
                    />
                    <NumberField
                      label="Y"
                      value={pair[1]}
                      step={0.1}
                      precision={2}
                      onchange={(y) => setVariable(v, { Vec2: [pair[0], y] }, true)}
                      onend={endEdit}
                    />
                  </div>
                {:else if v.ty === 'Node'}
                  <NodeRefField
                    label={v.name}
                    value={asNode(shown)}
                    anyNode
                    emptyLabel="Nenhum"
                    onchange={(uid) => setVariable(v, { Node: uid })}
                  />
                {/if}
              </div>
              {#if overridden}
                <button
                  class="reset"
                  title="Voltar ao padrão do script ({defaultText(v)})"
                  aria-label="Voltar {v.name} ao padrão do script"
                  onclick={() => setVariable(v, defaultOf(v))}
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
  .var.overridden :global(.lbl),
  .var.overridden .flabel {
    font-weight: 600;
    color: var(--text);
  }
  /* Text and vector fields use the same label column as the number fields in the Inspector. */
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
