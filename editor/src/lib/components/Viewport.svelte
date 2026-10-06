<script lang="ts">
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { scene, type SceneObject, type ShapeKind } from '$lib/state/scene.svelte';
  import { app } from '$lib/state/app.svelte';
  import { runtime } from '$lib/state/runtime.svelte';
  import { log } from '$lib/state/log.svelte';
  import { viewApi } from '$lib/state/view';
  import { createViewport, type Viewport } from '$lib/runtime/viewport';
  import ShapeIcon from './ShapeIcon.svelte';
  import { MousePointer2, Hand, Scan, Monitor, TriangleAlert, LoaderCircle } from '@lucide/svelte';

  let host: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let vp: Viewport | null = null;

  let status = $state<'loading' | 'ready' | 'error'>('loading');
  let errorText = $state('');
  let cam = $state({ x: 640, y: 360, zoom: 0.75 });
  let size = $state({ w: 1, h: 1 });
  let spaceDown = $state(false);
  let panning = $state(false);
  let moving = $state(false);
  let dropActive = $state(false);
  let dirty = true;

  const clamp = (v: number, a: number, b: number) => Math.min(b, Math.max(a, v));
  const toWorld = (sx: number, sy: number) => ({
    x: cam.x + (sx - size.w / 2) / cam.zoom,
    y: cam.y + (sy - size.h / 2) / cam.zoom,
  });
  const toScreen = (wx: number, wy: number) => ({
    x: (wx - cam.x) * cam.zoom + size.w / 2,
    y: (wy - cam.y) * cam.zoom + size.h / 2,
  });
  const local = (e: { clientX: number; clientY: number }) => {
    const r = host.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  };

  // Qualquer mudança relevante marca o frame para redesenho.
  $effect(() => {
    scene.revision;
    scene.selectedId;
    scene.hoveredId;
    cam.x;
    cam.y;
    cam.zoom;
    app.playing;
    app.zoom = cam.zoom;
    dirty = true;
  });

  // ---------- Câmera ----------
  let camAnim = 0;
  function animateTo(x: number, y: number, zoom: number) {
    cancelAnimationFrame(camAnim);
    const from = { ...cam };
    const t0 = performance.now();
    const step = (t: number) => {
      const k = Math.min(1, (t - t0) / 260);
      const e = 1 - Math.pow(1 - k, 3);
      cam.x = from.x + (x - from.x) * e;
      cam.y = from.y + (y - from.y) * e;
      cam.zoom = from.zoom * Math.pow(zoom / from.zoom, e);
      if (k < 1) camAnim = requestAnimationFrame(step);
    };
    camAnim = requestAnimationFrame(step);
  }
  function frameRect(minX: number, minY: number, maxX: number, maxY: number, pad = 72, instant = false) {
    const w = Math.max(maxX - minX, 1);
    const h = Math.max(maxY - minY, 1);
    const zoom = clamp(Math.min((size.w - pad * 2) / w, (size.h - pad * 2) / h), 0.05, 4);
    const x = (minX + maxX) / 2;
    const y = (minY + maxY) / 2;
    if (instant) cam = { x, y, zoom };
    else animateTo(x, y, zoom);
  }
  function bounds(o: SceneObject) {
    const a = (o.rotation * Math.PI) / 180;
    const ex = Math.abs((o.w / 2) * Math.cos(a)) + Math.abs((o.h / 2) * Math.sin(a));
    const ey = Math.abs((o.w / 2) * Math.sin(a)) + Math.abs((o.h / 2) * Math.cos(a));
    return { minX: o.x - ex, minY: o.y - ey, maxX: o.x + ex, maxY: o.y + ey };
  }
  viewApi.frameScreen = (instant = false) => frameRect(0, 0, scene.data.screen[0], scene.data.screen[1], 56, instant as boolean);
  viewApi.frameSelection = () => {
    const o = scene.selected;
    if (!o) return viewApi.frameScreen();
    const b = bounds(o);
    frameRect(b.minX, b.minY, b.maxX, b.maxY, Math.min(size.w, size.h) * 0.3);
  };
  viewApi.zoomTo = (z: number) => animateTo(cam.x, cam.y, z);
  viewApi.center = () => ({ x: cam.x, y: cam.y });

  // ---------- Renderização ----------
  function hexToRgb(hex: string) {
    const n = parseInt(hex.slice(1), 16);
    return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
  }
  let buf = new Float32Array(12 * 128);
  function upload(objs: SceneObject[]) {
    let n = 0;
    if (buf.length < objs.length * 12) buf = new Float32Array(objs.length * 24);
    const sel = app.playing ? null : scene.selectedId;
    const hov = app.playing ? null : scene.hoveredId;
    for (const o of objs) {
      if (!o.visible) continue;
      const k = n * 12;
      const [r, g, b] = hexToRgb(o.color);
      buf[k] = o.x;
      buf[k + 1] = o.y;
      buf[k + 2] = o.w;
      buf[k + 3] = o.h;
      buf[k + 4] = (o.rotation * Math.PI) / 180;
      buf[k + 5] = o.shape;
      buf[k + 6] = o.id === sel ? 1 : o.id === hov ? 2 : 0;
      buf[k + 7] = 0;
      buf[k + 8] = r;
      buf[k + 9] = g;
      buf[k + 10] = b;
      buf[k + 11] = 1;
      n++;
    }
    vp!.set_sprites(buf.subarray(0, n * 12));
  }

  function resize() {
    const r = host.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    size = { w: r.width, h: r.height };
    const w = Math.max(1, Math.round(r.width * dpr));
    const h = Math.max(1, Math.round(r.height * dpr));
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w;
      canvas.height = h;
      vp?.resize(w, h);
    }
    dirty = true;
  }

  onMount(() => {
    let raf = 0;
    let alive = true;
    let last = 0;
    let frames = 0;
    let acc = 0;
    let wasPlaying = false;

    const frame = (t: number) => {
      raf = requestAnimationFrame(frame);
      if (!vp) return;
      const dt = last ? Math.min((t - last) / 1000, 0.1) : 0;
      last = t;
      if (app.playing) {
        if (wasPlaying) runtime.step(dt);
        dirty = true;
        frames++;
        acc += dt;
        if (acc >= 0.5) {
          app.fps = Math.round(frames / acc);
          frames = 0;
          acc = 0;
        }
      }
      wasPlaying = app.playing;
      if (!dirty) return;
      dirty = false;
      upload(app.playing ? runtime.objects : scene.data.objects);
      const dpr = window.devicePixelRatio || 1;
      vp.set_camera(cam.x, cam.y, cam.zoom * dpr, t / 1000, scene.data.screen[0], scene.data.screen[1]);
      try {
        vp.render();
      } catch (e) {
        log.error('Erro de renderização: ' + e);
      }
    };

    const ro = new ResizeObserver(resize);
    ro.observe(host);
    resize();
    viewApi.frameScreen(true);

    (async () => {
      try {
        const v = await createViewport(canvas);
        if (!alive) return v.free();
        vp = v;
        resize();
        app.gpu = v.info();
        status = 'ready';
        log.info('Viewport pronto: ' + app.gpu);
        raf = requestAnimationFrame(frame);
      } catch (e) {
        status = 'error';
        errorText = e instanceof Error ? e.message : String(e);
        log.error('Falha ao iniciar o viewport: ' + errorText);
      }
    })();

    return () => {
      alive = false;
      cancelAnimationFrame(raf);
      cancelAnimationFrame(camAnim);
      ro.disconnect();
      vp?.free();
      vp = null;
    };
  });

  // ---------- Interação ----------
  function pick(wx: number, wy: number): SceneObject | null {
    const objs = scene.data.objects;
    for (let i = objs.length - 1; i >= 0; i--) {
      const o = objs[i];
      if (!o.visible) continue;
      const a = (o.rotation * Math.PI) / 180;
      const dx = wx - o.x;
      const dy = wy - o.y;
      const lx = dx * Math.cos(a) + dy * Math.sin(a);
      const ly = -dx * Math.sin(a) + dy * Math.cos(a);
      const hw = o.w / 2;
      const hh = o.h / 2;
      const inside =
        o.shape === 1
          ? Math.hypot(lx, ly) <= Math.min(hw, hh)
          : o.shape === 2
            ? Math.abs(lx) / hw + Math.abs(ly) / hh <= 1
            : Math.abs(lx) <= hw && Math.abs(ly) <= hh;
      if (inside) return o;
    }
    return null;
  }

  type Drag = { kind: 'pan'; sx: number; sy: number; cx: number; cy: number } | { kind: 'move'; id: string; ox: number; oy: number; key: string };
  let drag: Drag | null = null;

  function down(e: PointerEvent) {
    host.focus();
    const p = local(e);
    const wantsPan = e.button === 1 || e.button === 2 || (e.button === 0 && (spaceDown || app.tool === 'hand'));
    if (wantsPan) {
      canvas.setPointerCapture(e.pointerId);
      cancelAnimationFrame(camAnim);
      drag = { kind: 'pan', sx: p.x, sy: p.y, cx: cam.x, cy: cam.y };
      panning = true;
      return;
    }
    if (e.button !== 0 || app.playing) return;
    const w = toWorld(p.x, p.y);
    const hit = pick(w.x, w.y);
    scene.selectedId = hit?.id ?? null;
    if (hit && !hit.locked) {
      canvas.setPointerCapture(e.pointerId);
      drag = { kind: 'move', id: hit.id, ox: w.x - hit.x, oy: w.y - hit.y, key: 'drag:' + hit.id + ':' + performance.now() };
    }
  }

  function move(e: PointerEvent) {
    const p = local(e);
    if (drag?.kind === 'pan') {
      cam.x = drag.cx - (p.x - drag.sx) / cam.zoom;
      cam.y = drag.cy - (p.y - drag.sy) / cam.zoom;
      return;
    }
    if (drag?.kind === 'move') {
      const w = toWorld(p.x, p.y);
      const snap = e.shiftKey ? 16 : 1;
      const nx = Math.round((w.x - drag.ox) / snap) * snap;
      const ny = Math.round((w.y - drag.oy) / snap) * snap;
      const o = scene.data.objects.find((o) => o.id === (drag as { id: string }).id);
      if (o && (o.x !== nx || o.y !== ny)) {
        scene.update(o.id, { x: nx, y: ny }, 'Mover ' + o.name, drag.key);
        moving = true;
      }
      return;
    }
    if (!app.playing) {
      const w = toWorld(p.x, p.y);
      scene.hoveredId = pick(w.x, w.y)?.id ?? null;
    }
  }

  function up() {
    if (drag?.kind === 'move') scene.endGroup();
    drag = null;
    panning = false;
    moving = false;
  }

  function wheel(e: WheelEvent) {
    e.preventDefault();
    cancelAnimationFrame(camAnim);
    const p = local(e);
    const before = toWorld(p.x, p.y);
    cam.zoom = clamp(cam.zoom * Math.exp(-e.deltaY * 0.0015), 0.05, 16);
    const after = toWorld(p.x, p.y);
    cam.x += before.x - after.x;
    cam.y += before.y - after.y;
  }

  function keydown(e: KeyboardEvent) {
    if (e.code === 'Space' && !spaceDown) {
      spaceDown = true;
      e.preventDefault();
    }
  }
  function keyup(e: KeyboardEvent) {
    if (e.code === 'Space') spaceDown = false;
  }

  function dragover(e: DragEvent) {
    if (!e.dataTransfer?.types.includes('application/x-nexo-shape') || app.playing) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = 'copy';
    dropActive = true;
  }
  function drop(e: DragEvent) {
    dropActive = false;
    const data = e.dataTransfer?.getData('application/x-nexo-shape');
    if (data == null || data === '') return;
    e.preventDefault();
    const p = local(e);
    const w = toWorld(p.x, p.y);
    scene.add(Number(data) as ShapeKind, w.x, w.y);
  }

  const sel = $derived(scene.selected);
  const label = $derived.by(() => {
    if (!sel || app.playing || !sel.visible) return null;
    const b = bounds(sel);
    const s = toScreen(sel.x, b.minY);
    return { x: s.x, y: s.y };
  });
  const cursor = $derived(panning ? 'grabbing' : spaceDown || app.tool === 'hand' ? 'grab' : moving ? 'move' : scene.hoveredId ? 'pointer' : 'default');
