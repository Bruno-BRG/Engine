// Ponte com o backend Tauri. No navegador comum (sem Tauri) usa localStorage,
// o que permite desenvolver e testar a interface sem abrir o app desktop.
import type { SceneData } from './state/scene.svelte';

export interface ProjectInfo {
  name: string;
  path: string;
  template: string;
  engineVersion: string;
}

export interface RecentProject {
  name: string;
  path: string;
  template: string;
  openedAt: number;
}

export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

// ---------- Implementação de navegador ----------
const LS_PROJECTS = 'nexo.web.projects';
type WebProject = ProjectInfo & { scene: SceneData; openedAt: number };
const readWeb = (): Record<string, WebProject> => JSON.parse(localStorage.getItem(LS_PROJECTS) ?? '{}');
const writeWeb = (v: Record<string, WebProject>) => localStorage.setItem(LS_PROJECTS, JSON.stringify(v));
const now = () => Math.floor(Date.now() / 1000);

export const platform = {
  async defaultProjectsDir(): Promise<string> {
    return isTauri ? invoke('default_projects_dir') : 'Navegador / Nexo Projects';
  },

  async listRecent(): Promise<RecentProject[]> {
    if (isTauri) return invoke('list_recent_projects');
    return Object.values(readWeb())
      .sort((a, b) => b.openedAt - a.openedAt)
      .map(({ name, path, template, openedAt }) => ({ name, path, template, openedAt }));
  },

  async forgetRecent(path: string): Promise<void> {
    if (isTauri) return invoke('forget_recent_project', { path });
    const all = readWeb();
    delete all[path];
    writeWeb(all);
  },

  async createProject(name: string, parent: string, template: string, scene: SceneData): Promise<ProjectInfo> {
    if (isTauri) return invoke('create_project', { name, parent, template, scene });
    if (!name.trim()) throw new Error('Dê um nome ao projeto');
    const path = parent + ' / ' + name.trim();
    const all = readWeb();
    if (all[path]) throw new Error('Já existe um projeto com esse nome');
    const info: ProjectInfo = { name: name.trim(), path, template, engineVersion: '0.1.0' };
    all[path] = { ...info, scene, openedAt: now() };
    writeWeb(all);
    return info;
  },

  async openProject(path: string): Promise<ProjectInfo> {
    if (isTauri) return invoke('open_project', { path });
    const all = readWeb();
    const p = all[path];
    if (!p) throw new Error('Projeto não encontrado');
    p.openedAt = now();
    writeWeb(all);
    return { name: p.name, path: p.path, template: p.template, engineVersion: p.engineVersion };
  },

  async loadScene(path: string): Promise<SceneData> {
    if (isTauri) return invoke('load_scene', { path });
    return readWeb()[path].scene;
  },

  async saveScene(path: string, scene: SceneData): Promise<void> {
    if (isTauri) return invoke('save_scene', { path, scene });
    const all = readWeb();
    all[path].scene = scene;
    writeWeb(all);
  },

  async pickFolder(): Promise<string | null> {
    if (!isTauri) return null;
    const { open } = await import('@tauri-apps/plugin-dialog');
    const r = await open({ directory: true, multiple: false, title: 'Escolha uma pasta' });
    return typeof r === 'string' ? r : null;
  },

  async window() {
    if (!isTauri) return null;
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    return getCurrentWindow();
  },
};
