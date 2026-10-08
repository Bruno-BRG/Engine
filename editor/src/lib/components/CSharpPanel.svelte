<script lang="ts">
  import { scene } from '$lib/state/scene.svelte';
  import { log } from '$lib/state/log.svelte';
  import { nodeToCSharp, sceneToCSharp, highlight, className } from '$lib/csharp';
  import { Copy, Braces } from '@lucide/svelte';

  const code = $derived(scene.selected ? nodeToCSharp(scene.selected, scene.data.nodes) : sceneToCSharp(scene.data));
  const file = $derived((scene.selected ? className(scene.selected.name) : className(scene.data.name) + 'Scene') + '.cs');
  const lines = $derived(code.split('\n').map(highlight));

  async function copy() {
    await navigator.clipboard.writeText(code);
    log.toast('success', 'Código copiado', file);
  }
</script>

<div class="cs">
  <div class="bar">
    <Braces size={13} />
    <span class="mono file">{file}</span>
    <span class="ro">somente leitura · atualiza ao vivo</span>
    <button class="btn ghost" onclick={copy}><Copy size={13} /> Copiar</button>
  </div>
  <pre class="mono">{#each lines as toks, i}<span class="ln">{i + 1}</span>{#each toks as tk}<span class={tk.t}>{tk.v}</span>{/each}{'\n'}{/each}</pre>
</div>

<style>
  .cs {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .bar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 6px 0 12px;
    border-bottom: 1px solid var(--line-soft);
    color: var(--accent-hi);
  }
  .file {
    font-size: var(--fs-12);
    color: var(--text);
  }
  .ro {
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .bar .btn {
    margin-left: auto;
    height: 26px;
    font-size: var(--fs-12);
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
    width: 44px;
    padding-right: 14px;
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
