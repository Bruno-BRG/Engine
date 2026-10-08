// Workspace de Lógica: a lista de scripts do projeto, o script aberto (um GraphEditor do WASM por script) e a
// seleção, a câmera e o histórico dele. Cada script é um arquivo `scripts/<nome>.logic.ron` (VIS-001). O disco
// é gravado com um atraso curto depois da última alteração, e ao trocar de script ou fechar o projeto.

import { app } from '$lib/state/app.svelte';
import { scene, type NodeType, type NodeView } from '$lib/state/scene.svelte';
import { log } from '$lib/state/log.svelte';
import { createGraphEditor, csharpClassName, readNodeLibrary, readNodeRegistry, type GraphEditor } from '$lib/runtime/engine';
import { isTauri } from '$lib/platform';
import { logicDebug } from './debug.svelte';
import { isPropertyNode } from './properties';
import {
  COMMENT_SIZE,
  type Camera,
  type Rect,
  boundsOf,
  frameCamera,
  nodeSize,
  toWorld,
} from './geometry';
import {
  type Colados,
  type Fragment,
  type GraphError,
  type GraphNode,
  type GraphSnapshot,
  type NodeDef,
  type PinRef,
  type PinType,
  type Refusal,
  type Value,
  type Variable,
  type Wire,
  refusalText,
} from './model';

/** Eventos que só podem aparecer uma vez por script. Os de estado podem repetir, com nomes diferentes. */
const EVENTOS_UNICOS = new Set(['OnReady', 'OnProcess', 'OnPhysicsProcess']);

/** Atraso do salvamento automático, em milissegundos, depois da última alteração. */
const ATRASO_SALVAR = 600;

const VAZIO: GraphSnapshot = { nodes: [], wires: [], comments: [], variables: [], pins: [] };

/** C# of the open script (VIS-024): the generated text, or the graph message that blocks the generation. */
export type CSharpOutput =
  | { ok: true; className: string; code: string }
  | { ok: false; className: string; error: string };

const msg = (e: unknown) => (e instanceof Error ? e.message : String(e));

/** Chama um comando do Tauri. Sem o app desktop, scripts não existem. */
async function call<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  if (!isTauri) throw new Error('Os scripts de lógica precisam do app desktop');
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

/** Ordena nomes sem diferenciar maiúsculas, como o painel de Projeto. */
const porNome = (a: string, b: string) => a.localeCompare(b, 'pt-BR', { sensitivity: 'base' });

class LogicState {
  /** Tipos de nó da biblioteca de lógica (estáticos: lidos uma vez do WASM). */
  defs = $state.raw<NodeDef[]>([]);
  defsByKind = $derived(new Map(this.defs.map((d) => [d.kind, d] as const)));
  /** Registry types (with each one's properties), to pick the type and name of property nodes. */
  nodeTypes = $state.raw<NodeType[]>([]);

  /** Pasta do projeto aberto, ou null. */
  project = $state<string | null>(null);
  /** Nomes dos scripts do projeto, em ordem. */
  names = $state<string[]>([]);
  /** Script aberto, ou null quando nenhum está aberto. */
  active = $state<string | null>(null);
  /** Estado do grafo aberto, copiado do WASM a cada operação. */
  snap = $state.raw<GraphSnapshot>(VAZIO);
  /** Erros de validação do grafo aberto. */
  errors = $state.raw<GraphError[]>([]);
  /**
   * C# generated from the IR of the open script (VIS-024). It is derived state: it is only computed when the panel
   * reads it, and it is recomputed after every graph operation, which replaces `snap`.
   */
  csharp = $derived.by<CSharpOutput | null>(() => {
    void this.snap;
    const script = this.active;
    const ed = this.#ed();
    if (!script || !ed) return null;
    const className = csharpClassName(script);
    try {
      return { ok: true, className, code: ed.to_csharp(script) };
    } catch (e) {
      return { ok: false, className, error: msg(e) };
    }
  });
  /** Seleção do grafo aberto (ids de nós, de caixas e um fio). */
  selNodes = $state<number[]>([]);
  selComments = $state<number[]>([]);
  selWire = $state<Wire | null>(null);
  /** Câmera do canvas. Cada script guarda a sua ao trocar de aba. */
  camera = $state<Camera>({ x: 120, y: 90, zoom: 1 });
  canUndo = $state(false);
  canRedo = $state(false);
  /** Texto copiado (JSON de um trecho). Vale entre scripts. */
  clipboard = $state<string | null>(null);
  /** Tamanho do canvas em pixels, medido pelo componente. */
  viewW = 800;
  viewH = 500;

