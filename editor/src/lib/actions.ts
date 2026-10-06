import { app, type Workspace } from './state/app.svelte';
import { scene, type ShapeKind } from './state/scene.svelte';
import { runtime } from './state/runtime.svelte';
import { log } from './state/log.svelte';
import { viewApi } from './state/view';
import { platform, type ProjectInfo } from './platform';
import { templateById } from './templates';

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

export async function openProject(info: ProjectInfo) {
  try {
    const data = await platform.loadScene(info.path);
    scene.load(data);
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

export function togglePlay() {
  if (!app.project) return;
  if (app.playing) {
    runtime.stop();
    app.playing = false;
    app.fps = 0;
    log.info('Jogo parado. A cena voltou ao estado de edição.');
  } else {
    app.workspace = 'cena';
    runtime.start(scene.snapshot().objects);
    app.playing = true;
    log.info('Jogo iniciado no viewport.');
  }
}

export function undo() {
  const label = scene.undo();
  if (label) log.info('Desfeito: ' + label);
}

export function redo() {
  const label = scene.redo();
  if (label) log.info('Refeito: ' + label);
}

export function addShape(shape: ShapeKind) {
  const c = viewApi.center();
  scene.add(shape, c.x, c.y);
}

export function deleteSelected() {
  if (scene.selectedId) scene.remove(scene.selectedId);
}

export function duplicateSelected() {
  if (scene.selectedId) scene.duplicate(scene.selectedId);
}

export function setWorkspace(w: Workspace) {
  app.workspace = w;
}

export function togglePanel(p: 'left' | 'right' | 'bottom') {
  app.panels[p] = !app.panels[p];
  app.persistUi();
}

export function toggleTheme() {
  app.theme = app.theme === 'dark' ? 'light' : 'dark';
  app.persistUi();
}
