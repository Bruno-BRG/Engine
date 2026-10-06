<script lang="ts">
  import { log, type LogLevel } from '$lib/state/log.svelte';
  import { Trash2, Info, CircleCheck, TriangleAlert, CircleAlert } from '@lucide/svelte';

  let filter = $state<'all' | LogLevel>('all');
  let list: HTMLDivElement;
  const icons = { info: Info, success: CircleCheck, warn: TriangleAlert, error: CircleAlert };
  const shown = $derived(
    filter === 'all' ? log.entries : log.entries.filter((e) => e.level === filter || (filter === 'info' && e.level === 'success')),
  );
  const count = (l: LogLevel) => log.entries.filter((e) => e.level === l).length;

  $effect(() => {
    shown.length;
    queueMicrotask(() => list && (list.scrollTop = list.scrollHeight));
  });
  const time = (d: Date) => d.toLocaleTimeString('pt-BR', { hour12: false });
</script>

<div class="console">
  <div class="bar">
    <button class:on={filter === 'all'} onclick={() => (filter = 'all')}>Tudo <em>{log.entries.length}</em></button>
    <button class:on={filter === 'info'} onclick={() => (filter = 'info')}>Info <em>{count('info') + count('success')}</em></button>
    <button class:on={filter === 'warn'} onclick={() => (filter = 'warn')}>Avisos <em>{count('warn')}</em></button>
    <button class:on={filter === 'error'} onclick={() => (filter = 'error')}>Erros <em>{count('error')}</em></button>
    <button class="icon-btn clear" title="Limpar" onclick={() => log.clear()}><Trash2 size={13} /></button>
  </div>
  <div class="list mono" bind:this={list}>
    {#each shown as e (e.id)}
      {@const Icon = icons[e.level]}
      <div class="line {e.level}">
        <span class="t">{time(e.time)}</span>
        <Icon size={12} />
        <span class="msg">{e.text}</span>
      </div>
    {:else}
      <div class="empty">Nenhuma mensagem.</div>
    {/each}
  </div>
</div>

<style>
  .console {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .bar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--line-soft);
  }
  .bar > button:not(.clear) {
    height: 24px;
    padding: 0 9px;
    border-radius: 99px;
    font-size: var(--fs-12);
    color: var(--text-dim);
    border: 1px solid var(--line-soft);
  }
  .bar > button.on {
    color: var(--text);
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .bar em {
    font-style: normal;
    color: var(--text-faint);
    margin-left: 3px;
  }
  .clear {
    margin-left: auto;
    width: 24px;
    height: 24px;
  }
  .list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
    font-size: 11.5px;
    user-select: text;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 3px 12px;
  }
  .line:hover {
    background: var(--bg-hover);
  }
  .t {
    color: var(--text-faint);
  }
  .msg {
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .info :global(svg) {
    color: var(--info);
  }
  .success :global(svg) {
    color: var(--success);
  }
  .warn :global(svg),
  .warn .msg {
    color: var(--warning);
  }
  .error :global(svg),
  .error .msg {
    color: var(--danger);
  }
  .error {
    background: rgb(255 92 122 / 0.06);
  }
  .empty {
    padding: 16px;
    color: var(--text-faint);
    font-family: var(--font-ui);
  }
</style>
