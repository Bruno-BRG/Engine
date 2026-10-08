<script lang="ts">
  import { valueFromText, type PinType, type Value } from '$lib/logic/model';
  import type { PropDef } from '$lib/state/scene.svelte';

  // Campo do valor embutido de uma entrada sem fio. Só para os tipos que se digitam (decimal, inteiro,
  // booleano, texto e vetor 2D). Os demais recebem o valor por fio.
  // On a property node, the registry property picks the field: options for an enum, a picker for a colour (VIS-021).
  let {
    ty,
    valor,
    onchange,
    prop,
    suggestions = [],
  }: {
    ty: PinType;
    valor: Value | undefined;
    onchange: (v: Value) => void;
    prop?: PropDef;
    /** Options offered under a text field. The field still takes any text (VIS-021). */
    suggestions?: { value: string; label: string }[];
  } = $props();
  const listId = $props.id();

  const pare = (e: Event) => e.stopPropagation();
  const enter = (e: KeyboardEvent) => {
    if (e.key === 'Enter') (e.currentTarget as HTMLElement).blur();
  };
  const numero = (v: Value | undefined) => {
    if (!v) return 0;
    if ('Float' in v) return v.Float;
    if ('Int' in v) return v.Int;
    return 0;
  };
  const texto = (v: Value | undefined) => (v && 'String' in v ? v.String : '');
  const vetor = (v: Value | undefined): [number, number] => (v && 'Vec2' in v ? v.Vec2 : [0, 0]);

  function envia(text: string) {
    const v = valueFromText(ty, text);
    if (v) onchange(v);
  }
</script>

{#if ty === 'Bool'}
  <input
    class="marca"
    type="checkbox"
    checked={!!(valor && 'Bool' in valor && valor.Bool)}
    onpointerdown={pare}
    onchange={(e) => onchange({ Bool: e.currentTarget.checked })}
    aria-label="Valor booleano"
  />
{:else if ty === 'Float' || ty === 'Int'}
  <input
    class="campo num"
    type="number"
    step={ty === 'Int' ? 1 : 0.1}
    value={numero(valor)}
    onpointerdown={pare}
    onkeydown={enter}
    onchange={(e) => envia(e.currentTarget.value)}
    aria-label="Valor"
  />
{:else if ty === 'String' && prop?.kind === 'enum'}
  <select
    class="campo wide"
    value={texto(valor)}
    onpointerdown={pare}
    onchange={(e) => onchange({ String: e.currentTarget.value })}
    aria-label="Opção"
  >
    {#each prop.options as option (option)}
      <option value={option}>{option}</option>
    {/each}
  </select>
{:else if ty === 'String' && prop?.kind === 'color'}
  <input
    class="color"
    type="color"
    value={texto(valor) || '#ffffff'}
    onpointerdown={pare}
    onchange={(e) => onchange({ String: e.currentTarget.value })}
    aria-label="Cor"
  />
{:else if ty === 'String'}
  <input
    class="campo"
    type="text"
    value={texto(valor)}
    list={suggestions.length ? listId : undefined}
    onpointerdown={pare}
    onkeydown={enter}
    onchange={(e) => envia(e.currentTarget.value)}
    aria-label="Texto"
  />
  {#if suggestions.length}
    <datalist id={listId}>
      {#each suggestions as s (s.value)}
        <option value={s.value} label={s.label}></option>
      {/each}
    </datalist>
  {/if}
{:else if ty === 'Vec2'}
  {@const par = vetor(valor)}
  <span class="par">
    <input
      class="campo num"
      type="number"
      step="1"
      value={par[0]}
      onpointerdown={pare}
      onkeydown={enter}
      onchange={(e) => onchange({ Vec2: [Number(e.currentTarget.value), par[1]] })}
      aria-label="X"
    />
    <input
      class="campo num"
      type="number"
      step="1"
      value={par[1]}
      onpointerdown={pare}
      onkeydown={enter}
      onchange={(e) => onchange({ Vec2: [par[0], Number(e.currentTarget.value)] })}
      aria-label="Y"
    />
  </span>
{/if}

<style>
  .campo {
    height: 18px;
    min-width: 0;
    width: 64px;
    padding: 0 4px;
    border-radius: var(--r-ctl);
    background: var(--bg-0);
    border: 1px solid var(--line);
    outline: none;
    font-family: var(--font-mono);
    font-size: var(--fs-11);
    color: var(--text);
    user-select: text;
  }
  .campo:focus {
    border-color: var(--accent);
  }
  .num {
    width: 46px;
  }
  .wide {
    width: 96px;
  }
  .par {
    display: inline-flex;
    gap: 2px;
  }
  .marca {
    margin: 0 2px;
  }
  .color {
    width: 46px;
    height: 18px;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: var(--r-ctl);
    background: var(--bg-0);
  }
</style>
