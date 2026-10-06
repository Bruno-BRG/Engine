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
    height: 38px;
    padding: 0 8px 0 14px;
    border-bottom: 1px solid var(--line-soft);
  }
  span {
    font-size: var(--fs-12);
    font-weight: 600;
    color: var(--text-dim);
    letter-spacing: 0.01em;
  }
  .tools {
    display: flex;
    gap: 2px;
  }
</style>
