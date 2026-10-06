import { app } from './state/app.svelte';
import { scene } from './state/scene.svelte';
import { viewApi } from './state/view';
import * as act from './actions';

export interface Command {
  id: string;
  title: string;
  group: string;
  keys?: string;
  /** Executa mesmo com foco num campo de texto. */
  global?: boolean;
  enabled?: () => boolean;
  run: () => void;
}

const hasProject = () => app.project != null;
const hasSelection = () => scene.selectedId != null;
const editing = () => hasProject() && !app.playing;

export const COMMANDS: Command[] = [
  { id: 'palette', title: 'Abrir paleta de comandos', group: 'Geral', keys: 'Ctrl+K', global: true, run: () => (app.paletteOpen = !app.paletteOpen) },
  { id: 'save', title: 'Salvar cena', group: 'Projeto', keys: 'Ctrl+S', global: true, enabled: hasProject, run: () => act.save() },
  { id: 'close', title: 'Fechar projeto e voltar ao início', group: 'Projeto', enabled: hasProject, run: () => act.closeProject() },
  { id: 'play', title: 'Jogar / parar', group: 'Jogo', keys: 'F5', global: true, enabled: hasProject, run: act.togglePlay },
  { id: 'play-alt', title: 'Jogar / parar', group: 'Jogo', keys: 'Ctrl+P', global: true, enabled: hasProject, run: act.togglePlay },
  { id: 'undo', title: 'Desfazer', group: 'Editar', keys: 'Ctrl+Z', enabled: () => editing() && scene.canUndo, run: act.undo },
  { id: 'redo', title: 'Refazer', group: 'Editar', keys: 'Ctrl+Shift+Z', enabled: () => editing() && scene.canRedo, run: act.redo },
  { id: 'redo-alt', title: 'Refazer', group: 'Editar', keys: 'Ctrl+Y', enabled: () => editing() && scene.canRedo, run: act.redo },
  { id: 'duplicate', title: 'Duplicar objeto selecionado', group: 'Editar', keys: 'Ctrl+D', enabled: () => editing() && hasSelection(), run: act.duplicateSelected },
  { id: 'delete', title: 'Excluir objeto selecionado', group: 'Editar', keys: 'Delete', enabled: () => editing() && hasSelection(), run: act.deleteSelected },
  { id: 'deselect', title: 'Limpar seleção', group: 'Editar', keys: 'Escape', enabled: hasProject, run: () => (app.playing ? act.togglePlay() : (scene.selectedId = null)) },
  { id: 'add-rect', title: 'Adicionar retângulo', group: 'Criar', enabled: editing, run: () => act.addShape(0) },
  { id: 'add-circle', title: 'Adicionar círculo', group: 'Criar', enabled: editing, run: () => act.addShape(1) },
  { id: 'add-diamond', title: 'Adicionar losango', group: 'Criar', enabled: editing, run: () => act.addShape(2) },
  { id: 'tool-select', title: 'Ferramenta: selecionar e mover', group: 'Viewport', keys: 'V', enabled: hasProject, run: () => (app.tool = 'select') },
  { id: 'tool-hand', title: 'Ferramenta: mão (navegar)', group: 'Viewport', keys: 'H', enabled: hasProject, run: () => (app.tool = 'hand') },
  { id: 'frame-sel', title: 'Enquadrar seleção', group: 'Viewport', keys: 'F', enabled: hasProject, run: () => viewApi.frameSelection() },
  { id: 'frame-screen', title: 'Enquadrar tela do jogo', group: 'Viewport', keys: 'Home', enabled: hasProject, run: () => viewApi.frameScreen() },
  { id: 'zoom-100', title: 'Zoom 100%', group: 'Viewport', keys: 'Ctrl+1', enabled: hasProject, run: () => viewApi.zoomTo(1) },
  { id: 'ws-cena', title: 'Workspace: Cena', group: 'Workspaces', keys: 'Alt+1', enabled: hasProject, run: () => act.setWorkspace('cena') },
  { id: 'ws-logica', title: 'Workspace: Lógica', group: 'Workspaces', keys: 'Alt+2', enabled: hasProject, run: () => act.setWorkspace('logica') },
  { id: 'ws-animacao', title: 'Workspace: Animação', group: 'Workspaces', keys: 'Alt+3', enabled: hasProject, run: () => act.setWorkspace('animacao') },
  { id: 'ws-sprite', title: 'Workspace: Sprite', group: 'Workspaces', keys: 'Alt+4', enabled: hasProject, run: () => act.setWorkspace('sprite') },
  { id: 'panel-left', title: 'Mostrar/ocultar hierarquia', group: 'Painéis', keys: 'Ctrl+B', enabled: hasProject, run: () => act.togglePanel('left') },
  { id: 'panel-right', title: 'Mostrar/ocultar Inspector', group: 'Painéis', keys: 'Ctrl+Alt+B', enabled: hasProject, run: () => act.togglePanel('right') },
  { id: 'panel-bottom', title: 'Mostrar/ocultar painel inferior', group: 'Painéis', keys: 'Ctrl+J', enabled: hasProject, run: () => act.togglePanel('bottom') },
  { id: 'tab-assets', title: 'Abrir Assets', group: 'Painéis', enabled: hasProject, run: () => openTab('assets') },
  { id: 'tab-console', title: 'Abrir Console', group: 'Painéis', enabled: hasProject, run: () => openTab('console') },
  { id: 'tab-debug', title: 'Abrir Depurador', group: 'Painéis', enabled: hasProject, run: () => openTab('depurador') },
  { id: 'tab-csharp', title: 'Ver código C# gerado', group: 'Painéis', enabled: hasProject, run: () => openTab('csharp') },
  { id: 'theme', title: 'Alternar tema claro/escuro', group: 'Geral', run: act.toggleTheme },
];

