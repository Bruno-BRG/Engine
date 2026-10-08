// Itens dos menus do painel Projeto: Criar (botão e clique direito no vazio) e o menu de um item.
import type { MenuItem } from '$lib/state/contextmenu.svelte';

const PHASE2 = (label: string): MenuItem => ({ label: `${label} (Fase 2)`, enabled: false, run: () => {} });

/** Menu Criar. `enabled` é falso fora de pastas graváveis (formas e modo navegador). */
export function createMenu(opts: { enabled: boolean; folder: () => void; scene: () => void }): MenuItem[] {
  return [
    { label: 'Pasta', enabled: opts.enabled, run: opts.folder },
    { label: 'Cena', enabled: opts.enabled, run: opts.scene },
    'sep',
    PHASE2('Script visual'),
    PHASE2('Material de física'),
    PHASE2('Ação de entrada'),
    PHASE2('Canal de áudio'),
  ];
}

/** Menu de um item (arquivo ou pasta). `open` só existe para scripts (abrir no Lógica). */
export function itemMenu(opts: {
  enabled: boolean;
  rename: () => void;
  trash: () => void;
  reveal: () => void;
  canReveal: boolean;
  open?: () => void;
}): MenuItem[] {
  const abrir: MenuItem[] = opts.open ? [{ label: 'Abrir na Lógica', enabled: true, run: opts.open }, 'sep'] : [];
  return [
    ...abrir,
    { label: 'Renomear', keys: 'F2', enabled: opts.enabled, run: opts.rename },
    'sep',
    { label: 'Mostrar no explorador', enabled: opts.canReveal, run: opts.reveal },
    'sep',
    { label: 'Excluir', keys: 'Del', enabled: opts.enabled, run: opts.trash },
  ];
}
