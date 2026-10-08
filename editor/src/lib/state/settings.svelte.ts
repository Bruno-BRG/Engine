// Configurações do projeto aberto (settings.ron). A interface lê e edita `projectSettings`.
// Cada alteração vai para o editor WASM na hora (o próximo Jogar já usa o valor novo) e é gravada em
// disco 500 ms depois da última mudança. Toda edição da interface passa por `editSettings`, que
// chama `markSettingsDirty`.
import { platform } from '$lib/platform';
import { emptySettings, type ProjectSettings } from '$lib/settings';
import { defaultBusEffects } from '$lib/audio/types';
import type { SceneEditor } from '$lib/runtime/engine';
import { log } from './log.svelte';
import { scene } from './scene.svelte';

/** Atraso da gravação em disco depois da última alteração. */
export const SAVE_DELAY_MS = 500;

/** Configurações do projeto aberto. Os campos mudam por dentro: o objeto exportado não é trocado. */
export const projectSettings: ProjectSettings = $state(emptySettings());

export type SettingsTab = 'geral' | 'entrada' | 'fisica' | 'audio' | 'camadas';

/** Captura pendente: a próxima tecla vai para `action`. `add` acrescenta; `only` troca todas. */
export interface KeyCapture {
  action: string;
  mode: 'add' | 'only';
}

/** Estado do diálogo Configurações do Projeto. */
export const settingsUi = $state({
  open: false,
  tab: 'geral' as SettingsTab,
  capture: null as KeyCapture | null,
});

/** Arquivo carregado: qual projeto e, se o settings.ron não pôde ser lido, o motivo (grava nada então). */
export const settingsStatus = $state({
  path: null as string | null,
  error: null as string | null,
});

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

let timer: ReturnType<typeof setTimeout> | null = null;
let pendingPath: string | null = null;
let blockedNotified = false;

/** Copia o estado do editor WASM para `projectSettings` (depois de uma operação feita no WASM). */
function pullFrom(ed: SceneEditor) {
  Object.assign(projectSettings, JSON.parse(ed.settings_json()));
  // O Rust omite os efeitos quando estão nos padrões: o editor sempre trabalha com a forma completa.
  for (const bus of projectSettings.buses) bus.effects ??= defaultBusEffects();
}

export function openProjectSettings(tab?: SettingsTab) {
  if (tab) settingsUi.tab = tab;
  settingsUi.open = true;
}

export function closeProjectSettings() {
  settingsUi.open = false;
  settingsUi.capture = null;
}

/** Abre as configurações de um projeto: lê settings.ron e envia ao editor WASM. Nunca lança. */
export async function loadProjectSettings(path: string): Promise<void> {
  await flushSettings();
  blockedNotified = false;
  settingsStatus.path = path;
  settingsStatus.error = null;
  try {
    const ed = await scene.ensure();
    try {
      ed.set_settings_json(await platform.loadSettings(path));
    } catch (e) {
      // Arquivo ilegível: o editor usa os padrões, e nada é gravado por cima do arquivo.
      ed.set_settings_json('');
      settingsStatus.error = message(e);
      log.toast('error', 'Configurações do projeto não carregadas', message(e));
    }
    pullFrom(ed);
  } catch (e) {
    settingsStatus.error = message(e);
    Object.assign(projectSettings, emptySettings());
  }
}

/** Fecha o projeto: grava o que estiver pendente e esquece o arquivo carregado. */
export async function unloadProjectSettings(): Promise<void> {
  await flushSettings();
  settingsStatus.path = null;
  settingsStatus.error = null;
  closeProjectSettings();
  Object.assign(projectSettings, emptySettings());
}

/** Envia as configurações atuais ao editor WASM e agenda a gravação em disco (500 ms). */
export function markSettingsDirty(): void {
  const path = settingsStatus.path;
  const ed = scene.editor;
  if (!path || !ed) return;
  try {
    ed.set_settings_json(JSON.stringify(projectSettings));
  } catch (e) {
    log.error('Configuração recusada: ' + message(e));
    return;
  }
  if (settingsStatus.error) {
    if (!blockedNotified) {
      blockedNotified = true;
      log.toast('warn', 'Configurações não gravadas', 'settings.ron está ilegível. Corrija o arquivo para voltar a gravar.');
    }
    return;
  }
  pendingPath = path;
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => void flushSettings(), SAVE_DELAY_MS);
}

/** Grava já a alteração pendente, se houver. Chamado pelo temporizador, ao trocar de projeto e ao fechar. */
export async function flushSettings(): Promise<void> {
  if (timer) clearTimeout(timer);
  timer = null;
  const path = pendingPath;
  pendingPath = null;
  if (!path || settingsStatus.error) return;
  try {
    await platform.saveSettings(path, JSON.stringify(projectSettings));
  } catch (e) {
    log.toast('error', 'Configurações não salvas', message(e));
  }
}

/** Altera as configurações com `fn` e agenda a gravação. Toda edição da interface passa aqui. */
export function editSettings(fn: (s: ProjectSettings) => void): void {
  fn(projectSettings);
  markSettingsDirty();
}

