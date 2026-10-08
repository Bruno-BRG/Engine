// Menu de contexto (EDT-021). Um menu aberto por vez, desenhado por ContextMenu.svelte.

export type MenuItem =
  | { label: string; keys?: string; enabled?: boolean; checked?: boolean; run: () => void }
  | 'sep';

export interface OpenContextMenu {
  x: number;
  y: number;
  items: MenuItem[];
}

class ContextMenuState {
  open = $state<OpenContextMenu | null>(null);
}

export const contextMenu = new ContextMenuState();

/** Abre o menu na posição do ponteiro e bloqueia o menu nativo do sistema. */
export function showContextMenu(event: MouseEvent, items: MenuItem[]) {
  event.preventDefault();
  contextMenu.open = { x: event.clientX, y: event.clientY, items };
}

export function closeContextMenu() {
  contextMenu.open = null;
}
