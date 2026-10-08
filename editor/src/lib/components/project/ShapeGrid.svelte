<script lang="ts">
  import { app } from '$lib/state/app.svelte';
  import * as act from '$lib/actions';
  import { OBJECT_PALETTE as P } from '$lib/palette';

  // Pasta virtual "Formas básicas": arraste um card para o viewport, ou dê dois cliques para criar no centro.
  // Os índices 0, 1 e 2 são os de act.addShape (retângulo, círculo, losango).
  const shapes: { shape: 0 | 1 | 2; name: string; color: string }[] = [
    { shape: 0, name: 'Retângulo', color: P[0] },
    { shape: 1, name: 'Círculo', color: P[2] },
    { shape: 2, name: 'Losango', color: P[3] },
  ];
</script>

<div class="shapes">
  {#each shapes as s (s.shape)}
    <button
      class="card"
      draggable={!app.playing}
      ondragstart={(e) => {
        e.dataTransfer?.setData('application/x-nexo-shape', String(s.shape));
        if (e.dataTransfer) e.dataTransfer.effectAllowed = 'copy';
      }}
      ondblclick={() => act.addShape(s.shape)}
      title="Arraste para a cena ou dê dois cliques"
    >
      <span class="preview">
        <svg viewBox="0 0 64 64" width="44" height="44">
          {#if s.shape === 1}
            <circle cx="32" cy="32" r="22" fill={s.color} />
          {:else if s.shape === 2}
            <path d="M32 8 56 32 32 56 8 32Z" fill={s.color} />
          {:else}
            <rect x="10" y="10" width="44" height="44" rx="10" fill={s.color} />
          {/if}
        </svg>
      </span>
      <span class="label">{s.name}</span>
    </button>
  {/each}
  <p class="tip">Arraste uma forma para o viewport ou dê dois cliques para criar no centro da tela.</p>
</div>

<style>
  .shapes {
    display: flex;
    flex-wrap: wrap;
    align-content: flex-start;
    gap: 4px;
    padding: 8px;
  }
  .card {
    width: 96px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 6px;
    border: 1px solid transparent;
    border-radius: var(--r-ctl);
    cursor: grab;
  }
  .card:hover {
    background: var(--bg-hover);
    border-color: var(--line);
  }
  .card:active {
    cursor: grabbing;
  }
  .preview {
    width: 64px;
    height: 64px;
    display: grid;
    place-items: center;
    background: var(--bg-0);
    border: 1px solid var(--line-soft);
    border-radius: var(--r-ctl);
  }
  .label {
    font-size: var(--fs-12);
    color: var(--text-dim);
  }
  .tip {
    width: 100%;
    margin: 8px 2px 0;
    font-size: var(--fs-12);
    color: var(--text-faint);
  }
</style>
