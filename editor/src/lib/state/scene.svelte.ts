// Cena em edição. A árvore de nós vive no Rust (crate nexo-core, via WebAssembly, SCN-005).
// Este módulo guarda uma cópia reativa do contorno para a interface e envia as operações.
// Toda edição passa por aqui, para que a interface e a cena nunca divirjam.
import { createSceneEditor, randomSeed, readNodeRegistry, type SceneEditor } from '$lib/runtime/engine';
import { log } from './log.svelte';

export type PropKind = 'float' | 'int' | 'bool' | 'str' | 'vec2' | 'color' | 'enum' | 'uid' | 'mask';

export interface PropDef {
  name: string;
  label: string;
  group: string;
  kind: PropKind;
  default: unknown;
  min: number | null;
  max: number | null;
  step: number | null;
  unit: string | null;
  options: string[];
  asset: string | null;
}

export interface NodeType {
  name: string;
  base: string | null;
  category: string;
  description: string;
  props: PropDef[];
}

/** Forma de colisão no mundo: `points` são os cantos (retângulo) ou pontos do círculo. */
export interface ShapeOutline {
  uid: string;
  owner: string;
  points: [number, number][];
}

export interface NodeView {
  uid: string;
  name: string;
  kind: string;
  parent: string | null;
  children: string[];
  depth: number;
  visible: boolean;
  world: { x: number; y: number; rot: number; sx: number; sy: number };
  props: Record<string, any>;
}

export interface Outline {
  name: string;
  screen: [number, number];
  playing: boolean;
  canUndo: boolean;
  canRedo: boolean;
  undoLabel: string | null;
  redoLabel: string | null;
  nodes: NodeView[];
}

const EMPTY: Outline = {
  name: 'Principal',
  screen: [1280, 720],
  playing: false,
  canUndo: false,
  canRedo: false,
  undoLabel: null,
  redoLabel: null,
  nodes: [],
};

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

class SceneState {
  #ed: SceneEditor | null = null;
  #ready: Promise<SceneEditor> | null = null;

  /** Contorno da cena. Substituído inteiro a cada operação. */
  data = $state.raw<Outline>(EMPTY);
  /** Tipos de nó registrados no Rust, na ordem do menu de criação. */
  types = $state.raw<NodeType[]>([]);
  /** Nós selecionados, na ordem de seleção. O último é o principal. */
  selection = $state<string[]>([]);
  hoveredId = $state<string | null>(null);
  /** Incrementa a cada mudança; o salvamento automático observa este valor. */
  revision = $state(0);

  byUid = $derived(new Map(this.data.nodes.map((n) => [n.uid, n] as const)));
  selected = $derived(this.selectedId ? (this.byUid.get(this.selectedId) ?? null) : null);

  get selectedId(): string | null {
    return this.selection.at(-1) ?? null;
  }
  set selectedId(uid: string | null) {
    this.select(uid);
  }
  get canUndo() {
    return this.data.canUndo;
  }
  get canRedo() {
    return this.data.canRedo;
  }
  /** Nome do último passo feito, para a barra de status. */
  get lastAction() {
    return this.data.undoLabel ?? '';
  }
  /** Há algo na área de transferência? Reativo: a interface reage ao copiar. */
  hasClipboard = $state(false);
  /** Acesso direto ao editor WASM (uso restrito a viewport e reprodução). */
  get editor(): SceneEditor | null {
    return this.#ed;
  }
  get rootIds() {
    return this.data.nodes.filter((n) => n.parent == null).map((n) => n.uid);
  }