/** Erro para um nome de tag, layer ou material, ou null se o nome serve. `current` é o nome atual do item. */
export function nameError(names: string[], name: string, current: string | null = null): string | null {
  const n = name.trim();
  if (!n) return 'O nome não pode ficar vazio.';
  if (n !== current && names.includes(n)) return `"${n}" já existe.`;
  return null;
}

// ---------- Mapa de entrada (INP-001, INP-005) ----------

/** Nome de ação: letras, números e "_", sem espaços (ex.: ui_left, pular). */
const ACTION_NAME = /^[A-Za-z_][A-Za-z0-9_]*$/;

/** Cria uma ação sem teclas. Devolve o erro para mostrar, ou null se deu certo. */
export function addAction(name: string): string | null {
  const n = name.trim();
  if (!ACTION_NAME.test(n)) return 'Use letras, números e "_", sem espaços (ex.: pular).';
  if (Object.hasOwn(projectSettings.input.actions, n)) return `A ação "${n}" já existe.`;
  editSettings((s) => {
    s.input.actions[n] = [];
  });
  return null;
}

export function removeAction(name: string): void {
  if (settingsUi.capture?.action === name) settingsUi.capture = null;
  editSettings((s) => {
    delete s.input.actions[name];
  });
}

export function unbindKey(action: string, key: string): void {
  editSettings((s) => {
    s.input.actions[action] = (s.input.actions[action] ?? []).filter((k) => k !== key);
  });
}

/** Liga um código (tecla, botão do mouse ou do controle, ou eixo com sinal) à ação, sem repetir. */
export function addKey(action: string, code: string): void {
  editSettings((s) => {
    const keys = s.input.actions[action] ?? [];
    if (!keys.includes(code)) s.input.actions[action] = [...keys, code];
  });
}

/** Espera a próxima tecla para a ação. O diálogo recebe a tecla e chama `finishCapture`. */
export function startCapture(action: string, mode: KeyCapture['mode']): void {
  settingsUi.capture = { action, mode };
}

export function cancelCapture(): void {
  settingsUi.capture = null;
}

/**
 * Recebe a tecla capturada (KeyboardEvent.code). `add` acrescenta a tecla à ação; `only` troca as
 * teclas da ação pela nova, pelo `rebind` do WASM (INP-005), que vale já para o jogo em curso.
 */
export function finishCapture(code: string): void {
  const cap = settingsUi.capture;
  settingsUi.capture = null;
  if (!cap) return;
  const ed = scene.editor;
  if (cap.mode === 'only' && ed) {
    ed.rebind(cap.action, code);
    pullFrom(ed);
    markSettingsDirty();
    return;
  }
  editSettings((s) => {
    const keys = s.input.actions[cap.action] ?? [];
    if (cap.mode === 'only') s.input.actions[cap.action] = [code];
    else if (!keys.includes(code)) s.input.actions[cap.action] = [...keys, code];
  });
}

// ---------- Tags e camadas (SCN-033, PHY-020) ----------

/**
 * Troca o nome de uma tag ou layer nos nós da cena, para eles acompanharem o novo nome.
 * Nó sem a propriedade usa o padrão do registro ("untagged" e "default").
 */
export function renameInScene(prop: 'tag' | 'layer', from: string, to: string): void {
  const padrao = prop === 'tag' ? 'untagged' : 'default';
  for (const n of scene.data.nodes) {
    if ((n.props[prop] ?? padrao) === from) scene.setProp(n.uid, prop, to);
  }
}

/**
 * Renames the physics material at `index` across the project (PHY-023). The order matters:
 * 1. the settings get the new name, because the editor checks references against them;
 * 2. the open scene is renamed in memory as one undo step; if the editor refuses, the settings change is reverted;
 * 3. the settings are saved, so settings.ron holds the new name before the backend checks it;
 * 4. the other scene files on disk are rewritten by the backend.
 * Never throws: failures are shown as toasts.
 */
export async function renameMaterialInProject(index: number, to: string): Promise<void> {
  const from = projectSettings.physics_materials[index]?.name;
  if (from == null || from === to) return;
  if (settingsStatus.error) {
    log.toast('warn', 'Material não renomeado', 'settings.ron está ilegível. Corrija o arquivo para renomear.');
    return;
  }
  const setName = (name: string) =>
    editSettings((s) => {
      const m = s.physics_materials[index];
      if (m) m.name = name;
    });
  setName(to);
  if (scene.renameRefs('physics_material', from, to) === null) {
    // The open scene refused the new name: restore the old one so both sides still agree.
    setName(from);
    log.toast('error', 'Material não renomeado', 'A cena aberta recusou o nome novo.');
    return;
  }
  const path = settingsStatus.path;
  if (!path) return;
  await flushSettings();
  try {
    const report = await platform.renameMaterialInSceneFiles(path, from, to);
    if (report.changed.length) {
      log.info(`Material "${from}" trocado por "${to}" em ${report.changed.length} cena(s) do disco.`);
    }
    if (report.failed.length) log.toast('warn', 'Cenas não atualizadas', report.failed.join('; '));
    if (report.skipped.length) log.toast('warn', 'Cenas puladas', report.skipped.join('; '));
  } catch (e) {
    log.toast('error', 'Cenas não atualizadas', message(e));
  }
}
