<script lang="ts">
  import { logic } from '$lib/logic/store.svelte';
  import { highlight } from '$lib/csharp';
  import { log } from '$lib/state/log.svelte';
  import { Braces, Copy, ChevronDown, ChevronRight } from '@lucide/svelte';

  // C# do script aberto, gerado da IR (VIS-024). Só leitura: o store gera de novo a cada alteração do grafo.
  let recolhido = $state(false);

  const saida = $derived(logic.csharp);
  const arquivo = $derived(saida?.ok ? saida.classe + '.cs' : '');
  const linhas = $derived(saida?.ok ? saida.codigo.split('\n').map(highlight) : []);

  async function copia() {
    if (!saida?.ok) return;
    try {
      await navigator.clipboard.writeText(saida.codigo);
      log.toast('success', 'Código copiado', arquivo);
    } catch {
      log.toast('error', 'Não foi possível copiar', arquivo);
    }
  }
</script>

<section class="painel">
  <header>
    <button class="titulo" onclick={() => (recolhido = !recolhido)} aria-expanded={!recolhido}>
      {#if recolhido}<ChevronRight size={12} />{:else}<ChevronDown size={12} />{/if}
      <span>C#</span>
    </button>
    {#if saida?.ok && !recolhido}
      <button class="icon-btn" title="Copiar C#" aria-label="Copiar C#" onclick={copia}>
        <Copy size={12} />
      </button>
    {/if}
  </header>

  {#if !recolhido}
    <div class="corpo">
      {#if !logic.active || !saida}
        <p class="vazio">Abra um script para ver o C# gerado dele.</p>
      {:else if saida.ok}
        <span class="arquivo mono"><Braces size={11} /> {arquivo} · somente leitura</span>
        <pre class="mono">{#each linhas as toks}{#each toks as tk}<span class={tk.t}>{tk.v}</span>{/each}{'\n'}{/each}</pre>
      {:else}
        <p class="vazio">Corrija os erros do script para ver o C# dele.</p>
        <p class="erro mono">{saida.erro}</p>
      {/if}
    </div>
  {/if}
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
    padding: 0 6px 0 4px;
    background: var(--bg-toolbar);
    border-bottom: 1px solid var(--line);
  }
  .titulo {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: var(--fs-12);
    color: var(--text);
  }
  .corpo {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 6px;
    min-width: 0;
  }
  .arquivo {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  pre {
    margin: 0;
    max-height: 360px;
    overflow: auto;
    padding: 6px;
    border-radius: var(--r-ctl);
    background: var(--bg-2);
    border: 1px solid var(--line);
    font-size: var(--fs-11);
    line-height: 1.6;
    color: var(--text);
    user-select: text;
    tab-size: 4;
  }
  .kw {
    color: var(--code-keyword);
  }
  .type {
    color: var(--code-type);
  }
  .fn {
    color: var(--code-fn);
  }
  .str {
    color: var(--code-string);
  }
  .num {
    color: var(--code-number);
  }
  .com {
    color: var(--text-faint);
    font-style: italic;
  }
  .txt {
    color: var(--text);
  }
  .vazio {
    font-size: var(--fs-12);
    color: var(--text-faint);
    line-height: 1.5;
  }
  .erro {
    font-size: var(--fs-11);
    color: var(--danger);
    white-space: pre-wrap;
    user-select: text;
  }
</style>
