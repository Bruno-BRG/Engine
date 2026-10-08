<script lang="ts">
  import type { Snippet } from 'svelte';
  let { title, children }: { title: string; children?: Snippet } = $props();
</script>

<div class="ph">
  <span>{title}</span>
  <div class="tools">{@render children?.()}</div>
</div>

<style>
  .ph {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 22px;
    padding: 0 3px 0 8px;
    background: var(--bg-toolbar);
    border-bottom: 1px solid var(--line);
  }
  span {
    font-size: var(--fs-12);
    color: var(--text);
  }
  .tools {
    display: flex;
    gap: 1px;
  }
  .tools :global(.icon-btn) {
    width: 18px;
    height: 18px;
  }
</style>
