<script lang="ts">
  // Picker for the type of a property node (VIS-021). The value is what the node stores, even when it is not in
  // the list, so the user sees what to fix.
  let {
    selected,
    options,
    emptyLabel,
    onchange,
  }: {
    selected: string;
    options: { value: string; label: string }[];
    /** Text of the empty option ("no type"). */
    emptyLabel: string;
    onchange: (v: string) => void;
  } = $props();

  const stopPointer = (e: Event) => e.stopPropagation();
  const list = $derived(
    selected && !options.some((o) => o.value === selected)
      ? [...options, { value: selected, label: selected }]
      : options,
  );
</script>

<select
  class="campo"
  value={selected}
  onpointerdown={stopPointer}
  onchange={(e) => onchange(e.currentTarget.value)}
  aria-label={emptyLabel}
>
  <option value="">{emptyLabel}</option>
  {#each list as o (o.value)}
    <option value={o.value}>{o.label}</option>
  {/each}
</select>

<style>
  .campo {
    height: 18px;
    min-width: 0;
    flex: 0 1 110px;
    max-width: 110px;
    padding: 0 2px;
    border-radius: var(--r-ctl);
    background: var(--bg-0);
    border: 1px solid var(--line);
    outline: none;
    font-family: var(--font-mono);
    font-size: var(--fs-11);
    color: var(--text);
  }
  .campo:focus {
    border-color: var(--accent);
  }
</style>
