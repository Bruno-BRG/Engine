// Assets do projeto aberto: pastas, arquivos com UID, importação e operações de arquivo (AST-001 a AST-005).
// O disco é a fonte da verdade. Este estado guarda o que o painel mostra e avisa quem precisa saber das mudanças.
import { isTauri } from '$lib/platform';
import { log } from './log.svelte';

export type AssetKind = 'texture' | 'audio' | 'font' | 'scene' | 'script' | 'folder' | 'other';

/** Uma entrada de pasta, ou um asset de `listAssets`. Os campos vêm do backend (camelCase). */
export interface DirEntry {
  name: string;
  rel: string;
  isDir: boolean;
  size: number;
  kind: AssetKind;
  uid: string | null;
}

/** Asset com UID, indexado por uid e por caminho. */
export interface AssetRef {
  uid: string;
  rel: string;
  name: string;
  kind: AssetKind;
}

export interface ImportResult {
  rel: string;
  uid: string;
  kind: AssetKind;
}

/** Mudanças que outros módulos (texturas, miniaturas) precisam acompanhar. */
export type AssetsEvent = { type: 'project' } | { type: 'changed'; paths: string[] };

/** Pasta virtual com as formas básicas. Não existe no disco. */
export const SHAPES_DIR = '@formas';

const FS_EVENT = 'project-fs-changed';

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

