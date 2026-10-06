<script lang="ts">
  import { app } from '$lib/state/app.svelte';
  import * as act from '$lib/actions';
  import type { ShapeKind } from '$lib/state/scene.svelte';
  import { Shapes, Folder, Upload } from '@lucide/svelte';

  let folder = $state<'formas' | 'assets'>('formas');
  const shapes: { shape: ShapeKind; name: string; color: string }[] = [
    { shape: 0, name: 'Retângulo', color: '#7c5cff' },
    { shape: 1, name: 'Círculo', color: '#3ddc97' },
    { shape: 2, name: 'Losango', color: '#ffd84d' },
  ];
</script>

<div class="assets">
  <nav>
    <button class:on={folder === 'formas'} onclick={() => (folder = 'formas')}><Shapes size={14} /> Formas básicas</button>
    <button class:on={folder === 'assets'} onclick={() => (folder = 'assets')}><Folder size={14} /> assets/</button>
  </nav>
  <div class="area">
    {#if folder === 'formas'}
      <div class="grid">
        {#each shapes as s (s.shape)}
          <button
            class="card"
            draggable={!app.playing}
            ondragstart={(e) => { e.dataTransfer?.setData('application/x-nexo-shape', String(s.shape)); if (e.dataTransfer) e.dataTransfer.effectAllowed = 'copy'; }}
            ondblclick={() => act.addShape(s.shape)}
            title="Arraste para a cena ou dê dois cliques"
          >
            <div class="preview" style="--c: {s.color}">
              <svg viewBox="0 0 64 64" width="44" height="44">
                {#if s.shape === 1}
                  <circle cx="32" cy="32" r="22" fill={s.color} />
                {:else if s.shape === 2}
                  <path d="M32 8 56 32 32 56 8 32Z" fill={s.color} />
                {:else}
                  <rect x="10" y="10" width="44" height="44" rx="10" fill={s.color} />
                {/if}
              </svg>
            </div>
            <span>{s.name}</span>
          </button>
        {/each}
      </div>
      <p class="tip">Arraste uma forma para o viewport ou dê dois cliques para criar no centro da tela.</p>
    {:else}
      <div class="drop">
        <Upload size={20} />
        <strong>Importação de imagens, sons e fontes</strong>
        <span>Chega na Fase 1. Os arquivos ficarão em <code>assets/</code> no projeto e serão recarregados automaticamente quando você os editar em outro programa.</span>
      </div>
    {/if}
  </div>
</div>

<style>
  .assets {
    flex: 1;
    display: flex;
    min-width: 0;
  }
  nav {
    flex: none;
    width: 180px;
    padding: 8px;
    border-right: 1px solid var(--line-soft);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  nav button {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 28px;
    padding: 0 8px;
    border-radius: 6px;
    font-size: var(--fs-12);
    color: var(--text-dim);
  }
  nav button:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  nav button.on {
    background: var(--bg-active);
    color: var(--text);
  }
  .area {
    flex: 1;
    min-width: 0;
    overflow: auto;
    padding: 12px 14px;
  }
  .grid {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }
  .card {
    width: 96px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 8px 6px;
    border-radius: 10px;
    border: 1px solid transparent;
    transition: background var(--t-fast), border-color var(--t-fast), transform var(--t-fast) var(--ease);
    cursor: grab;
  }
  .card:hover {
    background: var(--bg-hover);
    border-color: var(--line);
  }
  .card:active {
    cursor: grabbing;
    transform: scale(0.97);
  }
  .preview {
    width: 72px;
    height: 72px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    background: radial-gradient(circle at 50% 30%, color-mix(in srgb, var(--c) 18%, transparent), transparent 70%), var(--bg-0);
    border: 1px solid var(--line-soft);
  }
  .card span {
    font-size: var(--fs-12);
    color: var(--text-dim);
  }
  .tip {
    margin: 12px 2px 0;
    font-size: var(--fs-12);
    color: var(--text-faint);
  }
  .drop {
    height: 100%;
    min-height: 120px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 16px;
    text-align: center;
    border: 1.5px dashed var(--line);
    border-radius: 12px;
    color: var(--text-faint);
  }
  .drop strong {
    color: var(--text-dim);
  }
  .drop span {
    max-width: 460px;
    font-size: var(--fs-12);
    line-height: 1.5;
  }
  code {
    font-family: var(--font-mono);
    color: var(--text-dim);
  }
</style>
