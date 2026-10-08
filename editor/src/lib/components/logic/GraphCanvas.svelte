<script lang="ts">
  import { logic } from '$lib/logic/store.svelte';
  import { logicDebug } from '$lib/logic/debug.svelte';
  import { wireKey } from '$lib/logic/highlight';
  import { scene } from '$lib/state/scene.svelte';
  import { focus, focusPanel } from '$lib/state/focus.svelte';
  import { showContextMenu } from '$lib/state/contextmenu.svelte';
  import { log } from '$lib/state/log.svelte';
  import { familyOf, type PinFilter } from '$lib/logic/library';
  import { isPropertyNode, propertyOfType, propertiesOfType, defaultValue } from '$lib/logic/properties';
  import {
    COMMENT_MIN,
    ROW_H,
    isReroute,
    nodeSize,
    normRect,
    pinPoint,
    rectsTouch,
    showsDetail,
    toWorld,
    wirePath,
    zoomAt,
    type Camera,
    type Rect,
    type Vec,
  } from '$lib/logic/geometry';
  import {
    TIPO_NOME,
    pinKey,
    pinStyle,
    refusalText,
    textOf,
    type GraphNode,
    type PinDef,
    type PinRef,
    type PinType,
    type Refusal,
    type Value,
    type Wire,
  } from '$lib/logic/model';
  import NodeSearch from './NodeSearch.svelte';
  import PinShape from './PinShape.svelte';
  import PinValue from './PinValue.svelte';
  import Minimap from './Minimap.svelte';
  import PropSelect from './PropSelect.svelte';

  /** Arrasto em andamento: pan, mover seleção, fio a partir de um pino, caixa de seleção ou redimensionar. */
  type Drag =
    | { kind: 'pan'; sx: number; sy: number; cam: Camera; moved: boolean }
    | {
        kind: 'mover';
        sx: number;
        sy: number;
        zoom: number;
        nos: { id: number; x: number; y: number }[];
        caixas: { id: number; x: number; y: number; w: number; h: number; text: string }[];
        moved: boolean;
      }
    | {
        kind: 'fio';
        de: PinRef;
        saida: boolean;
        tipo: PinType | null;
        existente: PinRef | null;
        wx: number;
        wy: number;
        sx: number;
        sy: number;
        moved: boolean;
      }
    | {
        kind: 'caixa';
        a: Vec;
        b: Vec;
        aditivo: boolean;
        base: { nos: number[]; caixas: number[] };
      }
    | {
        kind: 'redim';
        id: number;
        x: number;
        y: number;
        w0: number;
        h0: number;
        text: string;
        sx: number;
        sy: number;
        zoom: number;
        moved: boolean;
      };

  /** Menu de busca aberto: onde fica na tela, onde cai no mundo e o fio que o motivou (se houver). */
  interface Busca {
    x: number;
    y: number;
    wx: number;
    wy: number;
    de: PinRef | null;
    saida: boolean;
    tipo: PinType | null;
  }

  let host = $state<HTMLDivElement>();
  let drag = $state.raw<Drag | null>(null);
  let busca = $state<Busca | null>(null);
  let espaco = $state(false);
  /** For each pin: the refusal of the dragged wire there, or null when the link is valid (VIS-013). */
  let targets = $state.raw<Map<string, Refusal | null>>(new Map());

  const cam = $derived(logic.camera);
  /** Semantic zoom (VIS-014): when false, nodes drop their labels and value fields, and comments show a preview. */
  const detailed = $derived(showsDetail(cam.zoom));
  const nodes = $derived(logic.snap.nodes);
  const wires = $derived(logic.snap.wires);
  const nodePorId = $derived(new Map(nodes.map((n) => [n.id, n] as const)));
  /** Tipo resolvido de cada pino (chave: nó, direção e pino). */
  const tipoDe = $derived(
    new Map(logic.snap.pins.map((p) => [pinKey(p.node, (p.output ? 'o:' : 'i:') + p.pin), p.ty] as const)),
  );
  const ligadas = $derived(new Set(wires.map((w) => pinKey(w.to.node, 'i:' + w.to.pin))));
  const erros = $derived.by(() => {
    const out = new Map<number, string>();
    for (const e of logic.errors) {
      if (e.node === null) continue;
      const message = refusalText(e);
      out.set(e.node, out.has(e.node) ? `${out.get(e.node)}\n${message}` : message);
    }
    return out;
  });
  const ativo = $derived(logic.active);
  // The trace is per instance: the open graph shows the trace of the chosen instance, if it runs this script.
  const uidRastro = $derived(
    ativo && logicDebug.selecionada && scene.byUid.get(logicDebug.selecionada)?.props.script === ativo
      ? logicDebug.selecionada
      : null,
  );
  // What lights up now: the nodes and flow wires of the followed instance that ran in the last LIT_MS (VIS-030).
  const litNodes = $derived(new Set(uidRastro && logicDebug.lit.uid === uidRastro ? logicDebug.lit.nodes : []));
  const litWires = $derived(new Set(uidRastro && logicDebug.lit.uid === uidRastro ? logicDebug.lit.wires : []));
  const pontosDeParada = $derived(new Set(ativo ? logicDebug.breakpointsOf(ativo) : []));
  const valores = $derived<Record<number, Value>>(uidRastro ? (logicDebug.valores[uidRastro] ?? {}) : {});

  /** Ponto de um pino no mundo, ou null se o nó não existe. */
  function pinoNoMundo(ref: PinRef, saida: boolean): Vec | null {
    const n = nodePorId.get(ref.node);
    if (!n) return null;
    return pinPoint(n.position, n.kind, logic.defsByKind.get(n.kind), ref.pin, saida);
  }

  function tipoDoFio(w: Wire): PinType | null {
    return tipoDe.get(pinKey(w.from.node, 'o:' + w.from.pin)) ?? null;
  }

  /** Caminhos de todos os fios, no mundo. */
  const caminhos = $derived(
    wires.flatMap((w, i) => {
      const a = pinoNoMundo(w.from, true);
      const b = pinoNoMundo(w.to, false);
      if (!a || !b) return [];
      const ty = tipoDoFio(w);
      return [
        {
          i,
          d: wirePath(a, b),
          fluxo: ty === 'Flow',
          cor: pinStyle(ty).color,
          // Lit only if this wire was taken: a branch that did not run leaves its wire dark (VIS-030).
          quente: litWires.has(wireKey(w.from.node, w.from.pin, w.to.node)),
          sel: logic.selWire?.from.node === w.from.node &&
            logic.selWire?.from.pin === w.from.pin &&
            logic.selWire?.to.node === w.to.node &&
            logic.selWire?.to.pin === w.to.pin,
        },
      ];
    }),
  );

  /** Fio que está sendo arrastado, em coordenadas do mundo. */
  const arrastado = $derived.by(() => {
    if (drag?.kind !== 'fio') return null;
    const p = pinoNoMundo(drag.de, drag.saida);
    if (!p) return null;
    const cur: Vec = [drag.wx, drag.wy];
    return {
      d: drag.saida ? wirePath(p, cur) : wirePath(cur, p),
      fluxo: drag.tipo === 'Flow',
      cor: pinStyle(drag.tipo).color,
    };
  });

  /** Highlight of a pin during a drag: 'aceita' if the link is valid, 'recusa' if not, '' outside a drag. */
  function alvoDoArrasto(node: number, pin: string, saida: boolean): 'aceita' | 'recusa' | '' {
    const refusal = targets.get(pinKey(node, (saida ? 'o:' : 'i:') + pin));
    if (refusal === undefined) return '';
    return refusal === null ? 'aceita' : 'recusa';
  }

  /** Title of a refused pin during a drag: the refusal and its suggestion (VIS-013). */
  function refusalOnPin(node: number, pin: string, output: boolean): string | undefined {
    const refusal = targets.get(pinKey(node, (output ? 'o:' : 'i:') + pin));
    return refusal ? refusalText(refusal) : undefined;
  }

  /** Refusal for a link between two pins on the same side: output to output, or input to input (VIS-013). */
  function sameSideRefusal(fromOutput: boolean): Refusal {
    return fromOutput
      ? { message: 'Uma saída só liga a uma entrada', suggestion: 'Solte numa entrada' }
      : { message: 'Uma entrada só liga a uma saída', suggestion: 'Solte numa saída' };
  }

  /**
   * Tests the dragged wire against every pin with the rules of the link: the opposite side is checked for types,
   * occupied inputs and cycles; the same side is refused by direction. Done once per drag: the graph does not change
   * while the wire is dragged.
   */
  function evaluateTargets(d: Extract<Drag, { kind: 'fio' }>): Map<string, Refusal | null> {
    const out = new Map<string, Refusal | null>();
    for (const p of logic.snap.pins) {
      const key = pinKey(p.node, (p.output ? 'o:' : 'i:') + p.pin);
      // The pin the wire starts from is not a target.
      if (p.output === d.saida && p.node === d.de.node && p.pin === d.de.pin) continue;
      if (p.output === d.saida) {
        out.set(key, sameSideRefusal(d.saida));
        continue;
      }
      const outro: PinRef = { node: p.node, pin: p.pin };
      // Dropping the wire back on the input it already fills changes nothing, so that input is not occupied.
      const backToSameInput = d.existente?.node === p.node && d.existente.pin === p.pin;
      const refusal = backToSameInput
        ? undefined
        : d.saida
          ? logic.checkConnection(d.de, outro)
          : logic.checkConnection(outro, d.de);
      out.set(key, refusal ?? null);
    }
    return out;
  }

  /** Dica do pino: o último valor que o nó produziu, quando a reprodução com depurador mandou. */
  function dicaDoPino(node: number, saida: boolean): string | undefined {
    if (!saida) return undefined;
    const v = valores[node] as Value | undefined;
    return v ? textOf(v) : undefined;
  }

  function localDe(e: { clientX: number; clientY: number }): Vec {
    const r = host?.getBoundingClientRect();
    return [e.clientX - (r?.left ?? 0), e.clientY - (r?.top ?? 0)];
  }

  function estaDigitando(alvo: EventTarget | null): boolean {
    const t = alvo as HTMLElement | null;
    return !!t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable);
  }

  function teclaBaixo(e: KeyboardEvent) {
    if (e.code === 'Space' && !estaDigitando(e.target)) {
      espaco = true;
      e.preventDefault();
    }
  }

  function teclaCima(e: KeyboardEvent) {
    if (e.code === 'Space') espaco = false;
  }

  // ---------- Ponteiro ----------

  function baixo(e: PointerEvent) {
    if (!logic.active) return;
    host?.focus({ preventScroll: true });
    busca = null;
    const [sx, sy] = localDe(e);
    const [wx, wy] = toWorld(logic.camera, sx, sy);
    const alvo = e.target as HTMLElement;
    const capturar = () => (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);

    if (e.button === 1 || (e.button === 0 && espaco)) {
      drag = { kind: 'pan', sx, sy, cam: { ...logic.camera }, moved: false };
      capturar();
      return;
    }
    if (e.button !== 0) return;

    const pino = alvo.closest<HTMLElement>('[data-pin]');
    if (pino) {
      const node = Number(pino.dataset.node);
      const pin = pino.dataset.pin ?? '';
      const saida = pino.dataset.dir === 'out';
      const tipo = tipoDe.get(pinKey(node, (saida ? 'o:' : 'i:') + pin)) ?? null;
      // Arrastar a partir de uma entrada ligada leva o fio dela: a origem vira o ponto de partida.
      const ligado = !saida ? wires.find((w) => w.to.node === node && w.to.pin === pin) : undefined;
      drag = {
        kind: 'fio',
        de: ligado ? ligado.from : { node, pin },
        saida: ligado ? true : saida,
        tipo,
        existente: ligado ? { node, pin } : null,
        wx,
        wy,
        sx,
        sy,
        moved: false,
      };
      capturar();
      return;
    }

    const fio = alvo.closest<HTMLElement>('[data-wire]');
    if (fio) {
      logic.selectWire(wires[Number(fio.dataset.wire)] ?? null);
      return;
    }

    const alca = alvo.closest<HTMLElement>('[data-resize]');
    if (alca) {
      const id = Number(alca.dataset.resize);
      const c = logic.snap.comments.find((x) => x.id === id);
      if (!c) return;
      logic.select([], [id], false);
      drag = {
        kind: 'redim',
        id,
        x: c.position[0],
        y: c.position[1],
        w0: c.size[0],
        h0: c.size[1],
        text: c.text,
        sx,
        sy,
        zoom: logic.camera.zoom,
        moved: false,
      };
      capturar();
      return;
    }

    const no = alvo.closest<HTMLElement>('[data-node]');
    const caixa = alvo.closest<HTMLElement>('[data-comment]');
    if (no || caixa) {
      const ctrl = e.ctrlKey || e.metaKey;
      if (no) {
        const id = Number(no.dataset.node);
        if (ctrl) logic.select([id], [], true);
        else if (!logic.selNodes.includes(id)) logic.select([id], [], false);
      } else if (caixa) {
        const id = Number(caixa.dataset.comment);
        if (ctrl) logic.select([], [id], true);
        else if (!logic.selComments.includes(id)) logic.select([], [id], false);
      }
      const nos = logic.selNodes
        .map((id) => nodePorId.get(id))
        .filter((n): n is GraphNode => !!n)
        .map((n) => ({ id: n.id, x: n.position[0], y: n.position[1] }));
      const caixas = logic.snap.comments
        .filter((c) => logic.selComments.includes(c.id))
        .map((c) => ({ id: c.id, x: c.position[0], y: c.position[1], w: c.size[0], h: c.size[1], text: c.text }));
      if (nos.length || caixas.length) {
        drag = { kind: 'mover', sx, sy, zoom: logic.camera.zoom, nos, caixas, moved: false };
        capturar();
      }
      return;
    }

    if (e.shiftKey) {
      const base = { nos: [...logic.selNodes], caixas: [...logic.selComments] };
      drag = { kind: 'caixa', a: [wx, wy], b: [wx, wy], aditivo: e.ctrlKey || e.metaKey, base };
      capturar();
      return;
    }
    // Fundo vazio: arrastar navega. Um clique sem arrastar limpa a seleção.
    drag = { kind: 'pan', sx, sy, cam: { ...logic.camera }, moved: false };
    capturar();
  }

  function mover(e: PointerEvent) {
    if (!drag) return;
    const [sx, sy] = localDe(e);
    const d = drag;
    if (d.kind === 'pan') {
      if (Math.hypot(sx - d.sx, sy - d.sy) > 3) d.moved = true;
      logic.camera = { x: d.cam.x + sx - d.sx, y: d.cam.y + sy - d.sy, zoom: d.cam.zoom };
    } else if (d.kind === 'mover') {
      if (!d.moved && Math.hypot(sx - d.sx, sy - d.sy) < 3) return;
      if (!d.moved) {
        d.moved = true;
        logic.beginGroup();
      }
      const dx = (sx - d.sx) / d.zoom;
      const dy = (sy - d.sy) / d.zoom;
      for (const n of d.nos) logic.setPosition(n.id, Math.round(n.x + dx), Math.round(n.y + dy));
      for (const c of d.caixas) {
        logic.setCommentBox(c.id, c.text, Math.round(c.x + dx), Math.round(c.y + dy), c.w, c.h);
      }
    } else if (d.kind === 'fio') {
      if (!d.moved && Math.hypot(sx - d.sx, sy - d.sy) < 3) return;
      if (!d.moved) targets = evaluateTargets(d);
      const [wx, wy] = toWorld(logic.camera, sx, sy);
      drag = { ...d, moved: true, wx, wy };
    } else if (d.kind === 'caixa') {
      const b = toWorld(logic.camera, sx, sy);
      drag = { ...d, b };
      const r: Rect = normRect(d.a[0], d.a[1], b[0], b[1]);
      const nos = nodes.filter((n) => rectsTouch(r, logic.rectOf(n))).map((n) => n.id);
      const caixas = logic.snap.comments
        .filter((c) => rectsTouch(r, [c.position[0], c.position[1], c.size[0], c.size[1]]))
        .map((c) => c.id);
      logic.select(
        [...(d.aditivo ? d.base.nos : []), ...nos],
        [...(d.aditivo ? d.base.caixas : []), ...caixas],
        false,
      );
    } else if (d.kind === 'redim') {
      if (!d.moved && Math.hypot(sx - d.sx, sy - d.sy) < 3) return;
      if (!d.moved) {
        d.moved = true;
        logic.beginGroup();
      }
      const w = Math.max(COMMENT_MIN[0], Math.round(d.w0 + (sx - d.sx) / d.zoom));
      const h = Math.max(COMMENT_MIN[1], Math.round(d.h0 + (sy - d.sy) / d.zoom));
      logic.setCommentBox(d.id, d.text, d.x, d.y, w, h);
    }
  }

  function solta(e: PointerEvent) {
    const d = drag;
    drag = null;
    targets = new Map();
    if (!d) return;
    if (d.kind === 'pan' && !d.moved) logic.clearSelection();
    if ((d.kind === 'mover' || d.kind === 'redim') && d.moved) logic.endGroup();
    if (d.kind === 'fio') soltaFio(e, d);
  }

  /** Fim de um fio arrastado: liga a um pino compatível, ou abre a busca filtrada pelo tipo no espaço vazio. */
  function soltaFio(e: PointerEvent, d: Extract<Drag, { kind: 'fio' }>) {
    // Um clique numa entrada ligada não mexe no fio.
    if (d.existente && !d.moved) return;
    const pinoAlvo = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>('[data-pin]');
    if (!pinoAlvo) {
      // On empty space, the wire that reached the input is removed and the search opens filtered by its type.
      if (d.existente) logic.disconnectInput(d.existente.node, d.existente.pin);
      const [sx, sy] = localDe(e);
      busca = { x: sx, y: sy, wx: d.wx, wy: d.wy, de: d.de, saida: d.saida, tipo: d.tipo };
      return;
    }
    const outro: PinRef = { node: Number(pinoAlvo.dataset.node), pin: pinoAlvo.dataset.pin ?? '' };
    const outroSaida = pinoAlvo.dataset.dir === 'out';
    // Dropped back on the pin the wire starts from: nothing to do.
    if (outro.node === d.de.node && outro.pin === d.de.pin && outroSaida === d.saida) return;
    if (outroSaida === d.saida) {
      log.toast('warn', 'Ligação recusada', refusalText(sameSideRefusal(d.saida)));
      return;
    }
    // The old wire into the input leaves only after the new link is accepted: if it is refused, the old wire stays.
    const [from, to] = d.saida ? [d.de, outro] : [outro, d.de];
    logic.connect(from, to, d.existente);
  }

  /**
   * Blurs the text field being typed in, if it is a comment or pin field. The blur fires its change event, which
   * confirms the typed text (VIS-014).
   */
  function leaveTextField() {
    const active = document.activeElement as HTMLElement | null;
    if (estaDigitando(active) && active?.closest('.caixa, [data-node]')) active.blur();
  }

  function roda(e: WheelEvent) {
    if (!logic.active) return;
    const [sx, sy] = localDe(e);
    const next = zoomAt(logic.camera, Math.exp(-e.deltaY * 0.0015), sx, sy);
    // Below the detail threshold the comment and pin fields are unmounted: their typed text is confirmed first.
    if (!showsDetail(next.zoom)) leaveTextField();
    logic.camera = next;
  }

  /** Botão direito: sobre um nó ou caixa, o menu de contexto; no vazio, a busca de nós (VIS-011). */
  function contexto(e: MouseEvent) {
    if (!logic.active) return;
    e.preventDefault();
    const alvo = e.target as HTMLElement;
    const [sx, sy] = localDe(e);
    const [wx, wy] = toWorld(logic.camera, sx, sy);
    const no = alvo.closest<HTMLElement>('[data-node]');
    const caixa = alvo.closest<HTMLElement>('[data-comment]');
    if (no && !alvo.closest('[data-pin]')) {
      const id = Number(no.dataset.node);
      if (!logic.selNodes.includes(id)) logic.select([id], [], false);
      const bp = pontosDeParada.has(id);
      showContextMenu(e, [
        { label: bp ? 'Remover ponto de parada' : 'Adicionar ponto de parada', keys: 'F9', run: () => logic.toggleBreakpoint() },
        'sep',
        { label: 'Copiar', keys: 'Ctrl+C', run: () => logic.copySelection() },
        { label: 'Duplicar', keys: 'Ctrl+D', run: () => logic.duplicateSelection() },
        { label: 'Excluir', keys: 'Delete', run: () => logic.removeSelection() },
        'sep',
        { label: 'Enquadrar seleção', keys: 'F', run: () => logic.frameSelection() },
      ]);
      return;
    }
    if (caixa) {
      const id = Number(caixa.dataset.comment);
      if (!logic.selComments.includes(id)) logic.select([], [id], false);
      showContextMenu(e, [
        { label: 'Excluir caixa', keys: 'Delete', run: () => logic.removeSelection() },
      ]);
      return;
    }
    busca = { x: sx, y: sy, wx, wy, de: null, saida: false, tipo: null };
  }

  /** Filtro da busca: o tipo do fio solto e a direção que o nó novo precisa ter. */
  const filtroDaBusca = $derived.by<PinFilter | null>(() => {
    if (!busca || !busca.tipo) return null;
    return { ty: busca.tipo, precisaEntrada: busca.saida };
  });

  /** Adiciona o nó escolhido na busca e, se havia um fio solto, liga o pino compatível dele. */
  function criaNo(kind: string) {
    const b = busca;
    busca = null;
    if (!b) return;
    const novo = logic.addNode(kind, Math.round(b.wx), Math.round(b.wy));
    if (novo === null || !b.de) return;
    const def = logic.defsByKind.get(kind);
    const procurados = b.saida ? (def?.inputs ?? []) : (def?.outputs ?? []);
    const pino = procurados.find((p) => p.ty === null || p.ty === b.tipo) ?? procurados[0];
    if (!pino) return;
    if (b.saida) logic.connect(b.de, { node: novo, pin: pino.name });
    else logic.connect({ node: novo, pin: pino.name }, b.de);
  }

  // ---------- Renderização auxiliares ----------

  function rotuloDoNo(n: GraphNode): string {
    return logic.defsByKind.get(n.kind)?.label ?? n.kind;
  }

  /** Text of an embedded value ('' when the pin has no text value). */
  function embeddedText(n: GraphNode, pin: string): string {
    const v = n.values?.[pin];
    return v && 'String' in v ? v.String : '';
  }

  /** Registry property chosen on a property node (by its embedded type and name), or undefined. */
  function propOfNode(n: GraphNode) {
    const nodeType = embeddedText(n, 'type');
    return nodeType ? propertyOfType(logic.nodeTypes, nodeType, embeddedText(n, 'name')) : undefined;
  }

  /** Type of an input pin's field. On property nodes, the value pin has the type the graph resolved. */
  function fieldType(n: GraphNode, pi: PinDef): PinType | null {
    return isPropertyNode(n.kind) ? (tipoDe.get(pinKey(n.id, 'i:' + pi.name)) ?? null) : pi.ty;
  }

  /** Value shown in the pin's field: the embedded one, or the default of the chosen property. */
  function fieldValue(n: GraphNode, pi: PinDef): Value | undefined {
    const embedded = n.values?.[pi.name];
    if (embedded) return embedded;
    if (isPropertyNode(n.kind) && pi.name === 'value') {
      const def = propOfNode(n);
      return (def && defaultValue(def)) ?? undefined;
    }
    return pi.default ?? undefined;
  }

  /** Options of the type picker: every type in the registry. */
  const typeOptions = $derived(logic.nodeTypes.map((t) => ({ value: t.name, label: t.name })));

  /** Suggestions under the name field: the properties of the type chosen on the node (inherited ones included). */
  function nameOptions(n: GraphNode) {
    return propertiesOfType(logic.nodeTypes, embeddedText(n, 'type')).map((p) => ({ value: p.name, label: p.label }));
  }

  function temPontoDeParada(id: number) {
    return pontosDeParada.has(id);
  }

  $effect(() => {
    const el = host;
    if (!el) return;
    const medir = () => {
      logic.viewW = el.clientWidth;
      logic.viewH = el.clientHeight;
    };
    medir();
    const ro = new ResizeObserver(medir);
    ro.observe(el);
    return () => ro.disconnect();
  });