  /** Nós da cena que usam o script aberto (VIS-003). */
  usedBy = $derived<NodeView[]>(
    this.active ? scene.data.nodes.filter((n) => n.props.script === this.active) : [],
  );
  /** Nós de evento do script aberto (VIS-002). */
  events = $derived(
    this.snap.nodes.filter((n) => this.defsByKind.get(n.kind)?.event),
  );
  hasSelection = $derived(this.selNodes.length > 0 || this.selComments.length > 0 || this.selWire !== null);

  #editors = new Map<string, GraphEditor>();
  #exports = new Map<string, Variable[]>();
  #dirty = new Set<string>();
  #timer: ReturnType<typeof setTimeout> | null = null;
  #seq = 0;
  #cams = new Map<string, Camera>();

  // ---------- Projeto e scripts ----------

  /** Abre o projeto: lê a lista de scripts. Nenhum script fica aberto até o usuário escolher um. */
  async setProject(path: string) {
    this.unload();
    this.project = path;
    try {
      if (!this.defs.length) {
        this.defs = JSON.parse(await readNodeLibrary()) as NodeDef[];
        this.nodeTypes = JSON.parse(readNodeRegistry()) as NodeType[];
      }
      await this.#reload();
    } catch (e) {
      log.toast('error', 'Não foi possível ler os scripts', msg(e));
    }
  }

  /** Fecha o projeto: salva o que falta e libera os grafos. */
  async close() {
    await this.flush();
    this.unload();
  }

