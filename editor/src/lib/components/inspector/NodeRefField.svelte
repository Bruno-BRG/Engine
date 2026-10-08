<script lang="ts">
  // Campo que aponta para outro nó da cena pelo UID (Corpo B das juntas, PHY-031). Lista os corpos da cena,
  // sem o próprio nó. Vazio é um ponto fixo do mundo. Um nó que não é corpo, ou que sumiu, aparece com aviso.
  import { scene } from '$lib/state/scene.svelte';
  import { TriangleAlert } from '@lucide/svelte';

  let {
    label,
    value,
    self,
    onchange,
  }: {
    label: string;
    value: string;
    /** UID do nó que está sendo editado: não entra na lista. */
    self: string;
    onchange: (uid: string) => void;
  } = $props();

  // Os tipos que a física monta como corpo rígido (nexo-physics). Junta só aceita estes como corpo B.
  const BODIES = ['StaticBody2D', 'RigidBody2D', 'CharacterBody2D'];

  const bodies = $derived(scene.data.nodes.filter((n) => n.uid !== self && BODIES.includes(n.kind)));
  const missing = $derived(value !== '' && !bodies.some((n) => n.uid === value));
</script>

<div class="campo">
  <span class="flabel">{label}</span>
  <div class="pick">
    <select class="sel" class:warn={missing} aria-label={label} aria-invalid={missing} value={value} onchange={(e) => onchange(e.currentTarget.value)}>
      <option value="">Ponto fixo no mundo</option>
      {#each bodies as n (n.uid)}
        <option value={n.uid}>{n.name} ({n.kind})</option>
      {/each}
      {#if missing}
        <option value={value}>Nó ausente ou não é corpo</option>
      {/if}
    </select>
  </div>
  {#if missing}
    <p class="aviso" role="status"><TriangleAlert size={11} /> O nó escolhido não é um corpo da cena.</p>
  {/if}
</div>

<style>
  .campo {
    display: grid;
    grid-template-columns: 84px minmax(0, 1fr);
    align-items: start;
    gap: 6px;
  }
  .flabel {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-11);
    color: var(--text-dim);
  }
  .pick {
    display: flex;
    min-width: 0;
  }
  .sel {
    flex: 1;
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