export function messageOf(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/** Caminho da pasta pai. '' é a raiz. */
export function parentOf(rel: string): string {
  const i = rel.lastIndexOf('/');
  return i < 0 ? '' : rel.slice(0, i);
}

/** Bytes vindos do backend. Respostas binárias chegam como ArrayBuffer. */
function toBuffer(v: unknown): ArrayBuffer {
  if (v instanceof ArrayBuffer) return v;
  if (ArrayBuffer.isView(v)) return v.buffer.slice(v.byteOffset, v.byteOffset + v.byteLength) as ArrayBuffer;
  throw new Error('resposta inesperada ao ler o arquivo');
}

class AssetsState {
  /** Pasta do projeto aberto, ou null. */
  root = $state<string | null>(null);
  /** Pasta aberta no painel, relativa à raiz. '' é a raiz e SHAPES_DIR são as formas. */
  dir = $state('');
  /** Sobe a cada atualização da lista de assets. Quem guarda dados por UID confere este número. */
  version = $state(0);
  /** Assets de todas as pastas (com UID), para texturas e busca. */
  all = $state<AssetRef[]>([]);
  textures = $derived(this.all.filter((a) => a.kind === 'texture'));

  #listing = $state<Record<string, DirEntry[]>>({});
  #byUid = new Map<string, AssetRef>();
  #byRel = new Map<string, AssetRef>();
  #listeners = new Set<(ev: AssetsEvent) => void>();
  /** Identifica o projeto: respostas de um projeto antigo são descartadas. */
  #epoch = 0;
  /** Só a leitura mais recente aplica o resultado, para não voltar a um estado velho. */
  #seq = 0;
  #listening: Promise<unknown> | null = null;

  /** Entradas da pasta aberta. Vazio enquanto a pasta não foi lida. */
  get entries(): DirEntry[] {
    return this.#listing[this.dir] ?? [];
  }

  /** Entradas de uma pasta já lida; undefined enquanto ela carrega. */
  listingOf(rel: string): DirEntry[] | undefined {
    return this.#listing[rel];
  }

  byUid(uid: string): AssetRef | undefined {
    void this.version;
    return this.#byUid.get(uid);
  }

  byRel(rel: string): AssetRef | undefined {
    void this.version;
    return this.#byRel.get(rel);
  }

  /** Recebe as mudanças de projeto e de arquivos. Devolve a função que cancela a inscrição. */
  subscribe(fn: (ev: AssetsEvent) => void): () => void {
    this.#listeners.add(fn);
    return () => {
      this.#listeners.delete(fn);
    };
  }

  #emit(ev: AssetsEvent) {
    for (const fn of this.#listeners) {
      try {
        fn(ev);
      } catch (e) {
        log.error(`Assets: um ouvinte falhou: ${messageOf(e)}`);
      }
    }
  }

  #setAll(refs: AssetRef[]) {
    this.all = refs;
    this.#byUid = new Map(refs.map((r) => [r.uid, r]));
    this.#byRel = new Map(refs.map((r) => [r.rel, r]));
    this.version++;
  }

  /** Abre um projeto: cria os sidecars que faltam, lê as pastas e começa a vigiar o disco. */
  async setProject(path: string): Promise<void> {
    const epoch = ++this.#epoch;
    this.root = path;
    this.dir = '';
    this.#listing = {};
    this.#setAll([]);
    this.#emit({ type: 'project' });
    if (!isTauri) return;
    try {
      await call('ensure_imports', { project: path });
    } catch (e) {
      if (epoch === this.#epoch) log.error(`Imports do projeto: ${messageOf(e)}`);
    }
    if (epoch !== this.#epoch) return;
    await this.refresh();
    if (epoch !== this.#epoch) return;
    await this.#watch(path);
  }

  /** Relê a lista de assets e as pastas já carregadas. */
  async refresh(): Promise<void> {
    const root = this.root;
    if (!root || !isTauri) return;
    const epoch = this.#epoch;
    const seq = ++this.#seq;
    // A raiz, as pastas já lidas e a pasta aberta (mesmo que ainda não tenha sido lida).
    const set = new Set<string>(['', ...Object.keys(this.#listing)]);
    if (this.dir && !this.dir.startsWith('@')) set.add(this.dir);
    const rels = [...set];
    const quiet = <T>(p: Promise<T>) => p.catch(() => undefined);
    const [all, lists] = await Promise.all([
      quiet(call<DirEntry[]>('list_assets', { project: root })),
      Promise.all(rels.map((rel) => quiet(call<DirEntry[]>('list_dir', { project: root, rel })))),
    ]);
    if (epoch !== this.#epoch || seq !== this.#seq) return;
    if (all) {
      this.#setAll(
        all.filter((e) => e.uid).map((e) => ({ uid: e.uid as string, rel: e.rel, name: e.name, kind: e.kind })),
      );
    }
    // Mescla, sem trocar o mapa inteiro: uma pasta aberta enquanto esta leitura rodava continua no mapa.
    const merged = { ...this.#listing };
    const failed = new Set<string>();
    rels.forEach((rel, i) => {
      const list = lists[i];
      if (list) {
        merged[rel] = list;
      } else {
        delete merged[rel];
        failed.add(rel);
      }
    });
    this.#listing = merged;
    // A pasta aberta sumiu (apagada ou movida): sobe para a pasta de cima.
    if (this.dir && failed.has(this.dir)) {
      this.dir = parentOf(this.dir);
      void this.load(this.dir);
    }
  }

  /** Lê uma pasta (usado ao abrir e ao expandir a árvore). */
  async load(rel: string): Promise<void> {
    const root = this.root;
    if (!root || !isTauri || rel.startsWith('@')) return;
    const epoch = this.#epoch;
    try {
      const list = await call<DirEntry[]>('list_dir', { project: root, rel });
      if (epoch === this.#epoch) this.#listing = { ...this.#listing, [rel]: list };
    } catch (e) {
      if (epoch !== this.#epoch) return;
      log.error(`Não foi possível abrir a pasta “${rel}”: ${messageOf(e)}`);
      // Sobe para a pasta de cima. Na raiz não há para onde subir, então não repete.
      if (this.dir === rel && rel !== '') this.open(parentOf(rel));
    }
  }

  open(rel: string): void {
    this.dir = rel;
    if (!rel.startsWith('@')) void this.load(rel);
  }

  up(): void {
    if (this.dir === '') return;
    this.open(this.dir.startsWith('@') ? '' : parentOf(this.dir));
  }

  /** Chamado pelo vigia (AST-004): relê a lista e avisa as texturas e miniaturas dos caminhos mudados. */
  async onFsChanged(paths: string[]): Promise<void> {
    // Arquivo copiado para o projeto com o app aberto ainda não tem UID: cria o .import antes de listar.
    if (this.root) {
      try {
        await call('ensure_imports', { project: this.root });
      } catch (e) {
        log.warn(`Imports do projeto: ${messageOf(e)}`);
      }
    }
    await this.refresh();
    this.#emit({ type: 'changed', paths });
  }

  async #watch(path: string): Promise<void> {
    if (!this.#listening) {
      this.#listening = import('@tauri-apps/api/event').then(({ listen }) =>
        listen<{ paths: string[] }>(FS_EVENT, (e) => void this.onFsChanged(e.payload.paths)),
      );
    }
    try {
      await this.#listening;
    } catch (e) {
      this.#listening = null;
      log.warn(`Não foi possível ouvir mudanças no disco: ${messageOf(e)}`);
      return;
    }
    try {
      await call('watch_project', { project: path });
    } catch (e) {
      log.warn(`Não foi possível vigiar a pasta do projeto: ${messageOf(e)}`);
    }
  }

  #ctx(): { project: string } {
    if (!this.root || !isTauri) throw new Error('Os assets ficam no disco: abra o projeto no app desktop');
    return { project: this.root };
  }

  /** Bytes de um arquivo do projeto (miniaturas e texturas). */
  async readBytes(rel: string): Promise<ArrayBuffer> {
    return toBuffer(await call<unknown>('read_file', { ...this.#ctx(), rel }));
  }

  async createFolder(rel: string): Promise<void> {
    await call('create_folder', { ...this.#ctx(), rel });
    await this.refresh();
  }

  /** Cria uma cena nova. Devolve o caminho relativo criado (com `.scene.ron`). */
  async createScene(rel: string, name: string): Promise<string> {
    const created = await call<string>('create_scene', { ...this.#ctx(), rel, name });
    await this.refresh();
    return created;
  }

  async rename(from: string, to: string): Promise<void> {
    await call('rename_entry', { ...this.#ctx(), from, to });
    // A pasta aberta (ou uma pasta de dentro dela) mudou de nome: segue o novo caminho, sem subir de nível.
    if (this.dir === from || this.dir.startsWith(from + '/')) this.dir = to + this.dir.slice(from.length);
    await this.refresh();
  }

  async trash(rel: string): Promise<void> {
    await call('trash_entry', { ...this.#ctx(), rel });
    await this.refresh();
  }

  /** Copia arquivos para a pasta `dir`. Não relê a lista: quem chama decide quando (uma vez, no fim). */
  async importFiles(sources: string[], dir: string): Promise<ImportResult[]> {
    return call<ImportResult[]>('import_files', { ...this.#ctx(), sources, dir });
  }
}

export const assets = new AssetsState();
