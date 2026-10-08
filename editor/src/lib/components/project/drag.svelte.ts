// Arrastar um item do painel Projeto para uma pasta. Usa o ponteiro, e não o arrastar nativo do HTML5:
// no Windows, com o tratador de arrastar do Tauri ligado, o arrastar HTML5 não chega ao webview.
// Os destinos são os elementos com `data-dir` (o valor é o caminho relativo da pasta).

export const dragState = $state<{ from: string | null; over: string | null }>({ from: null, over: null });

/** Distância em pixels a partir da qual o clique vira arraste. */
const THRESHOLD = 4;

/**
 * Começa a acompanhar um ponteiro pressionado em `from`.
 * `accept(to)` diz se soltar em `to` é válido. `drop(to)` roda ao soltar num destino válido.
 * Escape cancela. Não faz nada se o arraste não passar do limiar.
 */
export function startDrag(
  e: PointerEvent,
  from: string,
  accept: (to: string) => boolean,
  drop: (to: string) => void,
): void {
  if (e.button !== 0) return;
  const x0 = e.clientX;
  const y0 = e.clientY;
  let active = false;
  let target: string | null = null;

  const finish = () => {
    window.removeEventListener('pointermove', move);
    window.removeEventListener('pointerup', up);
    window.removeEventListener('pointercancel', cancel);
    window.removeEventListener('keydown', key, true);
    dragState.from = null;
    dragState.over = null;
  };
  const move = (ev: PointerEvent) => {
    if (!active) {
      if (Math.hypot(ev.clientX - x0, ev.clientY - y0) < THRESHOLD) return;
      active = true;
      dragState.from = from;
    }
    const hit = document.elementFromPoint(ev.clientX, ev.clientY)?.closest('[data-dir]');
    const to = hit ? hit.getAttribute('data-dir') : null;
    target = to !== null && accept(to) ? to : null;
    dragState.over = target;
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
