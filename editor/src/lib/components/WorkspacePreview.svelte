<script lang="ts">
  import { fade } from 'svelte/transition';
  import type { Workspace } from '$lib/state/app.svelte';
  import { scene } from '$lib/state/scene.svelte';
  import { Workflow, Film, Brush } from '@lucide/svelte';

  // Só as abas que não são o viewport: Cena e Jogo são renderizadas pelo Viewport.
  let { workspace }: { workspace: Exclude<Workspace, 'cena' | 'jogo'> } = $props();

  const info = {
    logica: { icon: Workflow, title: 'Lógica visual', phase: 'Fase 2', text: 'Folhas de eventos e grafos de nós sobre o mesmo modelo, com depuração ao vivo e C# gerado na hora.' },
    animacao: { icon: Film, title: 'Animação', phase: 'Fase 3', text: 'Linha do tempo que anima qualquer propriedade, com curvas e eventos.' },
    sprite: { icon: Brush, title: 'Editor de sprites', phase: 'Fase 3', text: 'Desenho em pixel art, animação quadro a quadro e importação do Aseprite.' },
    cena: { icon: Workflow, title: '', phase: '', text: '' },
  } as const;
  const w = $derived(info[workspace]);
  const target = $derived(scene.selected?.name ?? 'Jogador');
</script>

<div class="wp" in:fade={{ duration: 180 }}>
  {#if workspace === 'logica'}
    <div class="graph" aria-hidden="true">
      <svg class="wires" viewBox="0 0 900 330">
        <path d="M240 98 C304 98 304 150 368 150" class="exec" />
        <path d="M240 268 C304 268 304 186 368 186" class="flt" />
        <path d="M568 150 C630 150 630 118 690 118" class="exec" />
      </svg>
      <!-- Cada família de nó tem uma cor (--node-*), definida em styles/tokens.css -->
      <div class="node event" style="left: 40px; top: 56px">
        <header>Ao pressionar ação</header>
        <div class="pin-row"><em>Ação</em><b>Pular</b><i class="tri out"></i></div>
      </div>
      <div class="node data" style="left: 40px; top: 226px">
        <header>Força do pulo</header>
        <div class="pin-row"><em>Valor</em><b>400</b><i class="dot flt out"></i></div>
      </div>
      <div class="node physics" style="left: 368px; top: 112px">
        <header>Aplicar impulso</header>
        <div class="pin-row"><i class="tri"></i><span>{target}</span><i class="tri out"></i></div>
        <div class="pin-row"><i class="dot flt"></i><em>Força Y</em></div>
      </div>
      <div class="node audio" style="left: 690px; top: 82px">
        <header>Tocar som</header>
        <div class="pin-row"><i class="tri"></i><em>pulo.wav</em></div>
      </div>
    </div>
  {/if}
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
  .graph {
    position: relative;
    flex: none;
    width: 900px;
    height: 330px;
    transform: scale(0.9);
  }
  .wires {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: visible;
  }
  .wires path {
    fill: none;
    stroke-width: 2px;
  }
  /* Fluxo de execução: tracejado animado. Dados: linha sólida na cor do tipo. */
  .wires .exec {
    stroke: var(--type-exec);
    stroke-dasharray: 6 8;
    animation: flow 1.2s linear infinite;
  }
  .wires .flt {
    stroke: var(--type-float);
  }
  @keyframes flow {
    to {
      stroke-dashoffset: -28;
    }
  }
  .node {
    --fam: var(--node-flow);
    position: absolute;
    width: 200px;
    border-radius: var(--r-ctl);
    background: var(--bg-2);
    border: 1px solid var(--line);
    box-shadow: var(--shadow-card);
    font-size: var(--fs-12);
    overflow: hidden;
  }
  .node.event {
    --fam: var(--node-event);
  }
  .node.data {
    --fam: var(--node-data);
  }
  .node.physics {
    --fam: var(--node-physics);
  }
  .node.audio {
    --fam: var(--node-audio);
  }
  /* Cabeçalho na cor da família, sem gradiente: a cor identifica o tipo do nó */
  .node header {
    padding: 4px 8px;
    font-weight: 500;
    color: var(--text);
    background: color-mix(in srgb, var(--fam) 30%, var(--bg-2));
    border-bottom: 1px solid var(--line);
  }
  .pin-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
  }
  .pin-row em {
    font-style: normal;
    color: var(--text-dim);
  }
  .pin-row b {
    margin-left: auto;
    font-family: var(--font-mono);
    font-size: var(--fs-11);
    font-weight: 500;
    padding: 0 6px;
    border-radius: var(--r-ctl);
    border: 1px solid var(--line);
    background: var(--bg-0);
  }
  .tri {
    flex: none;
    width: 0;
    height: 0;
    border-left: 7px solid var(--type-exec);
    border-top: 5px solid transparent;
    border-bottom: 5px solid transparent;
  }
  .pin-row span + .tri.out {
    margin-left: auto;
  }
  .dot {
    flex: none;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    border: 2px solid var(--type-float);
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
