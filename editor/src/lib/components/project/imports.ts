// Importação de arquivos para o projeto (AST-003): a janela de escolha e os arquivos soltos sobre o painel.
import { assets, messageOf } from '$lib/state/assets.svelte';
import { isTauri } from '$lib/platform';
import { log } from '$lib/state/log.svelte';
import { tasks } from '$lib/state/tasks.svelte';
import { IMPORT_EXTENSIONS, baseName } from './names';

/** Extensão em minúsculas, sem o ponto. Vazio se o caminho não tem uma. */
function extOf(path: string): string {
  const name = baseName(path);
  const i = name.lastIndexOf('.');
  return i > 0 ? name.slice(i + 1).toLowerCase() : '';
}

/**
 * Copia os arquivos aceitos para a pasta `dir`, um por vez, com uma tarefa na barra de status.
 * Um arquivo que falha não para os outros. Quem chama confere se a pasta é gravável.
 */
export async function importFilesTo(paths: string[], dir: string): Promise<void> {
  const supported = paths.filter((p) => IMPORT_EXTENSIONS.includes(extOf(p)));
  const skipped = paths.length - supported.length;
  if (skipped > 0) {
    log.toast('warn', `${skipped} arquivo(s) ignorado(s)`, 'Formatos aceitos: PNG, JPG, WEBP, WAV, OGG, TTF e OTF');
  }
  if (supported.length === 0) return;

  const task = tasks.start(`Importando ${supported.length} arquivo(s)…`);
  const failures: string[] = [];
  let done = 0;
  for (const path of supported) {
    try {
      await assets.importFiles([path], dir);
    } catch (e) {
      failures.push(`${baseName(path)}: ${messageOf(e)}`);
    }
    done++;
    task.progress(done / supported.length);
  }
  await assets.refresh();
  if (failures.length > 0) {
    task.fail(`${failures.length} de ${supported.length} não foram importados`);
    log.error(`Importação: ${failures.join('; ')}`);
  } else {
    task.done(`${supported.length} arquivo(s) importado(s)`);
  }
}

/** Janela do sistema para escolher arquivos. Devolve os caminhos escolhidos (vazio se cancelar). */
export async function pickImportFiles(): Promise<string[]> {
  const { open } = await import('@tauri-apps/plugin-dialog');
  const picked = await open({
    multiple: true,
    directory: false,
    title: 'Importar arquivos',
    filters: [{ name: 'Imagens, sons e fontes', extensions: IMPORT_EXTENSIONS }],
  });
  return typeof picked === 'string' ? [picked] : Array.isArray(picked) ? picked : [];
}

