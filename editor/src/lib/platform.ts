// Ponte com o backend Tauri. No navegador comum (sem Tauri) usa localStorage,
// o que permite desenvolver e testar a interface sem abrir o app desktop.

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

/** Resultado de trocar um nome nas cenas do disco: caminhos alterados e falhas (cena ilegível, por exemplo). */
export interface SceneRenameReport {
  changed: string[];
  failed: string[];
}

export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

// ---------- Implementação de navegador ----------
const LS_PROJECTS = 'nexo.web.projects';
type WebProject = ProjectInfo & { scene: unknown; openedAt: number; settings?: string };
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

  /** `scene` é o formato 1 do template; o Rust converte para a árvore de nós ao criar. */
  async createProject(name: string, parent: string, template: string, scene: unknown): Promise<ProjectInfo> {
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

  /** Texto da cena (RON no formato atual, ou o formato 1 ainda não migrado). */
  async loadScene(path: string): Promise<string> {
    if (isTauri) return invoke('load_scene', { path });
    const saved = readWeb()[path].scene;
    return typeof saved === 'string' ? saved : JSON.stringify(saved);
  },

  async saveScene(path: string, text: string): Promise<void> {
    if (isTauri) return invoke('save_scene', { path, text });
    const all = readWeb();
    all[path].scene = text;
    writeWeb(all);
  },

  /**
   * Troca um nome de referência (PHY-023: o material de física) nas cenas do projeto gravadas no disco.
   * `skip` é a cena aberta, que o editor atualiza e grava sozinho. A versão web tem uma cena por projeto.
   */
  async renameInSceneFiles(path: string, skip: string, key: string, from: string, to: string): Promise<SceneRenameReport> {
    if (isTauri) return invoke('rename_prop_in_scenes', { project: path, skip, key, from, to });
    return { changed: [], failed: [] };
  },

  /** Configurações do projeto em JSON (settings.ron no disco). Sem arquivo, texto vazio (padrões). */
  async loadSettings(path: string): Promise<string> {
    if (isTauri) return invoke('load_settings', { path });
    const saved = readWeb()[path]?.settings;
    return typeof saved === 'string' ? saved : '';
  },

  /** Grava as configurações (JSON) como settings.ron. O Rust valida antes de gravar. */
  async saveSettings(path: string, json: string): Promise<void> {
    if (isTauri) return invoke('save_settings', { path, text: json });
    const all = readWeb();
    if (!all[path]) throw new Error('Projeto não encontrado');
    all[path].settings = json;
    writeWeb(all);
  },

  /** Grava texto num arquivo relativo ao projeto. Sobrescreve: quem chama confere o nome antes. */
  async writeProjectText(project: string, rel: string, text: string): Promise<void> {
    if (isTauri) return invoke('write_text', { project, rel, text });
    throw new Error('Disponível no app desktop');
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
