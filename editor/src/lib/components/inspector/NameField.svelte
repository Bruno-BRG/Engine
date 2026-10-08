<script lang="ts">
  // Campo de texto que nomeia algo do projeto (tag, camada, barramento, material): um dropdown com os nomes
  // das Configurações do Projeto (SCN-033, AUD-001, PHY-023). Um nome que não existe mais continua na lista,
  // com aviso, para o nó não perder o valor. Escolher outro nome da lista corrige.
  import { TriangleAlert } from '@lucide/svelte';

  let {
    label,
    value,
    options,
    labels = {},
    emptyLabel = null,
    compact = false,
    onchange,
  }: {
    label: string;
    value: string;
    options: string[];
    /** Rótulos em português para nomes conhecidos (ex.: "untagged" vira "Sem tag"). Só mudam o texto. */
    labels?: Record<string, string>;
    /** Quando não é nulo, a lista ganha uma opção vazia com este rótulo (ex.: "Nenhum" para material). */
    emptyLabel?: string | null;
    compact?: boolean;
    onchange: (name: string) => void;
  } = $props();

  const base = $derived(emptyLabel !== null ? ['', ...options] : options);
  const missing = $derived(value !== '' && !options.includes(value));
  const list = $derived(missing ? [...base, value] : base);

  function text(v: string) {
    if (v === '' && emptyLabel !== null) return emptyLabel;
    return labels[v] ?? v;
  }
</script>

<div class="campo" class:compact>
  <span class="flabel">{label}</span>
  <select
    class="sel"
    class:warn={missing}
    aria-label={label}
    aria-invalid={missing}
    value={value}
    onchange={(e) => onchange(e.currentTarget.value)}
  >
    {#each list as o (o)}
      <option value={o}>{text(o)}{missing && o === value ? ' (não existe)' : ''}</option>
    {/each}
  </select>
  {#if missing}
    <p class="aviso" role="status"><TriangleAlert size={11} /> "{value}" não existe nas Configurações do Projeto.</p>
  {/if}
</div>

<style>
  .campo {
    display: grid;
    grid-template-columns: 84px minmax(0, 1fr);
    align-items: start;
    gap: 6px;
  }
  .campo.compact {
    grid-template-columns: 30px minmax(0, 1fr);
  }
  .flabel {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-11);
    color: var(--text-dim);
  }
  .sel {
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
  .sel:focus {
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .sel.warn {
    box-shadow: inset 0 0 0 1px var(--warning);
  }
  .aviso {
    grid-column: 2;
    display: flex;
    align-items: center;
    gap: 4px;
    margin: 0;
    font-size: var(--fs-11);
    color: var(--warning);
  }
</style>
