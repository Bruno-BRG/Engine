import { app, type Workspace } from './state/app.svelte';
import { scene } from './state/scene.svelte';
import { runtime } from './state/runtime.svelte';
import { audio } from './state/audio.svelte';
import { assets } from './state/assets.svelte';
import { textures } from './state/textures.svelte';
import { log } from './state/log.svelte';
import { viewApi } from './state/view';
import { platform, type ProjectInfo } from './platform';
import { templateById } from './templates';

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

export async function openProject(info: ProjectInfo) {
  try {
    const text = await platform.loadScene(info.path);
    await scene.load(text);
    textures.reset();
    await assets.setProject(info.path);
    app.project = info;
    app.workspace = 'cena';
    app.dirty = false;
    app.lastSaved = new Date();
    log.success('Projeto aberto: ' + info.name);
  } catch (e) {
    log.toast('error', 'Não foi possível abrir o projeto', message(e));
  }
}

export async function openProjectAt(path: string) {
  try {
    await openProject(await platform.openProject(path));
  } catch (e) {
    log.toast('error', 'Não foi possível abrir o projeto', message(e));
  }
}

export async function createProject(name: string, parent: string, templateId: string) {
  const info = await platform.createProject(name, parent, templateId, templateById(templateId).build());
  await openProject(info);
  log.toast('success', 'Projeto criado', info.name);
}

export async function pickAndOpenProject() {
  const dir = await platform.pickFolder();
  if (dir) await openProjectAt(dir);
}

export async function closeProject() {
  if (app.playing) togglePlay();
  if (app.dirty) await save(true);
  app.project = null;
  textures.reset();
}

export async function save(silent = false) {
  if (!app.project || app.saving) return;
  app.saving = true;
  try {
    await platform.saveScene(app.project.path, scene.snapshot());
    app.dirty = false;
    app.lastSaved = new Date();
    if (!silent) log.toast('success', 'Cena salva');
  } catch (e) {
    log.toast('error', 'Falha ao salvar', message(e));
  } finally {
    app.saving = false;
  }
}

/** Duração de um quadro no Passo a passo (60 quadros por segundo). */
export const STEP_SECONDS = 1 / 60;

export function togglePlay() {
  if (!app.project) return;
  if (app.playing) {
    runtime.stop();
    app.playing = false;
    app.paused = false;
    app.stepRequested = false;
    app.fps = 0;
    log.info('Jogo parado. A cena voltou ao estado de edição.');
  } else {
    app.workspace = 'cena';
    runtime.start();
    log.onPlayStart();
    app.playing = true;
    log.info('Jogo iniciado no viewport.');
  }
}

export function togglePause() {
  if (!app.playing) return;
  app.paused = !app.paused;
  // Pausar suspende o contexto de áudio: os sons param no ponto em que estão.
  audio.setPaused(app.paused);
  log.info(app.paused ? 'Jogo pausado.' : 'Jogo retomado.');
}

/** Avança um quadro. Fora do modo de jogo, inicia o jogo já pausado, como o Unity. */
export function stepFrame() {
  if (!app.project) return;
  if (!app.playing) togglePlay();
  app.paused = true;
  app.stepRequested = true;
}

export function copySelected() {
  if (scene.selectedId) scene.copy(scene.selectedId);
}

export function cutSelected() {
  if (scene.selectedId) scene.cut(scene.selectedId);
}

/** Cola ao lado da seleção (mesmo pai), ou na raiz quando nada está selecionado. */
export function pasteClipboard() {
  scene.paste(scene.selected?.parent ?? null);
}

export function undo() {
  const label = scene.undo();
  if (label) log.info('Desfeito: ' + label);
}

export function redo() {
  const label = scene.redo();
  if (label) log.info('Refeito: ' + label);
}

/** Cria um nó 2D na raiz, no centro da vista (como o Unity faz com GameObjects). */
export function addNode(kind: string, shape?: 'rect' | 'circle' | 'diamond') {
  const c = viewApi.center();
  const uid = scene.add(kind, null);
  if (!uid) return;
  scene.setProp(uid, 'position', [Math.round(c.x), Math.round(c.y)]);
  if (shape) scene.setProp(uid, 'shape', shape);
}

const SHAPE_IDS = ['rect', 'circle', 'diamond'] as const;

/** Atalho para criar um sprite de forma de teste (0 retângulo, 1 círculo, 2 losango). */
export function addShape(shape: 0 | 1 | 2) {
  addNode('Sprite2D', SHAPE_IDS[shape]);
}

export function deleteSelected() {
  scene.removeSelected();
}

export function duplicateSelected() {
  scene.duplicateSelected();
}

export function setWorkspace(w: Workspace) {
  app.workspace = w;
}

export function togglePanel(p: 'left' | 'right' | 'bottom') {
  app.panels[p] = !app.panels[p];
  app.persistUi();
}

/** Predefinições de layout (EDT-011, EDT-015). Salvar layouts próprios fica para a P1. */
export const LAYOUTS = {
  padrao: { label: 'Padrão', panels: { left: true, right: true, bottom: true }, bottom: 230 },
  cena: { label: 'Cena', panels: { left: true, right: true, bottom: false }, bottom: 230 },
  programacao: { label: 'Programação', panels: { left: true, right: true, bottom: true }, bottom: 360 },
  viewport: { label: 'Só viewport', panels: { left: false, right: false, bottom: false }, bottom: 230 },
} as const;

export type LayoutId = keyof typeof LAYOUTS;

export function applyLayout(id: LayoutId) {
  const l = LAYOUTS[id];
  app.panels = { ...l.panels };
  app.sizes = { ...app.sizes, bottom: l.bottom };
  app.layout = id;
  app.persistUi();
}

export function toggleTheme() {
  app.theme = app.theme === 'dark' ? 'light' : 'dark';
  app.persistUi();
}
