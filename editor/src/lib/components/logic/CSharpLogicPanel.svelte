<script lang="ts">
  import { logic } from '$lib/logic/store.svelte';
  import { highlight } from '$lib/csharp';
  import { log } from '$lib/state/log.svelte';
  import { Braces, Copy, X } from '@lucide/svelte';

  // C# panel for the open script (VIS-024): the code generated from the IR, read-only, updated on every edit.
  let { onclose }: { onclose: () => void } = $props();

  const output = $derived(logic.csharp);
  const fileName = $derived(output ? `${output.className}.cs` : '');
  const lines = $derived(output?.ok ? output.code.split('\n').map(highlight) : []);
  const errors = $derived(output && !output.ok ? output.error.split('\n') : []);

  async function copyCode() {
    const current = output;
    if (!current || !current.ok) return;
    try {
      await navigator.clipboard.writeText(current.code);
      log.toast('success', 'Código copiado', fileName);
    } catch (e) {
      log.toast('error', 'Não foi possível copiar', e instanceof Error ? e.message : String(e));
    }
  }
</script>

<section class="panel">
  <header>
    <Braces size={13} />
    <span class="file mono" title={fileName}>{fileName || 'C# gerado'}</span>
    <span class="readonly">somente leitura · atualiza ao vivo</span>
    <button class="icon-btn" title="Copiar o código" aria-label="Copiar o código" disabled={!output?.ok} onclick={copyCode}>
      <Copy size={13} />
    </button>
    <button class="icon-btn" title="Fechar o painel de C#" aria-label="Fechar o painel de C#" onclick={onclose}>
      <X size={13} />
    </button>
  </header>

  <div class="body">
    {#if !output}
      <p class="empty">Abra um script para ver o C# gerado da lógica dele.</p>
    {:else if !output.ok}
      <div class="error">
        <strong>Não é possível gerar o C# com erros no grafo.</strong>
        <ul>
          {#each errors as line, i (i)}
            <li>{line}</li>
          {/each}
        </ul>
        <p class="empty">Corrija os erros do grafo: o código aparece assim que ele for válido.</p>
      </div>
    {:else}
      <pre class="mono">{#each lines as tokens, i}<span class="ln">{i + 1}</span>{#each tokens as token}<span class={token.t}>{token.v}</span>{/each}{'\n'}{/each}</pre>
    {/if}
  </div>
</section>

<style>
  .panel {
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
  .file {
    font-size: var(--fs-12);
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .readonly {
    margin-left: auto;
    font-size: var(--fs-11);
    color: var(--text-faint);
    white-space: nowrap;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .empty {
    margin: 0;
    padding: 12px;
    font-size: var(--fs-12);
    line-height: 1.5;
    color: var(--text-faint);
  }
  .error {
    padding: 12px;
    font-size: var(--fs-12);
    line-height: 1.5;
    color: var(--text);
  }
  .error strong {
    color: var(--danger);
    font-weight: 600;
  }
  .error ul {
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
