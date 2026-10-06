<script lang="ts">
  import { app } from '$lib/state/app.svelte';
  import { paletteCommands, fuzzyScore, type Command } from '$lib/commands';
  import { Search, CornerDownLeft } from '@lucide/svelte';

  let query = $state('');
  let index = $state(0);
  let input: HTMLInputElement;
  let list: HTMLDivElement;

  const results = $derived.by(() => {
    const all = paletteCommands().filter((c) => !c.enabled || c.enabled());
    if (!query.trim()) return all;
    return all
      .map((c) => ({ c, s: Math.max(fuzzyScore(query, c.title), fuzzyScore(query, c.group + ' ' + c.title) - 50) }))
      .filter((r) => r.s >= 0)
      .sort((a, b) => b.s - a.s)
      .map((r) => r.c);
  });

  $effect(() => {
    query;
    index = 0;
  });
  $effect(() => {
    input?.focus();
  });
  $effect(() => {
    list?.querySelector('[data-active="true"]')?.scrollIntoView({ block: 'nearest' });
  });

  function close() {
    app.paletteOpen = false;
  }
  function run(c: Command) {
    close();
    queueMicrotask(c.run);
  }
  function onkey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      index = Math.min(index + 1, results.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      index = Math.max(index - 1, 0);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (results[index]) run(results[index]);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      close();
    }
  }
  const keyParts = (k: string) => k.split('+');
</script>

<div class="backdrop" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && close()}>
  <div class="palette" role="dialog" aria-label="Paleta de comandos">
    <div class="input">
      <Search size={16} />
      <input bind:this={input} bind:value={query} onkeydown={onkey} placeholder="O que você quer fazer?" spellcheck="false" />
      <kbd>Esc</kbd>
    </div>
    <div class="list" bind:this={list}>
      {#each results as c, i (c.id)}
        {@const showGroup = !query.trim() && (i === 0 || results[i - 1].group !== c.group)}
        {#if showGroup}<div class="group">{c.group}</div>{/if}
        <button data-active={i === index} class:active={i === index} onpointermove={() => (index = i)} onclick={() => run(c)}>
          <span class="title">{c.title}</span>
          {#if query.trim()}<span class="grp">{c.group}</span>{/if}
          {#if c.keys}
            <span class="keys">{#each keyParts(c.keys) as k}<kbd>{k}</kbd>{/each}</span>
          {/if}
          {#if i === index}<CornerDownLeft size={13} class="enter" />{/if}
        </button>
      {:else}
        <div class="empty">Nenhum comando encontrado para “{query}”.</div>
      {/each}
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 150;
    background: rgb(5 6 10 / 0.45);
    backdrop-filter: blur(3px);
    display: flex;
    justify-content: center;
    padding-top: 12vh;
    animation: fade var(--t-med) var(--ease);
  }
  .palette {
    width: min(620px, calc(100vw - 32px));
    max-height: 60vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-2);
    border-radius: var(--r-dlg);
    box-shadow: var(--shadow-pop);
    overflow: hidden;
    animation: rise 240ms var(--ease);
    align-self: flex-start;
  }
  .input {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 14px;
    height: 52px;
    border-bottom: 1px solid var(--line);
    color: var(--text-dim);
  }
  .input input {
    flex: 1;
    height: 100%;
    background: none;
    border: none;
    outline: none;
    font-size: var(--fs-15);
    color: var(--text);
  }
  .input input::placeholder {
    color: var(--text-faint);
  }
  .list {
    overflow-y: auto;
    padding: 6px;
  }
  .group {
    padding: 10px 10px 4px;
    font-size: var(--fs-11);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-faint);
  }
  .list button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 36px;
    padding: 0 10px;
    border-radius: 8px;
    text-align: left;
    color: var(--text);
  }
  .list button.active {
    background: var(--accent-soft);
  }
  .title {
    flex: 1;
  }
  .grp {
    color: var(--text-faint);
    font-size: var(--fs-12);
  }
  .keys {
    display: flex;
    gap: 3px;
  }
  .list :global(.enter) {
    color: var(--accent-hi);
  }
  .empty {
    padding: 28px;
    text-align: center;
    color: var(--text-dim);
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(-8px) scale(0.985);
    }
  }
</style>