  unload() {
    if (this.#timer) clearTimeout(this.#timer);
    this.#timer = null;
    for (const ed of this.#editors.values()) ed.free();
    this.#editors.clear();
    this.#exports.clear();
    this.#dirty.clear();
    this.#cams.clear();
    this.project = null;
    this.names = [];
    this.active = null;
    this.snap = VAZIO;
    this.errors = [];
    this.selNodes = [];
    this.selComments = [];
    this.selWire = null;
    this.canUndo = false;
    this.canRedo = false;
  }

  async #reload() {
    if (!this.project) return;
    const lista = await call<string[]>('list_scripts', { path: this.project });
    this.names = [...lista].sort(porNome);
  }

  /** Abre um script. Salva antes o que estava pendente. */
  async open(name: string) {
    if (!this.project) return;
    const seq = ++this.#seq;
    try {
      await this.flush();
      let ed = this.#editors.get(name);
      if (!ed) {
        const text = await call<string>('load_script', { path: this.project, name });
        const novo = await createGraphEditor();
        novo.load_ron(text);
        ed = novo;
        this.#editors.set(name, ed);
      }
      if (seq !== this.#seq) return;
      this.#enter(name);
    } catch (e) {
      log.toast('error', 'Não foi possível abrir o script', msg(e));
    }
  }

  #enter(name: string) {
    if (this.active) this.#cams.set(this.active, this.camera);
    this.active = name;
    this.camera = this.#cams.get(name) ?? { x: 120, y: 90, zoom: 1 };
    this.selNodes = [];
    this.selComments = [];
    this.selWire = null;
    this.#refresh();
  }

  /** Abre o script escolhido e leva o usuário ao workspace de Lógica (duplo clique no Projeto, Inspector). */
  async openIn(name: string) {
    app.workspace = 'logica';
    await this.open(name);
  }

  /** Cria um script vazio com o próximo nome livre ("Script 1", "Script 2"...) e o abre. */
  async create() {
    if (!this.project) return;
    const usados = new Set(this.names.map((n) => n.toLowerCase()));
    let n = 1;
    while (usados.has(`script ${n}`)) n++;
    try {
      const name = await call<string>('create_script', { path: this.project, name: `Script ${n}` });
      await this.#reload();
      await this.open(name);
      return name;
    } catch (e) {
      log.toast('error', 'Não foi possível criar o script', msg(e));
    }
  }

  /** Renomeia um script. Os nós que o usam passam a apontar para o nome novo. */
  async rename(from: string, to: string): Promise<boolean> {
    if (!this.project) return false;
    await this.flush();
    try {
      await call('rename_script', { path: this.project, from, to });
    } catch (e) {
      log.toast('error', 'Não foi possível renomear', msg(e));
      return false;
    }
    const ed = this.#editors.get(from);
    if (ed) {
      this.#editors.delete(from);
      this.#editors.set(to, ed);
    }
    const cam = this.#cams.get(from);
    if (cam) this.#cams.delete(from);
    if (this.active === from) this.active = to;
    if (cam && this.active === to) this.#cams.set(to, cam);
    for (const n of scene.data.nodes.filter((x) => x.props.script === from)) {
      scene.setProp(n.uid, 'script', to);
    }
    await this.#reload();
    return true;
  }

  /** Copia o script para “<nome> cópia” e abre a cópia. */
  async duplicate(name: string) {
    if (!this.project) return;
    await this.flush();
    try {
      const novo = await call<string>('duplicate_script', { path: this.project, name });
      await this.#reload();
      await this.open(novo);
    } catch (e) {
      log.toast('error', 'Não foi possível duplicar o script', msg(e));
    }
  }

  /** Manda o script para a lixeira. Os nós que o usavam ficam sem script. */
  async remove(name: string) {
    if (!this.project) return;
    await this.flush();
    try {
      await call('delete_script', { path: this.project, name });
    } catch (e) {
      log.toast('error', 'Não foi possível excluir o script', msg(e));
      return;
    }
    for (const n of scene.data.nodes.filter((x) => x.props.script === name)) {
      scene.setProp(n.uid, 'script', '');
    }
    this.#editors.get(name)?.free();
    this.#editors.delete(name);
    this.#exports.delete(name);
    this.#dirty.delete(name);
    this.#cams.delete(name);
    if (this.active === name) {
      this.active = null;
      this.snap = VAZIO;
      this.errors = [];
      this.selNodes = [];
      this.selComments = [];
      this.selWire = null;
    }
    await this.#reload();
    log.toast('success', 'Script enviado à lixeira', name);
  }

  /** Salva os scripts alterados. Chamado pelo atraso, ao trocar de script e ao fechar o projeto. */
  async flush() {
    if (this.#timer) clearTimeout(this.#timer);
    this.#timer = null;
    const pendentes = [...this.#dirty];
    this.#dirty.clear();
    for (const name of pendentes) {
      const ed = this.#editors.get(name);
      if (!ed || !this.project) continue;
      try {
        await call('save_script', { path: this.project, name, text: ed.to_ron() });
        this.#exports.delete(name);
      } catch (e) {
        this.#dirty.add(name);
        log.toast('error', 'Falha ao salvar o script', `${name}: ${msg(e)}`);
      }
    }
  }

  /** Texto RON dos scripts pedidos, para a reprodução (VIS-023). Um script aberto vale pelo que está na tela, mesmo sem salvar. */
  async textsForPlay(names: string[]): Promise<Map<string, string>> {
    const textos = new Map<string, string>();
    for (const name of names) {
      const aberto = this.#editors.get(name);
      if (aberto) {
        textos.set(name, aberto.to_ron());
        continue;
      }
      if (!this.project) throw new Error('Nenhum projeto aberto.');
      textos.set(name, await call<string>('load_script', { path: this.project, name }));
    }
    return textos;
  }

  /** Variáveis exportadas de um script (INS-005), mesmo que ele não esteja aberto. */
  async exportedOf(name: string): Promise<Variable[]> {
    if (name === this.active) return this.snap.variables.filter((v) => v.exported);
    const cache = this.#exports.get(name);
    if (cache) return cache;
    if (!this.project) return [];
    const text = await call<string>('load_script', { path: this.project, name });
    const ed = await createGraphEditor();
    try {
      ed.load_ron(text);
      const snap = JSON.parse(ed.snapshot()) as GraphSnapshot;
      const exportadas = snap.variables.filter((v) => v.exported);
      this.#exports.set(name, exportadas);
      return exportadas;
    } finally {
      ed.free();
    }
  }

  /** Liga o script ao nó selecionado na cena (VIS-003). */
  attachToSelected(): boolean {
    const uid = scene.selectedId;
    if (!this.active || !uid) return false;
    scene.setProp(uid, 'script', this.active);
    return true;
  }

  /** Tira o script de um nó da cena. */
  detach(uid: string) {
    scene.setProp(uid, 'script', '');
  }

  // ---------- Operações do grafo ----------

  #ed(): GraphEditor | null {
    return this.active ? (this.#editors.get(this.active) ?? null) : null;
  }

  #refresh() {
    const ed = this.#ed();
    if (!ed) {
      this.snap = VAZIO;
      this.errors = [];
      this.canUndo = false;
      this.canRedo = false;
      return;
    }
    this.snap = JSON.parse(ed.snapshot()) as GraphSnapshot;
    this.errors = JSON.parse(ed.validate()) as GraphError[];
    this.canUndo = ed.can_undo();
    this.canRedo = ed.can_redo();
    const nos = new Set(this.snap.nodes.map((n) => n.id));
    const caixas = new Set(this.snap.comments.map((c) => c.id));
    this.selNodes = this.selNodes.filter((id) => nos.has(id));
    this.selComments = this.selComments.filter((id) => caixas.has(id));
  }

  #touched() {
    if (!this.active) return;
    this.#dirty.add(this.active);
    this.#exports.delete(this.active);
    if (this.#timer) clearTimeout(this.#timer);
    this.#timer = setTimeout(() => void this.flush(), ATRASO_SALVAR);
  }

  /** Executa uma operação do WASM. Erros viram um aviso com o motivo (VIS-013). Devolve se deu certo. */
  #run(titulo: string, fn: (ed: GraphEditor) => void): boolean {
    const ed = this.#ed();
    if (!ed) return false;
    let ok = false;
    try {
      fn(ed);
      ok = true;
    } catch (e) {
      log.toast('error', titulo, msg(e));
    }
    this.#refresh();
    if (ok) this.#touched();
    return ok;
  }

  /** Refusal and suggestion for linking `from` to `to`, or undefined when the link is valid. Changes nothing. */
  checkConnection(from: PinRef, to: PinRef): Refusal | undefined {
    const json = this.#ed()?.check_connection(from.node, from.pin, to.node, to.pin);
    return json ? (JSON.parse(json) as Refusal) : undefined;
  }

  /**
   * Links two pins (VIS-013). A refused link shows the refusal and its suggestion, and the graph stays as it was. With
   * `replaces` (the input that already had a wire), that old wire leaves only after the new link is accepted: if the
   * new link is refused, the old wire stays. Dropping the wire back on the same input changes nothing.
   */
  connect(from: PinRef, to: PinRef, replaces: PinRef | null = null): boolean {
    if (replaces && replaces.node === to.node && replaces.pin === to.pin) return true;
    const refusal = this.checkConnection(from, to);
    if (refusal) {
      log.toast('warn', 'Ligação recusada', refusalText(refusal));
      return false;
    }
    const ok = this.#run('Ligação recusada', (ed) => ed.connect(from.node, from.pin, to.node, to.pin));
    if (ok && replaces) this.disconnectInput(replaces.node, replaces.pin);
    return ok;
  }

  disconnect(w: Wire) {
    this.#run('Não foi possível desconectar', (ed) =>
      ed.disconnect(w.from.node, w.from.pin, w.to.node, w.to.pin),
    );
    this.selWire = null;
  }