</script>

<svelte:window onkeydown={teclaBaixo} onkeyup={teclaCima} onblur={() => (espaco = false)} />

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="canvas"
  class:focused={focus.panel === 'logic'}
  class:pan={espaco || drag?.kind === 'pan'}
  bind:this={host}
  use:focusPanel={'logic'}
  tabindex="-1"
  role="application"
  aria-label="Grafo do script {ativo ?? ''}"
  style="background-position: {cam.x}px {cam.y}px; background-size: {22 * cam.zoom}px {22 * cam.zoom}px"
  onpointerdown={baixo}
  onpointermove={mover}
  onpointerup={solta}
  onpointercancel={solta}
  onwheel={roda}
  oncontextmenu={contexto}
>
  <div class="mundo" style="transform: translate({cam.x}px, {cam.y}px) scale({cam.zoom})">
    <svg class="fios" width="1" height="1" aria-hidden="true">
      {#each caminhos as c (c.i)}
        <path class="toque" d={c.d} data-wire={c.i} />
      {/each}
      {#each caminhos as c (c.i)}
        <path
          class="fio"
          class:fluxo={c.fluxo}
          class:quente={c.quente}
          class:sel={c.sel}
          d={c.d}
          style="stroke: {c.cor}"
        />
      {/each}
      {#if arrastado}
        <path class="fio arrastando" class:fluxo={arrastado.fluxo} d={arrastado.d} style="stroke: {arrastado.cor}" />
      {/if}
    </svg>

    {#each logic.snap.comments as c (c.id)}
      <div
        class="caixa"
        class:sel={logic.selComments.includes(c.id)}
        style="left: {c.position[0]}px; top: {c.position[1]}px; width: {c.size[0]}px; height: {c.size[1]}px"
      >
        <div class="cabeca" data-comment={c.id}>Comentário</div>
        {#if detailed}
          <textarea
            class="corpo"
            value={c.text}
            aria-label="Texto do comentário"
            onpointerdown={(e) => e.stopPropagation()}
            onchange={(e) =>
              logic.setCommentBox(c.id, e.currentTarget.value, c.position[0], c.position[1], c.size[0], c.size[1])}
          ></textarea>
        {:else}
          <!-- Zoomed out, the text is only a preview: the box still moves and selects through its body (VIS-014). -->
          <div class="corpo summary" data-comment={c.id}>{c.text}</div>
        {/if}
        <span class="alca" data-resize={c.id} aria-hidden="true"></span>
      </div>
    {/each}

    {#each nodes as n (n.id)}
      {@const def = logic.defsByKind.get(n.kind)}
      {@const selecionado = logic.selNodes.includes(n.id)}
      {#if isReroute(n.kind)}
        {@const tyEntrada = tipoDe.get(pinKey(n.id, 'i:in')) ?? null}
        <div
          class="reroute"
          class:sel={selecionado}
          class:ativo={litNodes.has(n.id)}
          class:erro={erros.has(n.id)}
          class:bp={temPontoDeParada(n.id)}
          data-node={n.id}
          style="left: {n.position[0]}px; top: {n.position[1]}px; --cor: {pinStyle(tyEntrada).color}"
          title={erros.get(n.id) ?? 'Reroute'}
        >
          <span
            class="pino entrada"
            class:aceita={alvoDoArrasto(n.id, 'in', false) === 'aceita'}
            class:recusa={alvoDoArrasto(n.id, 'in', false) === 'recusa'}
            data-pin="in"
            data-dir="in"
            data-node={n.id}
            title={refusalOnPin(n.id, 'in', false)}
          ></span>
          <span
            class="pino saida"
            class:aceita={alvoDoArrasto(n.id, 'out', true) === 'aceita'}
            class:recusa={alvoDoArrasto(n.id, 'out', true) === 'recusa'}
            data-pin="out"
            data-dir="out"
            data-node={n.id}
            title={refusalOnPin(n.id, 'out', true) ?? dicaDoPino(n.id, true)}
          ></span>
        </div>
      {:else if def}
        {@const altura = nodeSize(n.kind, def)}
        <div
          class="no"
          class:sel={selecionado}
          class:ativo={litNodes.has(n.id)}
          class:erro={erros.has(n.id)}
          data-node={n.id}
          style="left: {n.position[0]}px; top: {n.position[1]}px; width: {altura[0]}px; --fam: {familyOf(def.category)}"
          title={erros.get(n.id)}
        >
          <header>
            {#if temPontoDeParada(n.id)}<span class="bp" title="Ponto de parada"></span>{/if}
            <span class="titulo">{rotuloDoNo(n)}</span>
            {#if erros.has(n.id)}<span class="aviso" title={erros.get(n.id)}>!</span>{/if}
          </header>
          {#each Array.from({ length: Math.max(def.inputs.length, def.outputs.length) }, (_, i) => i) as i (i)}
            {@const pi = def.inputs[i]}
            {@const po = def.outputs[i]}
            <div class="linha" style="height: {ROW_H}px">
              {#if pi}
                {@const fieldPinType = fieldType(n, pi)}
                <span
                  class="pino entrada"
                  class:aceita={alvoDoArrasto(n.id, pi.name, false) === 'aceita'}
                  class:recusa={alvoDoArrasto(n.id, pi.name, false) === 'recusa'}
                  data-pin={pi.name}
                  data-dir="in"
                  data-node={n.id}
                  title={refusalOnPin(n.id, pi.name, false) ?? pi.label}
                >
                  <PinShape ty={tipoDe.get(pinKey(n.id, 'i:' + pi.name)) ?? pi.ty} />
                </span>
                {#if detailed}
                  <span class="rotulo">{pi.label}</span>
                  {#if !ligadas.has(pinKey(n.id, 'i:' + pi.name))}
                    {#if isPropertyNode(n.kind) && pi.name === 'type'}
                      <PropSelect
                        selected={embeddedText(n, 'type')}
                        options={typeOptions}
                        emptyLabel="Sem tipo (decimal)"
                        onchange={(v) => logic.setValue(n.id, 'type', { String: v })}
                      />
                    {:else if isPropertyNode(n.kind) && pi.name === 'name'}
                      <PinValue
                        ty="String"
                        valor={fieldValue(n, pi)}
                        suggestions={nameOptions(n)}
                        onchange={(v) => logic.setValue(n.id, 'name', v)}
                      />
                    {:else if fieldPinType && fieldPinType !== 'Flow' && fieldPinType !== 'Node'}
                      <PinValue
                        ty={fieldPinType}
                        valor={fieldValue(n, pi)}
                        prop={isPropertyNode(n.kind) ? propOfNode(n) : undefined}
                        onchange={(v) => logic.setValue(n.id, pi.name, v)}
                      />
                    {/if}
                  {/if}
                {/if}
              {/if}
              {#if po}
                {#if detailed}<span class="rotulo direita">{po.label}</span>{/if}
                <span
                  class="pino saida"
                  class:aceita={alvoDoArrasto(n.id, po.name, true) === 'aceita'}
                  class:recusa={alvoDoArrasto(n.id, po.name, true) === 'recusa'}
                  data-pin={po.name}
                  data-dir="out"
                  data-node={n.id}
                  title={refusalOnPin(n.id, po.name, true) ?? dicaDoPino(n.id, true) ?? po.label}
                >
                  <PinShape ty={tipoDe.get(pinKey(n.id, 'o:' + po.name)) ?? po.ty} />
                </span>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    {/each}
  </div>

  {#if drag?.kind === 'caixa'}
    {@const r = normRect(drag.a[0], drag.a[1], drag.b[0], drag.b[1])}
    <div
      class="selecao"
      style="left: {r[0] * cam.zoom + cam.x}px; top: {r[1] * cam.zoom + cam.y}px; width: {r[2] * cam.zoom}px; height: {r[3] * cam.zoom}px"
    ></div>
  {/if}

  {#if busca}
    <NodeSearch
      x={busca.x}
      y={busca.y}
      filtro={filtroDaBusca}
      onpick={criaNo}
      onclose={() => (busca = null)}
    />
  {/if}

  {#if !nodes.length && !logic.snap.comments.length}
    <div class="vazio">
      <strong>Grafo vazio</strong>
      <span>Clique com o botão direito para buscar um nó, ou escolha um evento no painel ao lado.</span>
    </div>
  {/if}

  <div class="dica" aria-hidden="true">
    <kbd>Botão do meio</kbd> ou <kbd>Espaço</kbd> arrastar · roda zoom · <kbd>Shift</kbd> arrastar seleciona
  </div>

  {#if logic.minimapVisible}
    <Minimap />
  {/if}
</div>

<style>
  .canvas {
    position: absolute;
    inset: 0;
    overflow: hidden;
    background-image: radial-gradient(circle, var(--canvas-dot) 1px, transparent 1px);
    background-color: var(--bg-0);
    outline: none;
    user-select: none;
    touch-action: none;
  }
  .canvas.focused {
    box-shadow: inset 0 0 0 1px var(--accent-ring);
  }
  .canvas.pan {
    cursor: grab;
  }
  .mundo {
    position: absolute;
    left: 0;
    top: 0;
    transform-origin: 0 0;
  }
  .fios {
    position: absolute;
    left: 0;
    top: 0;
    width: 1px;
    height: 1px;
    overflow: visible;
    pointer-events: none;
  }
  .fio {
    fill: none;
    stroke-width: 2px;
    pointer-events: none;
    /* The lit state fades out over the same LIT_MS as the store (VIS-030). */
    transition:
      stroke-width 0.3s ease-out,
      filter 0.3s ease-out;
  }
  /* Fluxo de execução: tracejado animado, como na Fase 0 (VIS-004). */
  .fio.fluxo {
    stroke-dasharray: 6 8;
    animation: flow 1.2s linear infinite;
  }
  .fio.quente {
    stroke-width: 3px;
    filter: drop-shadow(0 0 3px currentColor);
  }
  /* A flow wire taken just now runs its dashes faster (VIS-030). The store drops the class LIT_MS later. */
  .fio.fluxo.quente {
    animation-duration: 0.4s;
  }
  .fio.sel {
    stroke-width: 3px;
  }
  .fio.arrastando {
    stroke-dasharray: 4 4;
    opacity: 0.8;
  }
  .toque {
    fill: none;
    stroke: transparent;
    stroke-width: 12px;
    pointer-events: stroke;
    cursor: pointer;
  }
  @keyframes flow {
    to {
      stroke-dashoffset: -28;
    }
  }

  .no {
    position: absolute;
    border-radius: var(--r-ctl);
    background: var(--bg-2);
    border: 1px solid var(--line);
    box-shadow: var(--shadow-card);
    font-size: var(--fs-12);
    overflow: hidden;
  }
  .no.sel {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .no.ativo {
    border-color: var(--type-exec);
  }
  .no.erro {
    border-color: var(--type-bool);
  }
  .no header {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 8px;
    font-weight: 500;
    color: var(--text);
    background: color-mix(in srgb, var(--fam) 30%, var(--bg-2));
    border-bottom: 1px solid var(--line);
    cursor: grab;
  }
  .titulo {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .aviso {
    flex: none;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: var(--fs-11);
    font-weight: 700;
    color: white;
    background: var(--type-bool);
  }
  .bp {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--type-bool);
    box-shadow: 0 0 0 2px var(--bg-2);
  }
  .linha {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 0 6px;
    white-space: nowrap;
  }
  .rotulo {
    color: var(--text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .rotulo.direita {
    margin-left: auto;
    text-align: right;
  }
  .pino {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    height: 14px;
    cursor: crosshair;
    border-radius: 50%;
  }
  .pino.saida {
    order: 3;
  }
  .pino.aceita {
    box-shadow: 0 0 0 2px var(--accent-ring);
  }
  .pino.recusa {
    opacity: 0.3;
  }
  .linha .pino.saida {
    margin-left: auto;
  }
  .linha .rotulo.direita {
    margin-left: auto;
  }
  .linha .rotulo.direita + .pino.saida {
    margin-left: 0;
  }

  .reroute {
    position: absolute;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--bg-2);
    border: 2px solid var(--cor);
  }
  .reroute.sel {
    box-shadow: 0 0 0 2px var(--accent);
  }
  .reroute.ativo {
    box-shadow: 0 0 0 2px var(--type-exec);
  }
  .reroute .pino {
    position: absolute;
    top: 0;
    width: 16px;
    height: 16px;
    border-radius: 50%;
  }
  .reroute .entrada {
    left: -10px;
  }
  .reroute .saida {
    left: 10px;
  }

  .caixa {
    position: absolute;
    border-radius: var(--r-ctl);
    background: color-mix(in srgb, var(--accent) 8%, transparent);
    border: 1px dashed var(--text-faint);
    display: flex;
    flex-direction: column;
  }
  .caixa.sel {
    border-color: var(--accent);
  }
  .cabeca {
    flex: none;
    height: 20px;
    padding: 0 8px;
    line-height: 20px;
    font-size: var(--fs-11);
    color: var(--text-dim);
    cursor: grab;
  }
  .corpo {
    flex: 1;
    min-height: 0;
    margin: 0 6px 6px;
    padding: 4px;
    resize: none;
    background: transparent;
    border: none;
    outline: none;
    font-size: var(--fs-12);
    color: var(--text);
    user-select: text;
  }
  /* Zoomed out (semantic zoom, VIS-014): the text is a read-only preview, cut at the box edge. */
  .corpo.summary {
    overflow: hidden;
    white-space: pre-wrap;
    user-select: none;
    color: var(--text-dim);
  }
  .alca {
    position: absolute;
    right: 0;
    bottom: 0;
    width: 12px;
    height: 12px;
    cursor: nwse-resize;
    background: linear-gradient(135deg, transparent 50%, var(--text-faint) 50%);
  }

  .selecao {
    position: absolute;
    pointer-events: none;
    border: 1px solid var(--accent);
    background: var(--accent-soft);
  }
  .vazio {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    color: var(--text-faint);
    text-align: center;
    pointer-events: none;
  }
  .vazio strong {
    color: var(--text-dim);
  }
  .vazio span {
    max-width: 320px;
    font-size: var(--fs-12);
    line-height: 1.5;
  }
  .dica {
    position: absolute;
    left: 8px;
    bottom: 6px;
    font-size: var(--fs-11);
    color: var(--text-faint);
    pointer-events: none;
  }
</style>
