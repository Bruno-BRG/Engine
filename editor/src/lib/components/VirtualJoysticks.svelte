<script lang="ts">
  // Joysticks de toque sobre o viewport durante a reprodução (INP-004). Cada nó VirtualJoystick aparece como
  // um disco na lateral da tela; o polegar manda o vetor ao WASM, que empurra as ações de direção do nó.
  // No desktop ficam ocultos, a não ser que o nó tenha "Mostrar no desktop" ligado.
  import { scene } from '$lib/state/scene.svelte';

  /** Afastamento da base até a borda do viewport, em pixels. */
  const MARGEM = 24;
  /** Curso do polegar, como fração do raio: o disco do polegar nunca sai da base. */
  const CURSO = 0.6;
  /** Dispositivo com tela de toque (celular, tablet, notebook com touch). */
  const TOQUE = typeof navigator !== 'undefined' && navigator.maxTouchPoints > 0;

  const joysticks = $derived(
    scene.data.nodes.filter((n) => n.kind === 'VirtualJoystick' && (TOQUE || n.props.show_on_desktop === true)),
  );

  /** Polegar de cada joystick, de -1 a 1 em cada eixo. Volta ao centro quando o toque termina. */
  let polegar = $state<Record<string, { x: number; y: number }>>({});

  /** Mede o toque em relação ao centro da base, com o módulo limitado a 1. */
  function mira(e: PointerEvent, uid: string, base: HTMLElement) {
    const r = base.getBoundingClientRect();
    let x = (e.clientX - (r.left + r.width / 2)) / (r.width / 2);
    let y = (e.clientY - (r.top + r.height / 2)) / (r.height / 2);
    const len = Math.hypot(x, y);
    if (len > 1) {
      x /= len;
      y /= len;
    }
    polegar[uid] = { x, y };
    scene.editor?.set_joystick(uid, x, y);
  }

  function pressiona(e: PointerEvent, uid: string) {
    const base = e.currentTarget as HTMLElement;
    base.setPointerCapture(e.pointerId);
    mira(e, uid, base);
  }

  function arrasta(e: PointerEvent, uid: string) {
    const base = e.currentTarget as HTMLElement;
    if (base.hasPointerCapture(e.pointerId)) mira(e, uid, base);
  }

  /** Soltar o dedo (ou perder o toque, o que também tira a captura) devolve o polegar ao centro. */
  function solta(uid: string) {
    polegar[uid] = { x: 0, y: 0 };
    scene.editor?.set_joystick(uid, 0, 0);
  }
</script>

{#each joysticks as n (n.uid)}
  {@const raio = Number(n.props.radius ?? 64)}
  {@const p = polegar[n.uid] ?? { x: 0, y: 0 }}
  <div
    class="joy"
    role="application"
    aria-label="Joystick de toque {n.name}"
    class:ativo={p.x !== 0 || p.y !== 0}
    style="--raio: {raio}px; {n.props.side === 'right' ? 'right' : 'left'}: {MARGEM}px"
    onpointerdown={(e) => pressiona(e, n.uid)}
    onpointermove={(e) => arrasta(e, n.uid)}
    onlostpointercapture={() => solta(n.uid)}
  >
    <span class="polegar" style="transform: translate({p.x * raio * CURSO}px, {p.y * raio * CURSO}px)"></span>
  </div>
{/each}

<style>
  .joy {
    position: absolute;
    bottom: 24px;
    z-index: 2;
    display: grid;
    place-items: center;
    width: calc(var(--raio) * 2);
    height: calc(var(--raio) * 2);
    border-radius: 50%;
    background: var(--bg-overlay);
    box-shadow: 0 0 0 1px var(--line), var(--shadow-float);
    /* O toque no disco não rola a página nem dá zoom no navegador. */
    touch-action: none;
    user-select: none;
  }
  .polegar {
    display: block;
    width: calc(var(--raio) * 0.8);
    height: calc(var(--raio) * 0.8);
    border-radius: 50%;
    background: var(--bg-3);
    box-shadow: 0 0 0 1px var(--line);
    pointer-events: none;
  }
  .joy.ativo .polegar {
    background: var(--accent);
  }
</style>
