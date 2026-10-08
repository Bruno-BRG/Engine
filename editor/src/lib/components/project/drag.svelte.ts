// Drag an item of the Project panel onto a folder or onto a resource field of the Inspector (INS-003).
// Uses the pointer, not the native HTML5 drag: on Windows, with the Tauri drag handler on, HTML5 drag
// does not reach the webview.
// Targets are the elements with `data-dir` (the value is the folder's relative path) and the fields
// registered with the `assetSlot` action (`data-asset-slot`).
import type { SlotKind } from '$lib/assetslot';

export const dragState = $state<{ from: string | null; over: string | null; overSlot: HTMLElement | null }>({
  from: null,
  over: null,
  overSlot: null,
});

/** Resource field of the Inspector that accepts a dropped asset. `drop` receives the asset UID (INS-003). */
export interface AssetSlot {
  kind: SlotKind;
  drop: (uid: string) => void;
}

/** Where the drop landed: a folder (relative path) or a resource field. */
export type DropTarget = { kind: 'dir'; rel: string } | { kind: 'slot'; el: HTMLElement; slot: AssetSlot };

/** Distância em pixels a partir da qual o clique vira arraste. */
const THRESHOLD = 4;

const slots = new WeakMap<Element, AssetSlot>();

/**
 * Action: marks `node` as a resource slot. A Project asset released on it calls `slot.drop` with the asset UID.
 * Only the reference is recorded. The file on disk is never moved, copied or renamed.
 */
export function assetSlot(node: HTMLElement, slot: AssetSlot) {
  slots.set(node, slot);
  node.setAttribute('data-asset-slot', slot.kind);
  return {
    update(next: AssetSlot) {
      slots.set(node, next);
      node.setAttribute('data-asset-slot', next.kind);
    },
    destroy() {
      slots.delete(node);
      node.removeAttribute('data-asset-slot');
    },
  };
}

/** The drop target under the pointer, or null. */
function targetAt(x: number, y: number): DropTarget | null {
  const hit = document.elementFromPoint(x, y);
  // Logic graph pins also carry data-dir but are not folders: a drop on one must never move a file.
  const dir = hit?.closest('[data-dir]:not([data-pin])');
  if (dir) return { kind: 'dir', rel: dir.getAttribute('data-dir') ?? '' };
  const el = hit?.closest<HTMLElement>('[data-asset-slot]');
  const slot = el ? slots.get(el) : undefined;
  return el && slot ? { kind: 'slot', el, slot } : null;
}

/**
 * Começa a acompanhar um ponteiro pressionado em `from`.
 * `accept(to)` diz se soltar em `to` é válido. `drop(to)` roda ao soltar num destino válido.
 * Escape cancela. Não faz nada se o arraste não passar do limiar.
 */
export function startDrag(
  e: PointerEvent,
  from: string,
  accept: (to: DropTarget) => boolean,
  drop: (to: DropTarget) => void,
): void {
  if (e.button !== 0) return;
  const x0 = e.clientX;
  const y0 = e.clientY;
  let active = false;
  let target: DropTarget | null = null;

  const finish = () => {
    window.removeEventListener('pointermove', move);
    window.removeEventListener('pointerup', up);
    window.removeEventListener('pointercancel', cancel);
    window.removeEventListener('keydown', key, true);
    dragState.from = null;
    dragState.over = null;
    dragState.overSlot = null;
  };
  const move = (ev: PointerEvent) => {
    if (!active) {
      if (Math.hypot(ev.clientX - x0, ev.clientY - y0) < THRESHOLD) return;
      active = true;
      dragState.from = from;
    }
    const hit = targetAt(ev.clientX, ev.clientY);
    target = hit && accept(hit) ? hit : null;
    dragState.over = target?.kind === 'dir' ? target.rel : null;
    dragState.overSlot = target?.kind === 'slot' ? target.el : null;
  };
  const up = () => {
    const wasActive = active;
    const dest = target;
    finish();
    if (!wasActive) return;
    // O navegador dispara um clique depois de soltar: ele não deve abrir nem selecionar nada.
    const stop = (ev: Event) => {
      ev.stopPropagation();
      ev.preventDefault();
    };
    window.addEventListener('click', stop, { capture: true, once: true });
    setTimeout(() => window.removeEventListener('click', stop, { capture: true }), 50);
    if (dest !== null) drop(dest);
  };
  const cancel = () => finish();
  const key = (ev: KeyboardEvent) => {
    if (ev.key !== 'Escape') return;
    ev.stopPropagation();
    finish();
  };

  window.addEventListener('pointermove', move);
  window.addEventListener('pointerup', up);
  window.addEventListener('pointercancel', cancel);
  window.addEventListener('keydown', key, true);
}
