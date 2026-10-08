// Painel com o foco do teclado e do mouse (EDT-020).
// O painel focado usa a cor de seleção cheia; os demais usam a seleção sem foco.

export type PanelId = 'hierarchy' | 'inspector' | 'project' | 'console' | 'viewport' | 'game' | 'logic';

class FocusState {
  panel = $state<PanelId>('viewport');
}

export const focus = new FocusState();

/**
 * Action Svelte: marca o painel como focado quando o usuário entra nele.
 * Uso: <section use:focusPanel={'hierarchy'}>
 */
export function focusPanel(node: HTMLElement, id: PanelId) {
  const take = () => {
    focus.panel = id;
  };
  node.addEventListener('pointerdown', take, true);
  node.addEventListener('focusin', take, true);
  return {
    update(next: PanelId) {
      id = next;
    },
    destroy() {
      node.removeEventListener('pointerdown', take, true);
      node.removeEventListener('focusin', take, true);
    },
  };
}
