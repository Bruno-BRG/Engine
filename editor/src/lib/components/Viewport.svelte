<script lang="ts">
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { scene, type NodeView } from '$lib/state/scene.svelte';
  import { app, type Tool } from '$lib/state/app.svelte';
  import { runtime } from '$lib/state/runtime.svelte';
  import { textures } from '$lib/state/textures.svelte';
  import { STEP_SECONDS, addShape, copySelected, pasteClipboard, deleteSelected, duplicateSelected } from '$lib/actions';
  import { log } from '$lib/state/log.svelte';
  import { viewApi } from '$lib/state/view';
  import { createViewport, type Viewport } from '$lib/runtime/engine';
  import { showContextMenu, type MenuItem } from '$lib/state/contextmenu.svelte';
  import { focusPanel, focus } from '$lib/state/focus.svelte';
  import ViewportOverlay, { type Gizmo, type Box, type ShapeGizmo, type AudioRadius, type DebugOverlay } from './ViewportOverlay.svelte';
  import VirtualJoysticks from './VirtualJoysticks.svelte';
  import { Hand, Move, RotateCw, Maximize, Square, Scan, Monitor, TriangleAlert, LoaderCircle, Bug } from '@lucide/svelte';
  import type { SceneEditor } from '$lib/runtime/engine';

  /** `cena` edita a cena; `jogo` mostra a câmera atual com a proporção escolhida (EDT-012). */
  let { mode = 'cena', active = true }: { mode?: 'cena' | 'jogo'; active?: boolean } = $props();

  type Pt = { x: number; y: number };
  type World = NodeView['world'];

  const RAD = Math.PI / 180;
  const ROT_RING = 64;
  const HANDLE_HIT = 9;
  const IDENTITY: World = { x: 0, y: 0, rot: 0, sx: 1, sy: 1 };

  let host: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let vp: Viewport | null = null;

  let status = $state<'loading' | 'ready' | 'error'>('loading');
  let errorText = $state('');
  let cam = $state({ x: 640, y: 360, zoom: 0.75 });
  let size = $state({ w: 1, h: 1 });
  let spaceDown = $state(false);
  let panning = $state(false);
  let dragKind = $state<'' | 'move' | 'rotate' | 'scale' | 'rect' | 'box' | 'shape'>('');
  let box = $state<Box>(null);
  let dropActive = $state(false);
  let dirty = true;
  /** Ponteiro sobre este viewport, em pixels do canvas. Na reprodução, vai aos scripts (INP-003). */
  let pointerAt: Pt | null = null;

  const clamp = (v: number, a: number, b: number) => Math.min(b, Math.max(a, v));
  const toWorld = (sx: number, sy: number): Pt => ({
    x: cam.x + (sx - size.w / 2) / cam.zoom,
    y: cam.y + (sy - size.h / 2) / cam.zoom,
  });
  const toScreen = (wx: number, wy: number): Pt => ({
    x: (wx - cam.x) * cam.zoom + size.w / 2,
    y: (wy - cam.y) * cam.zoom + size.h / 2,
  });
  const local = (e: { clientX: number; clientY: number }): Pt => {
    const r = host.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  };

  // Qualquer mudança relevante marca o quadro para redesenho.
  $effect(() => {
    scene.revision;
    scene.selection;
    scene.hoveredId;
    textures.version;
    cam.x;
    cam.y;
    cam.zoom;
    app.playing;
    app.physicsDebug;
    app.gameAspect;
    app.gameScale;
    active;
    dirty = true;
    if (mode === 'cena' && active) app.zoom = cam.zoom;
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

  // ---------- Geometria dos nós ----------
  const world = (n: NodeView | undefined | null): World => n?.world ?? IDENTITY;
  const parentWorld = (n: NodeView): World => (n.parent ? world(scene.node(n.parent)) : IDENTITY);

  /** Converte um ponto do mundo para o espaço local de um pai com transformação `p`. */
  function toLocal(p: World, wx: number, wy: number): Pt {
    const dx = wx - p.x;
    const dy = wy - p.y;
    const c = Math.cos(p.rot * RAD);
    const s = Math.sin(p.rot * RAD);
    const lx = dx * c + dy * s;
    const ly = -dx * s + dy * c;
    return { x: lx / (p.sx || 1), y: ly / (p.sy || 1) };
  }

  /** Ponto do espaço do nó (unidades das propriedades) para o mundo: o inverso de `toLocal`. */
  function nodeToWorld(p: World, lx: number, ly: number): Pt {
    const c = Math.cos(p.rot * RAD);
    const s = Math.sin(p.rot * RAD);
    const ax = lx * p.sx;
    const ay = ly * p.sy;
    return { x: p.x + c * ax - s * ay, y: p.y + s * ax + c * ay };
  }

  // ---------- Formas de colisão (PHY-012) ----------
  const SHAPE_KINDS = ['CollisionShape2D', 'CollisionPolygon2D'];

  /**
   * Alças de uma forma, no espaço do nó. A chave diz o que a alça muda:
   * retângulo `c0..c3` (cantos, em ordem), círculo `r`, cápsula `x` (largura), `t`/`b` (altura),
   * segmento `a`/`b` (pontas), polígono `p<i>` (ponto i).
   */
  function shapeHandles(n: NodeView): { key: string; lx: number; ly: number }[] {
    if (n.kind === 'CollisionPolygon2D') {
      const pts = (n.props.points as [number, number][] | undefined) ?? [];
      return pts.map((p, i) => ({ key: 'p' + i, lx: p[0], ly: p[1] }));
    }
    const [w, h] = (n.props.size as [number, number]) ?? [32, 32];
    switch (n.props.shape) {
      case 'circle':
        return [{ key: 'r', lx: w / 2, ly: 0 }];
      case 'capsule_y':
        return [
          { key: 'x', lx: w / 2, ly: 0 },
          { key: 't', lx: 0, ly: -h / 2 },
          { key: 'b', lx: 0, ly: h / 2 },
        ];
      case 'segment':
        return [
          { key: 'a', lx: -w / 2, ly: 0 },
          { key: 'b', lx: w / 2, ly: 0 },
        ];
      default:
        return [
          { key: 'c0', lx: -w / 2, ly: -h / 2 },
          { key: 'c1', lx: w / 2, ly: -h / 2 },
          { key: 'c2', lx: w / 2, ly: h / 2 },
          { key: 'c3', lx: -w / 2, ly: h / 2 },
        ];
    }
  }

  /**
   * Ponto de ancoragem fixo durante o arrasto de uma alça, em mundo: o canto oposto do retângulo ou a
   * outra ponta do segmento. Círculo, cápsula e polígono não têm ancoragem (o centro não se move).
   */
  function anchorOf(n: NodeView, key: string): Pt | null {
    const [w, h] = (n.props.size as [number, number]) ?? [32, 32];
    const wp = world(n);
    if (n.kind !== 'CollisionShape2D') return null;
    if (n.props.shape === 'segment') return nodeToWorld(wp, key === 'a' ? w / 2 : -w / 2, 0);
    if (key.startsWith('c')) {
      const corners: [number, number][] = [
        [-w / 2, -h / 2],
        [w / 2, -h / 2],
        [w / 2, h / 2],
        [-w / 2, h / 2],
      ];
      const [ox, oy] = corners[(Number(key.slice(1)) + 2) % 4];
      return nodeToWorld(wp, ox, oy);
    }
    return null;
  }

  /** Alça de forma de colisão sob o ponto de tela, na forma selecionada (PHY-012). */
  function shapeHandleAt(p: Pt): { key: string } | null {
    if (!sel || mode !== 'cena' || app.playing || !SHAPE_KINDS.includes(sel.kind)) return null;
    const wp = world(sel);
    for (const h of shapeHandles(sel)) {
      const at = nodeToWorld(wp, h.lx, h.ly);
      const s = toScreen(at.x, at.y);
      if (Math.abs(s.x - p.x) <= HANDLE_HIT && Math.abs(s.y - p.y) <= HANDLE_HIT) return { key: h.key };
    }
    return null;
  }

  /**
   * Arrasta a alça `key` da forma `n` até o ponto `w` do mundo e grava com set_prop (PHY-012).
   * `anchor` é a ancoragem de `anchorOf`, medida no início do arrasto. Tamanho em unidades do nó.
   */
  function dragShape(n: NodeView, key: string, w: Pt, anchor: Pt | null, coalesce: string) {
    const wp = world(n);
    const lp = toLocal(wp, w.x, w.y);
    if (n.kind === 'CollisionPolygon2D') {
      const pts = ((n.props.points as [number, number][] | undefined) ?? []).map((q) => [q[0], q[1]] as [number, number]);
      const i = Number(key.slice(1));
      if (pts[i]) pts[i] = [round2(lp.x), round2(lp.y)];
      scene.setProp(n.uid, 'points', pts, coalesce);
      return;
    }
    const [w0, h0] = (n.props.size as [number, number]) ?? [32, 32];
    if (n.props.shape === 'circle') {
      const r = Math.max(0.5, Math.hypot(lp.x, lp.y));
      scene.setProp(n.uid, 'size', [round2(2 * r), round2(2 * r)], coalesce);
      return;
    }
    if (n.props.shape === 'capsule_y') {
      if (key === 'x') {
        const width = Math.max(1, round2(2 * Math.abs(lp.x)));
        scene.setProp(n.uid, 'size', [width, Math.max(h0, width)], coalesce);
      } else {
        const height = Math.max(w0, round2(2 * Math.abs(lp.y)));
        scene.setProp(n.uid, 'size', [w0, height], coalesce);
      }
      return;
    }
    if (!anchor) return;
    // Retângulo e segmento: a ancoragem fica no mundo e o centro vai para o meio dos dois pontos.
    const sx = Math.abs(wp.sx) || 1;
    const sy = Math.abs(wp.sy) || 1;
    let mid: Pt;
    let width: number;
    let height = h0;
    if (n.props.shape === 'segment') {
      // Só o comprimento no eixo do nó: a ponta acompanha o eixo, não o mouse fora dele.
      const ax = Math.cos(wp.rot * RAD);
      const ay = Math.sin(wp.rot * RAD);
      const along = (w.x - anchor.x) * ax + (w.y - anchor.y) * ay;
      mid = { x: anchor.x + ax * (along / 2), y: anchor.y + ay * (along / 2) };
      width = Math.max(1, Math.abs(along) / sx);
    } else {
      const d = rotateBy(w.x - anchor.x, w.y - anchor.y, -wp.rot * RAD);
      mid = { x: (anchor.x + w.x) / 2, y: (anchor.y + w.y) / 2 };
      width = Math.max(1, Math.abs(d.x) / sx);
      height = Math.max(1, Math.abs(d.y) / sy);
    }
    const pos = toLocal(parentWorld(n), mid.x, mid.y);
    scene.setProps(n.uid, { size: [round2(width), round2(height)], position: [round2(pos.x), round2(pos.y)] }, coalesce);
  }

  /** Geometria de um sprite no mundo, igual à do núcleo Rust (centro, tamanho e rotação). */
  function geom(n: NodeView) {
    const w0 = world(n);
    const size2 = (n.props.size as [number, number]) ?? [96, 96];
    const off = (n.props.offset as [number, number]) ?? [0, 0];
    const w = Math.abs(size2[0] * w0.sx);
    const h = Math.abs(size2[1] * w0.sy);
    const centered = n.props.centered !== false;
    const lx = centered ? off[0] * w0.sx : off[0] * w0.sx + w / 2;
    const ly = centered ? off[1] * w0.sy : off[1] * w0.sy + h / 2;
    const c = Math.cos(w0.rot * RAD);
    const s = Math.sin(w0.rot * RAD);
    const cx = w0.x + c * lx - s * ly;
    const cy = w0.y + s * lx + c * ly;
    const corner = (ux: number, uy: number): Pt => ({ x: cx + c * ux - s * uy, y: cy + s * ux + c * uy });
    return {
      cx,
      cy,
      w,
      h,
      rot: w0.rot * RAD,
      corners: [corner(-w / 2, -h / 2), corner(w / 2, -h / 2), corner(w / 2, h / 2), corner(-w / 2, h / 2)],
      /** Ponto de canto local (hx, hy em -1 ou 1) no mundo. */
      at: (hx: number, hy: number): Pt => corner((hx * w) / 2, (hy * h) / 2),
    };
  }

  // ---------- Renderização ----------
  let spriteTextures: string[] = [];
  let gameFrame = $state<{ w: number; h: number } | null>(null);

  /** Moldura da tela do jogo em unidades de mundo, conforme a proporção escolhida. */
  function frameSize(): { w: number; h: number } {
    const [sw, sh] = scene.data.screen;
    if (app.gameAspect === '16:9') return { w: sw, h: (sw * 9) / 16 };
    if (app.gameAspect === '4:3') return { w: sw, h: (sw * 3) / 4 };
    return { w: sw, h: sh };
  }

  // ---------- Depuração da física (DBG-004) ----------
  /** Desenho de depuração como o WASM o envia (`debug_draw_json`, em unidades de mundo). */
  type RawDebug = {
    shapes: { node: string; role: string; points: [number, number][]; closed: boolean }[];
    contacts: { point: [number, number]; normal: [number, number] }[];
    velocities: { node: string; origin: [number, number]; velocity: [number, number] }[];
  };
  /** Comprimento da seta de velocidade: quantos segundos de movimento ela mostra, e o teto em pixels. */
  const VEL_SECONDS = 0.1;
  const VEL_MAX = 160;
  /** Desenho de depuração da física já em pixels de tela; `null` sem a alternância ou fora da reprodução. */
  let debugOverlay = $state.raw<DebugOverlay>(null);

  function debugFrame(ed: SceneEditor): DebugOverlay {
    const d = JSON.parse(ed.debug_draw_json() as unknown as string) as RawDebug | null;
    if (!d) return null;
    const s = (p: [number, number]): Pt => toScreen(p[0], p[1]);
    return {
      shapes: d.shapes.filter((f) => f.points.length > 1).map((f) => ({ role: f.role, pts: f.points.map(s), closed: f.closed })),
      contacts: d.contacts.map((k) => {
        const p = s(k.point);
        return { p, q: { x: p.x + k.normal[0] * 12, y: p.y + k.normal[1] * 12 } };
      }),
      velocities: d.velocities.map((v) => {
        const from = s(v.origin);
        let dx = v.velocity[0] * VEL_SECONDS * cam.zoom;
        let dy = v.velocity[1] * VEL_SECONDS * cam.zoom;
        const len = Math.hypot(dx, dy);
        if (len > VEL_MAX) {
          dx *= VEL_MAX / len;
          dy *= VEL_MAX / len;
        }
        return { from, to: { x: from.x + dx, y: from.y + dy } };
      }),
    };
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

  /** Envia a cena (ou a reprodução) ao viewport: sprites, texturas e câmera. */
  function draw(t: number) {
    const ed = scene.editor;
    if (!vp || !ed) return;
    const playing = app.playing;
    const selected = playing || mode !== 'cena' ? [] : scene.selection;
    const hovered = playing || mode !== 'cena' ? null : scene.hoveredId;
    const floats = ed.sprites(JSON.stringify(selected), hovered ?? undefined);
    spriteTextures = JSON.parse(ed.sprite_textures() as unknown as string) as string[];
    textures.sync(vp);
    for (let i = 0; i < spriteTextures.length; i++) {
      const uid = spriteTextures[i];
      floats[i * 12 + 7] = uid ? textures.slotOf(uid) : 0;
    }
    // Depuração da física: só na Cena, durante a reprodução, com a alternância ligada (DBG-004).
    debugOverlay = playing && app.physicsDebug && mode === 'cena' ? debugFrame(ed) : null;
    const dpr = window.devicePixelRatio || 1;
    // Centro e zoom (pixels da janela por unidade de mundo) desta vista: o ponteiro usa os mesmos valores.
    let centro: Pt;
    let zoom: number;
    if (mode === 'cena') {
      centro = { x: cam.x, y: cam.y };
      zoom = cam.zoom;
      vp.set_camera(cam.x, cam.y, zoom * dpr, t / 1000, scene.data.screen[0], scene.data.screen[1]);
    } else {
      // Jogo: a câmera atual define o centro e o zoom; a moldura cabe na área com margem.
      const frame = frameSize();
      const cur = scene.camera() ?? [scene.data.screen[0] / 2, scene.data.screen[1] / 2, 1];
      const fit = Math.min(size.w / frame.w, size.h / frame.h) * 0.92;
      const z = fit * cur[2] * app.gameScale;
      centro = { x: cur[0], y: cur[1] };
      zoom = z;
      // Moldura em unidades de mundo divididas pelo zoom da câmera: ela fica do tamanho certo na tela.
      vp.set_camera(cur[0], cur[1], z * dpr, t / 1000, frame.w / cur[2], frame.h / cur[2]);
    }
    if (playing && pointerAt) {
      const p = pointerAt;
      ed.pointer_event(p.x, p.y, centro.x + (p.x - size.w / 2) / zoom, centro.y + (p.y - size.h / 2) / zoom);
    }
    vp.set_sprites(floats);
    try {
      vp.render();
    } catch (e) {
      log.error('Erro de renderização: ' + e);
    }
  }

  onMount(() => {
    if (mode === 'cena') {
      viewApi.frameScreen = (instant = false) => frameRect(0, 0, scene.data.screen[0], scene.data.screen[1], 56, instant as boolean);
      viewApi.frameSelection = () => {
        const id = scene.selectedId;
        const b = id ? scene.bounds(id) : null;
        if (!b) return viewApi.frameScreen();
        frameRect(b[0], b[1], b[2], b[3], Math.min(size.w, size.h) * 0.3);
      };
      viewApi.zoomTo = (z: number) => animateTo(cam.x, cam.y, z);
      viewApi.center = () => ({ x: cam.x, y: cam.y });
    }

    let raf = 0;
    let alive = true;
    let last = 0;
    let frames = 0;
    let acc = 0;
    let wasPlaying = false;
    let seenTextures = -1;

    const frame = (t: number) => {
      raf = requestAnimationFrame(frame);
      if (!vp || !active) {
        last = t;
        return;
      }
      const dt = last ? Math.min((t - last) / 1000, 0.1) : 0;
      last = t;
      if (app.playing && mode === 'cena') {
        if (app.paused) {
          // Pausado: só avança quando o Passo a passo pede um quadro.
          if (app.stepRequested) {
            app.stepRequested = false;
            runtime.step(STEP_SECONDS);
            runtime.syncLive();
            dirty = true;
          }
          app.fps = 0;
          frames = 0;
          acc = 0;
        } else {
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
      }
      wasPlaying = app.playing;
      if (textures.version !== seenTextures) {
        seenTextures = textures.version;
        dirty = true;
      }
      if (!dirty) return;
      dirty = false;
      draw(t);
    };

    const ro = new ResizeObserver(resize);
    ro.observe(host);
    resize();
    if (mode === 'cena') viewApi.frameScreen(true);

    (async () => {
      try {
        await scene.ensure();
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

  // ---------- Gizmos (sobreposição em tela) ----------
  const sel = $derived(scene.selected);

  const gizmo = $derived.by<Gizmo>(() => {
    if (mode !== 'cena' || app.playing || !sel || sel.kind !== 'Sprite2D') return null;
    const g = geom(sel);
    const pivot = toScreen(g.cx, g.cy);
    return {
      pivot,
      corners: g.corners.map((c) => toScreen(c.x, c.y)),
      rotR: ROT_RING,
    };
  });

  /**
   * Moldura da tela de cada câmera atual (VPT-005), em unidades de mundo: a tela do jogo dividida pelo zoom
   * da câmera. É o que a Visão do jogo mostra; o zoom da vista do editor não entra aqui.
   */
  const frames = $derived.by<Pt[][]>(() => {
    if (mode !== 'cena' || app.playing) return [];
    const f = frameSize();
    return scene.data.nodes
      .filter((n) => n.kind === 'Camera2D' && n.props.current !== false)
      .map((n) => {
        const w = world(n);
        const zoom = Math.max(Number(n.props.zoom) || 1, 0.05);
        const hw = f.w / zoom / 2;
        const hh = f.h / zoom / 2;
        return [
          toScreen(w.x - hw, w.y - hh),
          toScreen(w.x + hw, w.y - hh),
          toScreen(w.x + hw, w.y + hh),
          toScreen(w.x - hw, w.y + hh),
        ];
      });
  });

  /** Formas de colisão na edição: recalculadas quando a cena muda (VPT-005). */
  const colliderPts = $derived.by<Pt[][]>(() => {
    if (mode !== 'cena' || app.playing) return [];
    scene.revision;
    return scene.shapeOutlines().map((o) => o.points.map(([x, y]) => toScreen(x, y)));
  });

  /** Raio de atenuação de cada som 2D, em pixels de tela: `max_distance` no zoom atual (VPT-005). */
  const audioRadii = $derived.by<AudioRadius[]>(() => {
    if (mode !== 'cena' || app.playing) return [];
    return scene.data.nodes
      .filter((n) => n.kind === 'AudioStreamPlayer2D' && n.visible)
      .map((n) => ({
        c: toScreen(n.world.x, n.world.y),
        r: Math.max(0, Number(n.props.max_distance) || 0) * cam.zoom,
        sel: scene.isSelected(n.uid),
      }));
  });

  /** Contorno e alças da forma de colisão selecionada (PHY-012). */
  const shapeGizmo = $derived.by<ShapeGizmo>(() => {
    if (mode !== 'cena' || app.playing || !sel || !SHAPE_KINDS.includes(sel.kind)) return null;
    scene.revision;
    const outline = scene.shapeOutlines().find((o) => o.uid === sel.uid);
    const wp = world(sel);
    return {
      pts: outline ? outline.points.map(([x, y]) => toScreen(x, y)) : [],
      handles: shapeHandles(sel).map((h) => {
        const at = nodeToWorld(wp, h.lx, h.ly);
        return toScreen(at.x, at.y);
      }),
    };
  });

  const gameFramePts = $derived.by<Pt[] | null>(() => {
    if (mode !== 'jogo') return null;
    const frame = frameSize();
    // Mesmo cálculo de draw(): a moldura ocupa frame.w * fit * escala pixels, centrada na vista.
    const k = Math.min(size.w / frame.w, size.h / frame.h) * 0.92 * app.gameScale;
    const hw = (frame.w * k) / 2;
    const hh = (frame.h * k) / 2;
    const cx = size.w / 2;
    const cy = size.h / 2;
    return [
      { x: cx - hw, y: cy - hh },
      { x: cx + hw, y: cy - hh },
      { x: cx + hw, y: cy + hh },
      { x: cx - hw, y: cy + hh },
    ];
  });

  // ---------- Interação ----------
  type Drag =
    | { kind: 'pan'; sx: number; sy: number; cx: number; cy: number }
    | { kind: 'move'; uids: string[]; start: Record<string, Pt>; primary: string; grab: Pt; key: string }
    | { kind: 'rotate'; uid: string; cx: number; cy: number; a0: number; r0: number; key: string }
    | { kind: 'scale'; uid: string; cx: number; cy: number; d0: number; s0: [number, number]; key: string }
    | { kind: 'rect'; uid: string; ox: number; oy: number; key: string }
    | { kind: 'shape'; uid: string; handle: string; anchor: Pt | null; key: string }
    | { kind: 'box'; x0: number; y0: number; additive: boolean };
  let drag: Drag | null = null;
  let contextAt: Pt | null = null;

  const snapOn = (e: { ctrlKey: boolean }) => app.snap !== e.ctrlKey;
  const snapTo = (v: number, grid: number) => Math.round(v / grid) * grid;

  /** Alça sob o ponto de tela, se houver, para a ferramenta atual. */
  function handleAt(p: Pt): { kind: 'move' | 'rotate' | 'scale' | 'rect'; hx?: number; hy?: number } | null {
    const g = gizmo;
    if (!g) return null;
    if (app.tool === 'move' && Math.hypot(p.x - g.pivot.x, p.y - g.pivot.y) <= HANDLE_HIT) return { kind: 'move' };
    if (app.tool === 'rotate' && Math.abs(Math.hypot(p.x - g.pivot.x, p.y - g.pivot.y) - ROT_RING) <= HANDLE_HIT / 2 + 2)
      return { kind: 'rotate' };
    if (app.tool === 'scale' || app.tool === 'rect') {
      const hs = [
        { hx: -1, hy: -1 },
        { hx: 1, hy: -1 },
        { hx: 1, hy: 1 },
        { hx: -1, hy: 1 },
      ];
      if (!sel || sel.kind !== 'Sprite2D') return null;
      const g2 = geom(sel);
      for (const h of hs) {
        const c = toScreen(g2.at(h.hx, h.hy).x, g2.at(h.hx, h.hy).y);
        if (Math.abs(c.x - p.x) <= HANDLE_HIT && Math.abs(c.y - p.y) <= HANDLE_HIT) {
          return { kind: app.tool === 'scale' ? 'scale' : 'rect', hx: h.hx, hy: h.hy };
        }
      }
    }
    return null;
  }

  function down(e: PointerEvent) {
    focus.panel = isScene ? 'viewport' : 'game';
    host.focus();
    const p = local(e);
    // Reprodução: botões do mouse e toques viram códigos de entrada (INP-001). A captura garante o soltar fora do canvas.
    if (app.playing && isScene) {
      canvas.setPointerCapture(e.pointerId);
      runtime.pointerButtons(e.buttons);
    }
    if (mode === 'jogo') {
      if (e.button === 1 || e.button === 2 || spaceDown || app.tool === 'hand') {
        canvas.setPointerCapture(e.pointerId);
        drag = { kind: 'pan', sx: p.x, sy: p.y, cx: cam.x, cy: cam.y };
        panning = true;
      }
      return;
    }
    const wantsPan = e.button === 1 || e.button === 2 || (e.button === 0 && (spaceDown || app.tool === 'hand'));
    if (wantsPan) {
      canvas.setPointerCapture(e.pointerId);
      cancelAnimationFrame(camAnim);
      drag = { kind: 'pan', sx: p.x, sy: p.y, cx: cam.x, cy: cam.y };
      panning = true;
      // Botão direito sem arrastar abre o menu de contexto (VPT-004).
      panMoved = false;
      contextAt = e.button === 2 ? toWorld(p.x, p.y) : null;
      return;
    }
    if (e.button !== 0 || app.playing) return;
    const w = toWorld(p.x, p.y);

    // Alça de forma de colisão da seleção (PHY-012): vem antes das alças de transformação.
    const sh = shapeHandleAt(p);
    if (sh && sel) {
      canvas.setPointerCapture(e.pointerId);
      drag = { kind: 'shape', uid: sel.uid, handle: sh.key, anchor: anchorOf(sel, sh.key), key: 'drag:' + sel.uid + ':' + performance.now() };
      dragKind = 'shape';
      return;
    }

    // Alça da ferramenta sob o mouse.
    const h = handleAt(p);
    if (h && sel) {
      canvas.setPointerCapture(e.pointerId);
      const key = 'drag:' + sel.uid + ':' + performance.now();
      if (h.kind === 'move') {
        startMove(p, w, key);
      } else if (h.kind === 'rotate') {
        const g = geom(sel);
        drag = { kind: 'rotate', uid: sel.uid, cx: g.cx, cy: g.cy, a0: Math.atan2(w.y - g.cy, w.x - g.cx), r0: Number(sel.props.rotation ?? 0), key };
        dragKind = 'rotate';
      } else if (h.kind === 'scale') {
        const g = geom(sel);
        const d0 = Math.max(1, Math.hypot(w.x - g.cx, w.y - g.cy));
        drag = { kind: 'scale', uid: sel.uid, cx: g.cx, cy: g.cy, d0, s0: [...(sel.props.scale as [number, number])] as [number, number], key };
        dragKind = 'scale';
      } else {
        const g = geom(sel);
        const opp = g.at(-(h.hx ?? 1), -(h.hy ?? 1));
        drag = { kind: 'rect', uid: sel.uid, ox: opp.x, oy: opp.y, key };
        dragKind = 'rect';
      }
      return;
    }

    const hit = scene.pick(w.x, w.y);
    if (hit) {
      const additive = e.shiftKey || e.ctrlKey;
      if (additive && scene.isSelected(hit)) {
        scene.select(hit, true);
        return;
      }
      if (e.altKey && scene.isSelected(hit)) {
        // Alt-arrastar duplica a seleção e move a cópia (VPT-003).
        duplicateSelected();
      } else {
        scene.select(hit, additive);
      }
      if (!scene.isSelected(hit)) return;
      if (app.tool === 'hand') return;
      canvas.setPointerCapture(e.pointerId);
      startMove(p, w, 'drag:' + hit + ':' + performance.now());
    } else {
      // Clique no vazio: caixa de seleção (mesmo com a ferramenta Mover).
      canvas.setPointerCapture(e.pointerId);
      drag = { kind: 'box', x0: p.x, y0: p.y, additive: e.shiftKey || e.ctrlKey };
      box = { x0: p.x, y0: p.y, x1: p.x, y1: p.y };
      dragKind = 'box';
    }
  }

  function startMove(_p: Pt, w: Pt, key: string) {
    const uids = scene.selection.filter((u) => !scene.isLocked(u));
    if (!uids.length) return;
    const start: Record<string, Pt> = {};
    for (const u of uids) {
      const n = scene.node(u);
      if (n) start[u] = { x: n.world.x, y: n.world.y };
    }
    const primary = scene.selectedId && uids.includes(scene.selectedId) ? scene.selectedId : uids[0];
    drag = { kind: 'move', uids, start, primary, grab: w, key };
    dragKind = 'move';
  }

  function move(e: PointerEvent) {
    const p = local(e);
    pointerAt = p;
    if (drag?.kind === 'pan') {
      if (Math.hypot(p.x - drag.sx, p.y - drag.sy) > 4) panMoved = true;
      cam.x = drag.cx - (p.x - drag.sx) / cam.zoom;
      cam.y = drag.cy - (p.y - drag.sy) / cam.zoom;
      return;
    }
    if (drag?.kind === 'box') {
      box = { x0: drag.x0, y0: drag.y0, x1: p.x, y1: p.y };
      return;
    }
    if (drag?.kind === 'shape') {
      const n = scene.node(drag.uid);
      if (!n) return;
      let w = toWorld(p.x, p.y);
      if (snapOn(e)) w = { x: snapTo(w.x, app.gridSize), y: snapTo(w.y, app.gridSize) };
      dragShape(n, drag.handle, w, drag.anchor, drag.key);
      dirtyDrag();
      return;
    }
    if (drag?.kind === 'move') {
      const w = toWorld(p.x, p.y);
      const pStart = drag.start[drag.primary];
      let dx = w.x - drag.grab.x;
      let dy = w.y - drag.grab.y;
      if (snapOn(e)) {
        const nx = snapTo(pStart.x + dx, app.gridSize);
        const ny = snapTo(pStart.y + dy, app.gridSize);
        dx = nx - pStart.x;
        dy = ny - pStart.y;
      }
      for (const uid of drag.uids) {
        const n = scene.node(uid);
        const s = drag.start[uid];
        if (!n || !s) continue;
        const lp = toLocal(parentWorld(n), s.x + dx, s.y + dy);
        scene.setProp(uid, 'position', [round2(lp.x), round2(lp.y)], drag.key);
      }
      dirtyDrag();
      return;
    }
    if (drag?.kind === 'rotate') {
      const w = toWorld(p.x, p.y);
      let a = Math.atan2(w.y - drag.cy, w.x - drag.cx) - drag.a0;
      let deg = drag.r0 + a / RAD;
      if (snapOn(e)) deg = snapTo(deg, app.rotationSnap);
      scene.setProp(drag.uid, 'rotation', round2(deg), drag.key);
      dirtyDrag();
      return;
    }
    if (drag?.kind === 'scale') {
      const w = toWorld(p.x, p.y);
      const f = Math.max(0.01, Math.hypot(w.x - drag.cx, w.y - drag.cy) / drag.d0);
      scene.setProp(drag.uid, 'scale', [round2(drag.s0[0] * f), round2(drag.s0[1] * f)], drag.key);
      dirtyDrag();
      return;
    }
    if (drag?.kind === 'rect') {
      let w = toWorld(p.x, p.y);
      if (snapOn(e)) w = { x: snapTo(w.x, app.gridSize), y: snapTo(w.y, app.gridSize) };
      const n = scene.node(drag.uid);
      if (!n) return;
      // Retângulo com a alça oposta fixa: o centro é o meio dos dois cantos; o tamanho vem da
      // distância entre eles no espaço local do nó (sem rotação e sem escala).
      const d = rotateBy(w.x - drag.ox, w.y - drag.oy, -world(n).rot * RAD);
      const wSize = Math.max(1, Math.abs(d.x) / (Math.abs(world(n).sx) || 1));
      const hSize = Math.max(1, Math.abs(d.y) / (Math.abs(world(n).sy) || 1));
      const lp = toLocal(parentWorld(n), (drag.ox + w.x) / 2, (drag.oy + w.y) / 2);
      scene.setProps(drag.uid, { size: [round2(wSize), round2(hSize)], position: [round2(lp.x), round2(lp.y)] }, drag.key);
      dirtyDrag();
    }
    if (!app.playing && !drag && mode === 'cena') {
      const w = toWorld(p.x, p.y);
      scene.hoveredId = scene.pick(w.x, w.y);
    }
  }

  const round2 = (v: number) => Math.round(v * 100) / 100;
  function rotateBy(x: number, y: number, rad: number): Pt {
    const c = Math.cos(rad);
    const s = Math.sin(rad);
    return { x: x * c - y * s, y: x * s + y * c };
  }
  /** Marca o quadro para redesenho durante o arrasto (o efeito já observa a revisão da cena). */
  function dirtyDrag() {
    dirty = true;
  }

  function up(e: PointerEvent) {
    if (app.playing && isScene) runtime.pointerButtons(e.buttons);
    if (drag?.kind === 'box' && box) {
      const a = toWorld(Math.min(box.x0, box.x1), Math.min(box.y0, box.y1));
      const b = toWorld(Math.max(box.x0, box.x1), Math.max(box.y0, box.y1));
      if (Math.abs(box.x1 - box.x0) > 3 || Math.abs(box.y1 - box.y0) > 3) {
        const hits = scene.data.nodes
          .filter((n) => n.kind === 'Sprite2D' && n.visible)
          .filter((n) => {
            const bb = scene.bounds(n.uid);
            return !!bb && bb[2] >= a.x && bb[0] <= b.x && bb[3] >= a.y && bb[1] <= b.y;
          })
          .map((n) => n.uid);
        if (!drag.additive) scene.selection = hits;
        else scene.selection = [...new Set([...scene.selection, ...hits])];
      } else if (!drag.additive) {
        scene.select(null);
      }
    }
    if (drag?.kind === 'move' || drag?.kind === 'rotate' || drag?.kind === 'scale' || drag?.kind === 'rect' || drag?.kind === 'shape') {
      scene.endGroup();
    }
    if (e.type === 'pointerup' && e.button === 2 && contextAt && !panMoved) openContext(e);
    drag = null;
    box = null;
    panning = false;
    dragKind = '';
    contextAt = null;
  }
  let panMoved = false;

  function openContext(e: PointerEvent) {
    const p = local(e);
    const w = toWorld(p.x, p.y);
    const target = scene.pick(w.x, w.y);
    if (target && !scene.isSelected(target)) scene.select(target);
    const items: MenuItem[] = [
      ...scene.types
        .filter((t) => t.name !== 'Node' && t.category !== 'Comportamento (protótipo)')
        .map((t) => ({
          label: 'Criar ' + t.name,
          run: () => {
            const uid = scene.add(t.name, null);
            if (uid && t.name.endsWith('2D')) scene.setProp(uid, 'position', [Math.round(w.x), Math.round(w.y)]);
          },
        })),
      'sep',
      { label: 'Colar', keys: 'Ctrl+V', enabled: scene.hasClipboard, run: pasteClipboard },
      { label: 'Duplicar', keys: 'Ctrl+D', enabled: scene.selection.length > 0, run: duplicateSelected },
      { label: 'Excluir', keys: 'Delete', enabled: scene.selection.length > 0, run: deleteSelected },
      'sep',
      { label: 'Enquadrar seleção', keys: 'F', enabled: !!scene.selectedId, run: () => viewApi.frameSelection() },
      { label: 'Enquadrar tela do jogo', keys: 'Home', run: () => viewApi.frameScreen() },
    ];
    showContextMenu(e, items);
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
    if (!e.dataTransfer?.types.includes('application/x-nexo-shape') || app.playing || mode !== 'cena') return;
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
    const shape = Number(data) as 0 | 1 | 2;
    addShape(shape);
    const uid = scene.selectedId;
    if (uid) scene.setProp(uid, 'position', [Math.round(w.x), Math.round(w.y)]);
  }

  const label = $derived.by(() => {
    if (!sel || app.playing || mode !== 'cena' || !gizmo) return null;
    const top = gizmo.corners.reduce((a, c) => (c.y < a.y ? c : a), gizmo.corners[0]);
    return { x: top.x, y: top.y };
  });
  const cursor = $derived(
    panning
      ? 'grabbing'
      : mode === 'jogo'
        ? 'default'
        : spaceDown || app.tool === 'hand'
          ? 'grab'
          : dragKind === 'move' || dragKind === 'shape'
            ? 'move'
            : dragKind === 'rotate'
              ? 'grabbing'
              : scene.hoveredId
                ? 'pointer'
                : 'default',
  );
  const isScene = $derived(mode === 'cena');
</script>

<svelte:window onkeyup={keyup} onblur={() => (spaceDown = false)} />

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="viewport"
  class:drop={dropActive}
  class:playing={app.playing && isScene}
  class:hidden={!active}
  class:focused={focus.panel === (isScene ? 'viewport' : 'game')}
  bind:this={host}
  use:focusPanel={isScene ? 'viewport' : 'game'}
  tabindex="-1"
  role="application"
  aria-label={isScene ? 'Viewport da cena' : 'Visão do jogo'}
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
    onpointerleave={() => {
      pointerAt = null;
      if (!drag && isScene) scene.hoveredId = null;
    }}
    onwheel={wheel}
  ></canvas>

  <ViewportOverlay
    tool={app.tool}
    gizmo={app.playing ? null : gizmo}
    frames={frames}
    box={box}
    frame={gameFramePts}
    colliders={colliderPts}
    shape={shapeGizmo}
    audio={audioRadii}
    debug={debugOverlay}
  />

  {#if label && sel}
    <div class="sel-label" style="transform: translate({label.x}px, {label.y - 12}px) translate(-50%, -100%)">
      <span class="name">{sel.name}</span>
      <span class="mono">{sel.kind}</span>
    </div>
  {/if}

  {#if isScene}
    <div class="toolbar">
      <button class="icon-btn" class:active={app.tool === 'hand'} title="Mão (Q) · ou segure Espaço" onclick={() => (app.tool = 'hand')}><Hand size={15} /></button>
      <button class="icon-btn" class:active={app.tool === 'move'} title="Mover (W)" onclick={() => (app.tool = 'move')}><Move size={15} /></button>
      <button class="icon-btn" class:active={app.tool === 'rotate'} title="Girar (E)" onclick={() => (app.tool = 'rotate')}><RotateCw size={15} /></button>
      <button class="icon-btn" class:active={app.tool === 'scale'} title="Escala (R)" onclick={() => (app.tool = 'scale')}><Maximize size={15} /></button>
      <button class="icon-btn" class:active={app.tool === 'rect'} title="Retângulo (T)" onclick={() => (app.tool = 'rect')}><Square size={15} /></button>
      <span class="div"></span>
      <button class="icon-btn" class:active={app.snap} title="Snap de grade ({app.gridSize} px) · segure Ctrl para alternar" onclick={() => (app.snap = !app.snap)}>
        <span class="snap-mark mono">#</span>
      </button>
      <button
        class="icon-btn"
        class:active={app.physicsDebug}
        title="Depuração de física: formas, contatos e velocidades na reprodução (sessão)"
        aria-pressed={app.physicsDebug}
        onclick={() => (app.physicsDebug = !app.physicsDebug)}
      >
        <Bug size={15} />
      </button>
      <span class="div"></span>
      <button class="icon-btn" title="Enquadrar seleção (F)" onclick={() => viewApi.frameSelection()}><Scan size={15} /></button>
      <button class="icon-btn" title="Enquadrar tela do jogo (Home)" onclick={() => viewApi.frameScreen()}><Monitor size={15} /></button>
      <span class="div"></span>
      <button class="zoom mono" title="Voltar para 100% (Ctrl+1)" onclick={() => viewApi.zoomTo(1)}>{Math.round(cam.zoom * 100)}%</button>
    </div>
  {:else}
    <div class="toolbar">
      <label class="field-label">
        Proporção
        <select class="field-select" bind:value={app.gameAspect}>
          <option value="16:9">16:9</option>
          <option value="4:3">4:3</option>
          <option value="livre">Livre</option>
        </select>
      </label>
      <span class="div"></span>
      <label class="field-label">
        Escala
        <input type="range" min="0.25" max="3" step="0.05" bind:value={app.gameScale} aria-label="Escala da visão do jogo" />
        <span class="mono">{Math.round(app.gameScale * 100)}%</span>
      </label>
    </div>
  {/if}

  {#if app.playing && isScene}
    <VirtualJoysticks />
    <div class="playing-badge" transition:fade={{ duration: 160 }}>
      <span class="dot" class:paused={app.paused}></span> {app.paused ? 'Pausado' : 'Jogando'} <kbd>Esc</kbd> para parar
    </div>
  {/if}

  {#if status === 'ready' && scene.data.nodes.length === 0 && !app.playing && isScene}
    <div class="empty" transition:fade={{ duration: 200 }}>
      <strong>Sua cena está vazia</strong>
      <span>Crie um nó com o botão direito na hierarquia, ou pressione <kbd>Ctrl</kbd> <kbd>K</kbd>.</span>
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
    background: var(--bg-1);
    transition: box-shadow var(--t-med) var(--ease);
  }
  .viewport.hidden {
    display: none;
  }
  .viewport.drop {
    box-shadow: inset 0 0 0 2px var(--accent);
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
    padding: 2px;
    border-radius: var(--r-ctl);
    background: var(--bg-overlay);
    backdrop-filter: blur(12px);
    box-shadow: 0 0 0 1px var(--line), var(--shadow-float);
    color: var(--text);
  }
  .toolbar .icon-btn {
    color: var(--text-dim);
  }
  .toolbar .icon-btn:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
  .toolbar .icon-btn.active {
    color: var(--accent-hi);
    background: var(--accent-soft);
  }
  .snap-mark {
    font-size: var(--fs-12);
    font-weight: 600;
  }
  .div {
    width: 1px;
    height: 18px;
    margin: 0 4px;
    background: var(--line);
  }
  .zoom {
    height: 28px;
    min-width: 52px;
    padding: 0 8px;
    border-radius: var(--r-ctl);
    font-size: var(--fs-12);
    color: var(--text-dim);
  }
  .zoom:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
  .field-label {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 6px;
    font-size: var(--fs-12);
    color: var(--text-dim);
  }
  .field-select {
    height: 22px;
    padding: 0 4px;
    border-radius: var(--r-ctl);
    background: var(--bg-2);
    color: var(--text);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .sel-label {
    position: absolute;
    left: 0;
    top: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 6px;
    border-radius: var(--r-ctl);
    font-size: var(--fs-11);
    font-weight: 500;
    white-space: nowrap;
    color: white;
    background: var(--accent);
    pointer-events: none;
  }
  .sel-label .mono {
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
    padding: 4px 8px 4px 10px;
    border-radius: var(--r-ctl);
    font-size: var(--fs-12);
    font-weight: 600;
    color: var(--text);
    background: var(--bg-overlay);
    backdrop-filter: blur(12px);
    box-shadow: 0 0 0 1px var(--line), var(--shadow-float);
  }
  .playing-badge kbd {
    color: var(--text-dim);
    background: var(--bg-3);
    border-color: var(--line);
  }
  .dot.paused {
    background: var(--warning);
    animation: none;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--success);
    animation: pulse 1.4s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      box-shadow: 0 0 0 5px color-mix(in srgb, var(--success) 25%, transparent);
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
    padding: 12px 18px;
    border-radius: var(--r-dlg);
    text-align: center;
    color: var(--text-dim);
    background: var(--bg-overlay);
    backdrop-filter: blur(12px);
    box-shadow: 0 0 0 1px var(--line), var(--shadow-float);
    pointer-events: none;
  }
  .empty strong {
    color: var(--text);
  }
  .empty kbd {
    color: var(--text-dim);
    background: var(--bg-3);
    border-color: var(--line);
  }
  .state {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--text-dim);
    pointer-events: none;
  }
  .state.error {
    color: var(--danger);
  }
  .state.error strong {
    color: var(--text);
  }
  .state.error span {
    max-width: 520px;
    text-align: center;
    font-size: var(--fs-12);
    color: var(--text-dim);
  }
</style>