</script>

<svelte:window onkeyup={keyup} onblur={() => (spaceDown = false)} />

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="viewport"
  class:drop={dropActive}
  class:playing={app.playing}
  bind:this={host}
  tabindex="-1"
  role="application"
  aria-label="Viewport da cena"
  onkeydown={keydown}
  ondragover={dragover}
  ondragleave={() => (dropActive = false)}
  ondrop={drop}
  style="cursor: {cursor}"
>
  <canvas
    bind:this={canvas}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={up}
    onpointerleave={() => !drag && (scene.hoveredId = null)}
    onwheel={wheel}
  ></canvas>

  {#if label}
    <div class="sel-label" style="transform: translate({label.x}px, {label.y - 12}px) translate(-50%, -100%)">
      <ShapeIcon shape={sel!.shape} color={sel!.color} size={11} />
      {sel!.name}
      <span class="mono">{Math.round(sel!.w)}×{Math.round(sel!.h)}</span>
    </div>
  {/if}

  <div class="toolbar">
    <button class="icon-btn" class:active={app.tool === 'select'} title="Selecionar e mover (V)" onclick={() => (app.tool = 'select')}><MousePointer2 size={15} /></button>
    <button class="icon-btn" class:active={app.tool === 'hand'} title="Mão (H) · ou segure Espaço" onclick={() => (app.tool = 'hand')}><Hand size={15} /></button>
    <span class="div"></span>
    <button class="icon-btn" title="Enquadrar seleção (F)" onclick={() => viewApi.frameSelection()}><Scan size={15} /></button>
    <button class="icon-btn" title="Enquadrar tela do jogo (Home)" onclick={() => viewApi.frameScreen()}><Monitor size={15} /></button>
    <span class="div"></span>
    <button class="zoom mono" title="Voltar para 100% (Ctrl+1)" onclick={() => viewApi.zoomTo(1)}>{Math.round(cam.zoom * 100)}%</button>
  </div>

  {#if app.playing}
    <div class="playing-badge" transition:fade={{ duration: 160 }}>
      <span class="dot"></span> Jogando <kbd>Esc</kbd> para parar
    </div>
  {/if}

  {#if status === 'ready' && scene.data.objects.length === 0 && !app.playing}
    <div class="empty" transition:fade={{ duration: 200 }}>
      <strong>Sua cena está vazia</strong>
      <span>Arraste uma forma do painel <b>Assets</b> para cá, ou pressione <kbd>Ctrl</kbd> <kbd>K</kbd>.</span>
    </div>
  {/if}

  {#if status === 'loading'}
    <div class="state"><LoaderCircle size={20} class="spin" /> Iniciando runtime…</div>
  {:else if status === 'error'}
    <div class="state error">
      <TriangleAlert size={22} />
      <strong>Não foi possível iniciar o viewport</strong>
      <span class="mono">{errorText}</span>
    </div>
  {/if}
</div>

<style>
  .viewport {
    position: absolute;
    inset: 0;
    overflow: hidden;
    outline: none;
    background: #0e0f13;
    transition: box-shadow var(--t-med) var(--ease);
  }
  .viewport.drop {
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  .viewport.playing {
    box-shadow: inset 0 0 0 2px rgb(61 220 151 / 0.55);
  }
  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    touch-action: none;
  }
  .toolbar {
    position: absolute;
    top: 12px;
    left: 12px;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 4px;
    border-radius: 10px;
    background: rgb(21 23 29 / 0.82);
    backdrop-filter: blur(12px);
    box-shadow: 0 0 0 1px #2a2e39, 0 8px 24px rgb(0 0 0 / 0.35);
    color: #e8eaf0;
  }
  .toolbar .icon-btn {
    color: #8a90a2;
  }
  .toolbar .icon-btn:hover {
    color: #e8eaf0;
    background: rgb(255 255 255 / 0.06);
  }
  .toolbar .icon-btn.active {
    color: #9a82ff;
    background: rgb(124 92 255 / 0.18);
  }
  .div {
    width: 1px;
    height: 18px;
    margin: 0 4px;
    background: #2a2e39;
  }
  .zoom {
    height: 28px;
    min-width: 52px;
    padding: 0 8px;
    border-radius: 6px;
    font-size: var(--fs-12);
    color: #8a90a2;
  }
  .zoom:hover {
    color: #e8eaf0;
    background: rgb(255 255 255 / 0.06);
  }
  .sel-label {
    position: absolute;
    left: 0;
    top: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 8px;
    border-radius: 6px;
    font-size: var(--fs-11);
    font-weight: 600;
    white-space: nowrap;
    color: white;
    background: #7c5cff;
    box-shadow: 0 4px 14px rgb(124 92 255 / 0.4);
    pointer-events: none;
  }
  .sel-label span {
    font-weight: 400;
    opacity: 0.75;
  }
  .playing-badge {
    position: absolute;
    top: 12px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px 6px 12px;
    border-radius: 99px;
    font-size: var(--fs-12);
    font-weight: 600;
    color: #e8eaf0;
    background: rgb(21 23 29 / 0.85);
    backdrop-filter: blur(12px);
    box-shadow: 0 0 0 1px rgb(61 220 151 / 0.4), 0 8px 24px rgb(0 0 0 / 0.35);
  }
  .playing-badge kbd {
    color: #8a90a2;
    background: #242833;
    border-color: #2a2e39;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #3ddc97;
    animation: pulse 1.4s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      box-shadow: 0 0 0 5px rgb(61 220 151 / 0.2);
    }
  }
  .empty {
    position: absolute;
    left: 50%;
    bottom: 32px;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 14px 20px;
    border-radius: 12px;
    text-align: center;
    color: #8a90a2;
    background: rgb(21 23 29 / 0.85);
    backdrop-filter: blur(12px);
    box-shadow: 0 0 0 1px #2a2e39, 0 8px 24px rgb(0 0 0 / 0.35);
    pointer-events: none;
  }
  .empty strong {
    color: #e8eaf0;
  }
  .empty b {
    color: #e8eaf0;
    font-weight: 600;
  }
  .empty kbd {
    color: #8a90a2;
    background: #242833;
    border-color: #2a2e39;
  }
  .state {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: #8a90a2;
    pointer-events: none;
  }
  .state.error {
    color: var(--danger);
  }
  .state.error strong {
    color: #e8eaf0;
  }
  .state.error span {
    max-width: 520px;
    text-align: center;
    font-size: var(--fs-12);
    color: #8a90a2;
  }
</style>
