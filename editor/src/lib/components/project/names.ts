// Nomes de arquivos e pastas: validação, extensão, nome livre e formatação para o painel.
import type { AssetKind, DirEntry } from '$lib/state/assets.svelte';

/** Item em criação: uma pasta ou uma cena, com o nome digitado no campo. */
export interface NewItem {
  kind: 'folder' | 'scene';
  name: string;
}

/** Extensões que o import aceita (o backend confere de novo). */
export const IMPORT_EXTENSIONS = ['png', 'jpg', 'jpeg', 'webp', 'wav', 'ogg', 'ttf', 'otf'];

const SCENE_EXT = '.scene.ron';
/** Scripts de lógica (VIS-001): `scripts/<nome>.logic.ron`. */
const SCRIPT_EXT = '.logic.ron';
/** Nomes que já trazem extensão conhecida: não recebem a extensão antiga de novo. */
const KNOWN_EXT = /\.(png|jpe?g|webp|gif|bmp|wav|ogg|mp3|flac|ttf|otf|woff2?|ron)$/i;

/** Extensão de um nome (`.scene.ron` para cenas, `.logic.ron` para scripts). Vazio se não tiver. */
export function extOf(name: string): string {
  const baixo = name.toLowerCase();
  if (baixo.endsWith(SCENE_EXT)) return SCENE_EXT;
  if (baixo.endsWith(SCRIPT_EXT)) return SCRIPT_EXT;
  const i = name.lastIndexOf('.');
  return i > 0 ? name.slice(i) : '';
}

/** Nome do script (como o Lógica o chama) a partir do nome do arquivo: sem pasta e sem `.logic.ron`. */
export function scriptNameOf(file: string): string {
  return file.replace(/\.logic\.ron$/i, '');
}

/** Nome para editar no campo: sem a extensão. Pastas ficam como estão. */
export function editableStem(entry: DirEntry): string {
  if (entry.isDir) return entry.name;
  const ext = extOf(entry.name);
  return entry.name.slice(0, entry.name.length - ext.length);
}

/** Nome digitado para o item. Se o usuário não digitou uma extensão conhecida, mantém a original. */
export function withExtension(typed: string, entry: DirEntry): string {
  if (entry.isDir) return typed;
  // Script: o nome sempre termina em `.logic.ron`, mesmo que o usuário digite outra extensão.
  if (entry.kind === 'script') return scriptNameOf(typed) + SCRIPT_EXT;
  const ext = extOf(entry.name);
  if (!ext || KNOWN_EXT.test(typed)) return typed;
  return typed + ext;
}

/** Mensagem de erro para um nome inválido, ou null. */
export function invalidName(name: string): string | null {
  const n = name.trim();
  if (!n) return 'Dê um nome ao item';
  if (/[\\/:*?"<>|]/.test(n)) return 'O nome não pode ter \\ / : * ? " < > |';
  if (n.startsWith('.')) return 'O nome não pode começar com ponto';
  return null;
}

/** `base`, `base 2`, `base 3`... sem repetir nenhum nome já usado (sem diferenciar maiúsculas). */
export function uniqueName(base: string, taken: Set<string>): string {
  const used = new Set([...taken].map((t) => t.toLowerCase()));
  if (!used.has(base.toLowerCase())) return base;
  let n = 2;
  while (used.has(`${base} ${n}`.toLowerCase())) n++;
  return `${base} ${n}`;
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

const KIND_LABEL: Record<AssetKind, string> = {
  folder: 'Pasta',
  texture: 'Textura',
  audio: 'Áudio',
  font: 'Fonte',
  scene: 'Cena',
  script: 'Script',
  other: 'Arquivo',
};

export function kindLabel(kind: AssetKind): string {
  return KIND_LABEL[kind] ?? 'Arquivo';
}

/** Compara nomes sem acento e sem diferenciar maiúsculas, para a busca. */
export function normalize(s: string): string {
  return s.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase();
}

export function baseName(rel: string): string {
  return rel.slice(rel.lastIndexOf('/') + 1);
}
