<script lang="ts">
  import { fade } from 'svelte/transition';
  import type { Workspace } from '$lib/state/app.svelte';
  import { scene } from '$lib/state/scene.svelte';
  import { Workflow, Film, Brush } from '@lucide/svelte';

  let { workspace }: { workspace: Workspace } = $props();

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
      <div class="node ev" style="left: 40px; top: 56px">
        <header>Ao pressionar ação</header>
        <div class="pin-row"><em>Ação</em><b>Pular</b><i class="tri out"></i></div>
      </div>
      <div class="node pure" style="left: 40px; top: 226px">
        <header>Força do pulo</header>
        <div class="pin-row"><em>Valor</em><b>400</b><i class="dot flt out"></i></div>
      </div>
      <div class="node" style="left: 368px; top: 112px">
        <header>Aplicar impulso</header>
        <div class="pin-row"><i class="tri"></i><span>{target}</span><i class="tri out"></i></div>
        <div class="pin-row"><i class="dot flt"></i><em>Força Y</em></div>
      </div>
      <div class="node" style="left: 690px; top: 82px">
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
    background:
      radial-gradient(circle at 50% 40%, rgb(124 92 255 / 0.08), transparent 60%),
      radial-gradient(circle, rgb(255 255 255 / 0.05) 1px, transparent 1px) 0 0 / 22px 22px,
      var(--bg-0);
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
    stroke-width: 2.5;
  }
  .wires .exec {
    stroke: #e8eaf0;
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
    position: absolute;
    width: 200px;
    border-radius: 10px;
    background: var(--bg-2);
    box-shadow: 0 0 0 1px var(--line), 0 12px 28px rgb(0 0 0 / 0.35);
    font-size: var(--fs-12);
    overflow: hidden;
  }
  .node header {
    padding: 8px 12px;
    font-weight: 600;
    background: linear-gradient(90deg, rgb(92 157 255 / 0.35), transparent);
    border-bottom: 1px solid var(--line-soft);
  }
  .node.ev header {
    background: linear-gradient(90deg, rgb(255 92 122 / 0.4), transparent);
  }
  .node.pure header {
    background: linear-gradient(90deg, rgb(61 220 151 / 0.3), transparent);
  }
  .pin-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 12px;
  }
  .pin-row em {
    font-style: normal;
    color: var(--text-dim);
  }
  .pin-row b {
    margin-left: auto;
    font-weight: 500;
    padding: 1px 8px;
    border-radius: 4px;
    background: var(--bg-0);
  }
  .tri {
    flex: none;
    width: 0;
    height: 0;
    border-left: 7px solid #e8eaf0;
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
    border-radius: 14px;
    background: var(--bg-1);
    box-shadow: 0 0 0 1px var(--line-soft), var(--shadow-card);
  }
  .icon {
    flex: none;
    width: 42px;
    height: 42px;
    border-radius: 11px;
    display: grid;
    place-items: center;
    color: var(--accent-hi);
    background: var(--accent-soft);
  }
  .title {
    font-size: var(--fs-15);
    font-weight: 650;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .title span {
    font-size: var(--fs-11);
    font-weight: 600;
    color: var(--accent-hi);
    border: 1px solid var(--accent-ring);
    padding: 1px 7px;
    border-radius: 99px;
  }
  p {
    margin: 4px 0 0;
    color: var(--text-dim);
    line-height: 1.5;
  }
</style>
