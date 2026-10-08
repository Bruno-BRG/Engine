<script lang="ts">
  import { fade } from 'svelte/transition';
  import { Film, Brush } from '@lucide/svelte';

  // Abas que ainda são só uma prévia: Animação e Sprite (fases 3). Cena e Jogo são renderizadas pelo Viewport,
  // e Lógica tem o próprio workspace (LogicWorkspace).
  let { workspace }: { workspace: 'animacao' | 'sprite' } = $props();

  const info = {
    animacao: { icon: Film, title: 'Animação', phase: 'Fase 3', text: 'Linha do tempo que anima qualquer propriedade, com curvas e eventos.' },
    sprite: { icon: Brush, title: 'Editor de sprites', phase: 'Fase 3', text: 'Desenho em pixel art, animação quadro a quadro e importação do Aseprite.' },
  } as const;
  const w = $derived(info[workspace]);
</script>

<div class="wp" in:fade={{ duration: 180 }}>
  <div class="card">
    <div class="icon"><w.icon size={22} /></div>
    <div>
      <div class="title">{w.title} <span>{w.phase}</span></div>
      <p>{w.text}</p>
    </div>
  </div>
</div>

<style>
  .wp {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 28px;
    background: radial-gradient(circle, var(--canvas-dot) 1px, transparent 1px) 0 0 / 22px 22px, var(--bg-0);
    overflow: hidden;
  }
  .card {
    display: flex;
    gap: 14px;
    max-width: 520px;
    padding: 16px 18px;
    border-radius: var(--r-dlg);
    background: var(--bg-1);
    box-shadow: 0 0 0 1px var(--line-soft), var(--shadow-card);
  }
  .icon {
    flex: none;
    width: 42px;
    height: 42px;
    border-radius: var(--r-ctl);
    display: grid;
    place-items: center;
    color: var(--accent-hi);
    background: var(--accent-soft);
  }
  .title {
    font-size: var(--fs-15);
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .title span {
    font-size: var(--fs-11);
    font-weight: 500;
    color: var(--text);
    border: 1px solid var(--accent-ring);
    padding: 0 6px;
    border-radius: var(--r-ctl);
  }
  p {
    margin: 4px 0 0;
    color: var(--text-dim);
    line-height: 1.5;
  }
</style>