  /** Garante que o editor WASM existe e que os tipos foram lidos. */
  ensure(): Promise<SceneEditor> {
    this.#ready ??= createSceneEditor().then((ed) => {
      this.#ed = ed;
      this.types = JSON.parse(readNodeRegistry()) as NodeType[];
      return ed;
    });
    return this.#ready;
  }

  /** Carrega o texto de uma cena (formato atual ou formato 1 migrado). */
  async load(text: string) {
    const ed = await this.ensure();
    ed.load_text(text, randomSeed());
    this.#reset();
  }

  /** Cena nova com raiz e câmera. */
  async newDefault(name: string) {
    const ed = await this.ensure();
    ed.new_default(name, randomSeed());
    this.#reset();
  }

  /** Texto RON de uma cena feita só com a subárvore de `uid` (SCN-022). */
  branchText(uid: string): string {
    if (!this.#ed) throw new Error('A cena ainda não foi carregada');
    return this.#ed.branch_to_ron(uid, randomSeed());
  }

  /** Texto RON da cena atual, para salvar. */
  snapshot(): string {
    if (!this.#ed) throw new Error('A cena ainda não foi carregada');
    return this.#ed.to_ron();
  }

  #reset() {
    this.selection = [];
    this.hoveredId = null;
    this.#refresh();
  }

  #refresh() {
    if (!this.#ed) return;
    this.data = JSON.parse(this.#ed.outline() as unknown as string) as Outline;
    this.revision++;
  }

  /** Executa uma operação do editor, atualiza o contorno e registra erros no console. */
  #run<T>(fn: (ed: SceneEditor) => T): T | undefined {
    if (!this.#ed) return undefined;
    try {
      const r = fn(this.#ed);
      this.#refresh();
      return r;
    } catch (e) {
      log.error('Operação recusada: ' + message(e));
      return undefined;
    }
  }

  // ---------- Seleção e consultas ----------

  /** Seleciona um nó (substitui a seleção). `null` limpa. Com `additive`, alterna o nó na seleção. */
  select(uid: string | null, additive = false) {
    if (uid == null) {
      this.selection = [];
      return;
    }
    if (!this.byUid.has(uid)) return;
    if (!additive) {
      this.selection = [uid];
      return;
    }
    this.selection = this.selection.includes(uid)
      ? this.selection.filter((u) => u !== uid)
      : [...this.selection, uid];
  }

  isSelected(uid: string) {
    return this.selection.includes(uid);
  }

  node(uid: string): NodeView | undefined {
    return this.byUid.get(uid);
  }

  /** Filhos diretos de um nó, na ordem da árvore. */
  children(uid: string | null): NodeView[] {
    const ids = uid == null ? this.rootIds : (this.byUid.get(uid)?.children ?? []);
    return ids.map((id) => this.byUid.get(id)).filter((n): n is NodeView => !!n);
  }

  isLocked(uid: string) {
    return !!this.byUid.get(uid)?.props.locked;
  }

  pick(x: number, y: number): string | null {
    return this.#ed?.pick(x, y) ?? null;
  }

  /** Limites [minx, miny, maxx, maxy] do nó no mundo, ou null. */
  bounds(uid: string): [number, number, number, number] | null {
    const b = this.#ed?.bounds(uid);
    return b && b.length === 4 ? (b as unknown as [number, number, number, number]) : null;
  }

  /** Contornos das formas de colisão (VPT-005): pontos no mundo, em volta de cada forma ativa. */
  shapeOutlines(): ShapeOutline[] {
    if (!this.#ed) return [];
    return JSON.parse(this.#ed.shape_outlines() as unknown as string) as ShapeOutline[];
  }

  /** Câmera atual [x, y, zoom] ou null. */
  camera(): [number, number, number] | null {
    const c = this.#ed?.camera();
    return c && c.length === 3 ? [c[0], c[1], c[2]] : null;
  }

  // ---------- Operações ----------

  /** Cria um nó do tipo `kind` dentro de `parent` (ou na raiz) e o seleciona. */
  add(kind: string, parent: string | null = null, index?: number): string | null {
    const uid = this.#run((ed) => ed.add_node(parent ?? undefined, kind, index));
    if (uid) this.selection = [uid];
    return uid ?? null;
  }

  remove(uid: string) {
    this.#run((ed) => ed.remove_node(uid));
    this.selection = this.selection.filter((u) => this.byUid.has(u));
  }

  removeSelected() {
    for (const uid of [...this.selection]) this.remove(uid);
  }

  duplicate(uid: string) {
    const copy = this.#run((ed) => ed.duplicate_node(uid));
    if (copy) this.selection = [copy];
  }

  duplicateSelected() {
    const ids = [...this.selection];
    this.selection = [];
    for (const uid of ids) {
      const copy = this.#run((ed) => ed.duplicate_node(uid));
      if (copy) this.selection = [...this.selection, copy];
    }
  }

  copy(uid: string) {
    this.#run((ed) => ed.copy_node(uid));
    this.hasClipboard = this.#ed?.has_clipboard() ?? false;
  }

  /** Copia e remove. Desfazer restaura o nó na mesma posição. */
  cut(uid: string) {
    this.copy(uid);
    this.remove(uid);
  }

  /** Cola a área de transferência dentro de `parent` (ou na raiz) e seleciona a cópia. */
  paste(parent: string | null = null) {
    const uid = this.#run((ed) => ed.paste_node(parent ?? undefined));
    if (uid) this.selection = [uid];
  }

  rename(uid: string, name: string) {
    this.#run((ed) => ed.rename_node(uid, name));
  }

  /** Define uma ou mais propriedades. Com `coalesceKey`, alterações seguidas viram um passo de desfazer. */
  setProps(uid: string, patch: Record<string, unknown>, coalesceKey?: string) {
    this.#run((ed) => {
      for (const [key, value] of Object.entries(patch)) {
        ed.set_prop(uid, key, JSON.stringify(value), coalesceKey);
      }
    });
  }

  setProp(uid: string, key: string, value: unknown, coalesceKey?: string) {
    this.setProps(uid, { [key]: value }, coalesceKey);
  }

  /** Move o nó para `parent` (ou raiz) na posição final `index` entre os irmãos. */
  move(uid: string, parent: string | null, index: number) {
    this.#run((ed) => ed.move_node(uid, parent ?? undefined, index));
  }

  changeKind(uid: string, kind: string) {
    this.#run((ed) => ed.change_kind(uid, kind));
  }

  /** Fecha o agrupamento de alterações (soltar o mouse, sair de um campo numérico). */
  endGroup() {
    this.#ed?.end_group();
  }

  undo(): string | null {
    const label = this.#run((ed) => ed.undo());
    this.#pruneSelection();
    return label ?? null;
  }

  redo(): string | null {
    const label = this.#run((ed) => ed.redo());
    this.#pruneSelection();
    return label ?? null;
  }

  #pruneSelection() {
    this.selection = this.selection.filter((u) => this.byUid.has(u));
  }
}

export const scene = new SceneState();