function openTab(t: typeof app.bottomTab) {
  app.bottomTab = t;
  app.panels.bottom = true;
  app.persistUi();
}

/** Comandos únicos para exibir na paleta (sem os atalhos alternativos). */
export const paletteCommands = () => COMMANDS.filter((c) => !c.id.endsWith('-alt') && c.id !== 'palette');

function matches(keys: string, e: KeyboardEvent) {
  const parts = keys.split('+');
  const key = parts.pop()!.toLowerCase();
  const ctrl = parts.includes('Ctrl');
  const shift = parts.includes('Shift');
  const alt = parts.includes('Alt');
  if (ctrl !== (e.ctrlKey || e.metaKey) || shift !== e.shiftKey || alt !== e.altKey) return false;
  const k = e.key.toLowerCase();
  if (key === 'delete') return k === 'delete' || k === 'backspace';
  if (/^\d$/.test(key)) return e.code === 'Digit' + key;
  return k === key;
}

export function handleShortcut(e: KeyboardEvent): boolean {
  const t = e.target as HTMLElement | null;
  const typing = !!t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable);
  for (const c of COMMANDS) {
    if (!c.keys || !matches(c.keys, e)) continue;
    if (typing && !c.global) return false;
    if (c.enabled && !c.enabled()) return false;
    e.preventDefault();
    c.run();
    return true;
  }
  return false;
}

/** Busca difusa simples, sem acentos. Retorna -1 quando não casa. */
export function fuzzyScore(query: string, text: string): number {
  const norm = (s: string) => s.normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLowerCase();
  const q = norm(query.trim());
  const s = norm(text);
  if (!q) return 0;
  const direct = s.indexOf(q);
  if (direct >= 0) return 1000 - direct;
  let score = 0;
  let pos = 0;
  let streak = 0;
  for (const ch of q) {
    if (ch === ' ') continue;
    const i = s.indexOf(ch, pos);
    if (i < 0) return -1;
    streak = i === pos ? streak + 1 : 0;
    score += 10 + streak * 5 - (i - pos);
    pos = i + 1;
  }
  return score;
}
