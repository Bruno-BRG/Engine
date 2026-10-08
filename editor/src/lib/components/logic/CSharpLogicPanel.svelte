<script lang="ts">
  import { logic } from '$lib/logic/store.svelte';
  import { highlight } from '$lib/csharp';
  import { log } from '$lib/state/log.svelte';
  import { Braces, Copy, X } from '@lucide/svelte';

  // Painel de C# do script aberto (VIS-024): o código gerado da IR, somente leitura, que se atualiza a cada edição.
  let { onclose }: { onclose: () => void } = $props();

  const saida = $derived(logic.csharp);
  const arquivo = $derived(saida ? `${saida.classe}.cs` : '');
  const linhas = $derived(saida?.ok ? saida.codigo.split('\n').map(highlight) : []);
  const erros = $derived(saida && !saida.ok ? saida.erro.split('\n') : []);

  async function copia() {
    const atual = saida;
    if (!atual || !atual.ok) return;
    try {
      await navigator.clipboard.writeText(atual.codigo);
      log.toast('success', 'Código copiado', arquivo);
    } catch (e) {
      log.toast('error', 'Não foi possível copiar', e instanceof Error ? e.message : String(e));
    }
  }
</script>

<section class="painel">
  <header>
    <Braces size={13} />
    <span class="arquivo mono" title={arquivo}>{arquivo || 'C# gerado'}</span>
    <span class="ro">somente leitura · atualiza ao vivo</span>
    <button class="icon-btn" title="Copiar o código" aria-label="Copiar o código" disabled={!saida?.ok} onclick={copia}>
      <Copy size={13} />
    </button>
    <button class="icon-btn" title="Fechar o painel de C#" aria-label="Fechar o painel de C#" onclick={onclose}>
      <X size={13} />
    </button>
  </header>

  <div class="corpo">
    {#if !saida}
      <p class="vazio">Abra um script para ver o C# gerado da lógica dele.</p>
    {:else if !saida.ok}
      <div class="erro">
        <strong>Não é possível gerar o C# com erros no grafo.</strong>
        <ul>
          {#each erros as linha, i (i)}
            <li>{linha}</li>
          {/each}
        </ul>
        <p class="vazio">Corrija os erros do grafo: o código aparece assim que ele for válido.</p>
      </div>
    {:else}
      <pre class="mono">{#each linhas as toks, i}<span class="ln">{i + 1}</span>{#each toks as tk}<span class={tk.t}>{tk.v}</span>{/each}{'\n'}{/each}</pre>
    {/if}
  </div>
</section>

<style>
  .painel {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  header {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 6px 0 10px;
    background: var(--bg-toolbar);
    border-bottom: 1px solid var(--line);
    color: var(--accent-hi);
    min-width: 0;
  }
  .arquivo {
    font-size: var(--fs-12);
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .ro {
    margin-left: auto;
    font-size: var(--fs-11);
    color: var(--text-faint);
    white-space: nowrap;
  }
  .corpo {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .vazio {
    margin: 0;
    padding: 12px;
    font-size: var(--fs-12);
    line-height: 1.5;
    color: var(--text-faint);
  }
  .erro {
    padding: 12px;
    font-size: var(--fs-12);
    line-height: 1.5;
    color: var(--text);
  }
  .erro strong {
    color: var(--danger);
    font-weight: 600;
  }
  .erro ul {
    margin: 8px 0;
    padding-left: 18px;
    color: var(--text-dim);
  }
  pre {
    flex: 1;
    margin: 0;
    overflow: auto;
    padding: 10px 0;
    font-size: var(--fs-12);
    line-height: 1.6;
    user-select: text;
    tab-size: 4;
  }
  .ln {
    display: inline-block;
    width: 40px;
    padding-right: 12px;
    text-align: right;
    color: var(--text-faint);
    user-select: none;
    opacity: 0.6;
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
</style>