  disconnectInput(node: number, pin: string) {
    this.#run('Não foi possível desconectar', (ed) => {
      ed.disconnect_input(node, pin);
    });
  }

  /** Adiciona um nó e o seleciona. Devolve o id, ou null se o tipo não existe. */
  addNode(kind: string, x: number, y: number): number | null {
    const out: { id?: number } = {};
    this.#run('Não foi possível adicionar o nó', (ed) => {
      out.id = ed.add_node(kind, x, y);
    });
    if (out.id !== undefined) this.select([out.id], [], false);
    return out.id ?? null;
  }

  addReroute(x: number, y: number): number | null {
    const out: { id?: number } = {};
    this.#run('Não foi possível adicionar o reroute', (ed) => {
      out.id = ed.add_reroute(x, y);
    });
    if (out.id !== undefined) this.select([out.id], [], false);
    return out.id ?? null;
  }

  /** Caixa de comentário centrada na tela (botão da barra). */
  addCommentAtCenter(): number | null {
    const [x, y] = toWorld(this.camera, this.viewW / 2, this.viewH / 2);
    return this.addComment(Math.round(x - COMMENT_SIZE[0] / 2), Math.round(y - COMMENT_SIZE[1] / 2));
  }

  addComment(x: number, y: number, text = 'Comentário'): number | null {
    const out: { id?: number } = {};
    this.#run('Não foi possível adicionar a caixa', (ed) => {
      out.id = ed.add_comment(text, x, y, COMMENT_SIZE[0], COMMENT_SIZE[1]);
    });
    if (out.id !== undefined) this.select([], [out.id], false);
    return out.id ?? null;
  }

  /** Adiciona um evento (VIS-002) no centro da tela. Eventos de quadro só aparecem uma vez por script. */
  addEvent(kind: string): number | null {
    if (EVENTOS_UNICOS.has(kind) && this.events.some((n) => n.kind === kind)) {
      log.toast('warn', 'Esse evento já existe', 'Cada script tem um só por tipo de evento.');
      return null;
    }
    const [x, y] = toWorld(this.camera, this.viewW / 2, this.viewH / 2);
    return this.addNode(kind, Math.round(x), Math.round(y));
  }

  setPosition(id: number, x: number, y: number) {
    this.#run('Não foi possível mover', (ed) => ed.set_position(id, x, y));
  }

  setCommentBox(id: number, text: string, x: number, y: number, w: number, h: number) {
    this.#run('Não foi possível atualizar a caixa', (ed) => ed.set_comment(id, text, x, y, w, h));
  }

  /** Valor embutido de uma entrada (campo do próprio nó). */
  setValue(node: number, pin: string, value: Value) {
    const cleared = this.#clearsTypedValue(node, pin, value);
    const ok = this.#runDroppingWires('Valor recusado', (ed) => ed.set_value(node, pin, JSON.stringify(value)));
    if (ok && cleared) this.#warnValueCleared();
  }

  removeValue(node: number, pin: string) {
    const cleared = this.#clearsTypedValue(node, pin, undefined);
    const ok = this.#runDroppingWires('Não foi possível limpar o valor', (ed) => ed.remove_value(node, pin));
    if (ok && cleared) this.#warnValueCleared();
  }

  /** True when setting `pin` of a property node to `next` (undefined when removed) clears its typed value (VIS-021). */
  #clearsTypedValue(node: number, pin: string, next: Value | undefined): boolean {
    if (pin !== 'type' && pin !== 'name') return false;
    const n = this.snap.nodes.find((x) => x.id === node);
    if (!n || !isPropertyNode(n.kind) || !n.values?.value) return false;
    return JSON.stringify(n.values[pin]) !== JSON.stringify(next);
  }

  #warnValueCleared() {
    log.toast('warn', 'Valor digitado limpo', 'Trocar o tipo ou o nome de um nó de propriedade tira o valor do pino de valor.');
  }

  /**
   * Like `#run`, for edits that can drop wires: changing the type or the name of a property node may leave wires on its
   * value pin that no longer match. The wires the engine returns are dropped, and the user gets a warning (VIS-021).
   */
  #runDroppingWires(title: string, fn: (ed: GraphEditor) => string): boolean {
    let dropped: Wire[] = [];
    const ok = this.#run(title, (ed) => {
      dropped = JSON.parse(fn(ed)) as Wire[];
    });
    if (ok && dropped.length > 0) {
      const summary = dropped.length === 1 ? 'Um fio foi desligado' : `${dropped.length} fios foram desligados`;
      log.toast(
        'warn',
        'Fio desligado no pino de valor',
        `${summary}: o tipo ou o nome do nó de propriedade mudou e o pino não aceita mais o valor de antes.`,
      );
    }
    return ok;
  }

  setVariable(name: string, ty: PinType, def: Value, exported: boolean): boolean {
    return this.#run('Variável recusada', (ed) =>
      ed.set_variable(name, JSON.stringify(ty), JSON.stringify(def), exported),
    );
  }

  removeVariable(name: string) {
    this.#run('Não foi possível remover a variável', (ed) => ed.remove_variable(name));
  }

  /** Abre um grupo de edições: arrastar um nó vira um passo de desfazer só. */
  beginGroup() {
    this.#ed()?.begin_group();
  }

  endGroup() {
    this.#ed()?.end_group();
  }

  /** Apaga a seleção: o fio escolhido, ou os nós e as caixas selecionados (com os fios deles). Um passo de desfazer. */
  removeSelection() {
    if (this.selWire) {
      this.disconnect(this.selWire);
      return;
    }
    const ed = this.#ed();
    if (!ed || !this.hasSelection) return;
    const nos = [...this.selNodes];
    const caixas = [...this.selComments];
    ed.begin_group();
    this.#run('Não foi possível apagar', (e) => {
      for (const id of nos) e.remove_node(id);
      for (const id of caixas) e.remove_comment(id);
    });
    ed.end_group();
    this.selNodes = [];
    this.selComments = [];
  }

  removeComment(id: number) {
    this.#run('Não foi possível apagar a caixa', (ed) => ed.remove_comment(id));
    this.selComments = this.selComments.filter((c) => c !== id);
  }

  /** Copia a seleção para a área de transferência do editor. */
  copySelection(): boolean {
    const ed = this.#ed();
    if (!ed || !this.hasSelection) return false;
    this.clipboard = ed.copy_fragment(Float64Array.from(this.selNodes), Float64Array.from(this.selComments));
    return true;
  }

  /** Cola o trecho. Com (x, y), o canto do trecho cai ali; sem, desloca 40 px da origem. Seleciona o colado. */
  paste(x?: number, y?: number): boolean {
    const texto = this.clipboard;
    if (!texto) return false;
    const trecho = JSON.parse(texto) as Fragment;
    let dx = 40;
    let dy = 40;
    const pontos = [...trecho.nodes.map((n) => n.position), ...trecho.comments.map((c) => c.position)];
    if (x !== undefined && y !== undefined && pontos.length) {
      dx = x - Math.min(...pontos.map((p) => p[0]));
      dy = y - Math.min(...pontos.map((p) => p[1]));
    }
    const out: { novos?: Colados } = {};
    this.#run('Não foi possível colar', (ed) => {
      out.novos = JSON.parse(ed.paste_fragment(texto, dx, dy)) as Colados;
    });
    if (out.novos) this.select(out.novos.nodes, out.novos.comments, false);
    return out.novos !== undefined;
  }

  /** Duplica a seleção, deslocada. */
  duplicateSelection(): boolean {
    return this.copySelection() && this.paste();
  }

  undo(): boolean {
    const ed = this.#ed();
    if (!ed) return false;
    const ok = ed.undo();
    this.#refresh();
    if (ok) this.#touched();
    return ok;
  }

  redo(): boolean {
    const ed = this.#ed();
    if (!ed) return false;
    const ok = ed.redo();
    this.#refresh();
    if (ok) this.#touched();
    return ok;
  }

  /** Colapsa a seleção numa função com o nome dado (VIS-015). Seleciona o Chamar função que a substitui. */
  collapseSelection(name: string): boolean {
    const out: { id?: number } = {};
    const ok = this.#run('Não foi possível colapsar', (ed) => {
      out.id = ed.collapse_to_function(Float64Array.from(this.selNodes), name);
    });
    if (ok && out.id !== undefined) this.select([out.id], [], false);
    return ok;
  }

  // ---------- Seleção, câmera e pontos de parada ----------

  /** Seleciona nós e caixas. Com `additive`, acrescenta (ou tira, se já estava) em vez de substituir. */
  select(nodes: number[], comments: number[], additive: boolean) {
    if (additive) {
      const nos = new Set(this.selNodes);
      for (const id of nodes) (nos.has(id) ? nos.delete(id) : nos.add(id));
      const caixas = new Set(this.selComments);
      for (const id of comments) (caixas.has(id) ? caixas.delete(id) : caixas.add(id));
      this.selNodes = [...nos];
      this.selComments = [...caixas];
    } else {
      this.selNodes = [...nodes];
      this.selComments = [...comments];
    }
    this.selWire = null;
  }

  selectWire(w: Wire | null) {
    this.selNodes = [];
    this.selComments = [];
    this.selWire = w;
  }

  clearSelection() {
    this.selNodes = [];
    this.selComments = [];
    this.selWire = null;
  }

  selectAll() {
    this.select(this.snap.nodes.map((n) => n.id), this.snap.comments.map((c) => c.id), false);
  }

  /** Retângulo de um nó, no mundo. */
  rectOf(node: GraphNode): Rect {
    const [w, h] = nodeSize(node.kind, this.defsByKind.get(node.kind));
    return [node.position[0], node.position[1], w, h];
  }

  /** Enquadra todo o grafo no canvas. */
  frameAll() {
    const rects: Rect[] = [
      ...this.snap.nodes.map((n) => this.rectOf(n)),
      ...this.snap.comments.map((c) => [c.position[0], c.position[1], c.size[0], c.size[1]] as Rect),
    ];
    this.#frame(rects);
  }

  /** Enquadra a seleção (ou tudo, se nada está selecionado). */
  frameSelection() {
    if (!this.hasSelection) return this.frameAll();
    const rects: Rect[] = [
      ...this.snap.nodes.filter((n) => this.selNodes.includes(n.id)).map((n) => this.rectOf(n)),
      ...this.snap.comments
        .filter((c) => this.selComments.includes(c.id))
        .map((c) => [c.position[0], c.position[1], c.size[0], c.size[1]] as Rect),
    ];
    this.#frame(rects);
  }

  #frame(rects: Rect[]) {
    const r = boundsOf(rects);
    if (!r) {
      this.camera = { x: this.viewW / 2, y: this.viewH / 2, zoom: 1 };
      return;
    }
    this.camera = frameCamera(r, this.viewW, this.viewH);
  }

  /** Liga ou desliga o ponto de parada dos nós selecionados (VIS-031). */
  toggleBreakpoint() {
    if (!this.active || !this.selNodes.length) return;
    logicDebug.toggleBreakpoints(this.active, this.selNodes);
  }

  /** Nome de um nó para as mensagens e para o painel de eventos. */
  labelOf(node: GraphNode): string {
    return this.defsByKind.get(node.kind)?.label ?? node.kind;
  }
}

export const logic = new LogicState();
