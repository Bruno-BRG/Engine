// Cena em edição, seleção e histórico de desfazer/refazer.

export type ShapeKind = 0 | 1 | 2; // retângulo, círculo, losango
export type BehaviorKind = 'girar' | 'flutuar' | 'pulsar';

export interface Behavior {
  kind: BehaviorKind;
  speed: number;
}

export interface SceneObject {
  id: string;
  name: string;
  shape: ShapeKind;
  x: number;
  y: number;
  w: number;
  h: number;
  rotation: number; // graus
  color: string;
  visible: boolean;
  locked: boolean;
  behaviors: Behavior[];
}

export interface SceneData {
  version: 1;
  name: string;
  screen: [number, number];
  objects: SceneObject[];
}

export const SHAPE_NAMES = ['Retângulo', 'Círculo', 'Losango'] as const;

export const BEHAVIORS: Record<BehaviorKind, { name: string; description: string; unit: string; default: number }> = {
  girar: { name: 'Girar', description: 'Gira o objeto continuamente.', unit: '°/s', default: 90 },
  flutuar: { name: 'Flutuar', description: 'Sobe e desce suavemente.', unit: 'ciclos/s', default: 1 },
  pulsar: { name: 'Pulsar', description: 'Aumenta e diminui de tamanho.', unit: 'ciclos/s', default: 1.5 },
};

export const newId = () => Math.random().toString(36).slice(2, 10);

export function emptyScene(name = 'Principal'): SceneData {
  return { version: 1, name, screen: [1280, 720], objects: [] };
}

const MAX_HISTORY = 200;

class SceneState {
  data = $state<SceneData>(emptyScene());
  selectedId = $state<string | null>(null);
  hoveredId = $state<string | null>(null);
  /** Incrementa a cada mudança; o viewport e o salvamento automático observam este valor. */
  revision = $state(0);
  canUndo = $state(false);
  canRedo = $state(false);
  lastAction = $state('');

  #undo: { label: string; snap: string }[] = [];
  #redo: { label: string; snap: string }[] = [];
  #coalesceKey: string | null = null;
  #coalesceAt = 0;

  selected = $derived(this.data.objects.find((o) => o.id === this.selectedId) ?? null);

  load(data: SceneData) {
    this.data = data;
    this.selectedId = null;
    this.hoveredId = null;
    this.#undo = [];
    this.#redo = [];
    this.#sync();
    this.revision++;
  }

  snapshot(): SceneData {
    return $state.snapshot(this.data) as SceneData;
  }

  /**
   * Aplica uma alteração registrando um passo de desfazer.
   * Alterações seguidas com a mesma chave (arrastar, scrub) viram um passo só.
   */
  mutate(label: string, fn: (d: SceneData) => void, coalesceKey?: string) {
    const t = performance.now();
    const merge =
      coalesceKey != null &&
      coalesceKey === this.#coalesceKey &&
      (coalesceKey.startsWith('drag:') || t - this.#coalesceAt < 1000);
    if (!merge) {
      this.#undo.push({ label, snap: JSON.stringify(this.snapshot()) });
      if (this.#undo.length > MAX_HISTORY) this.#undo.shift();
      this.#redo = [];
    }
    this.#coalesceKey = coalesceKey ?? null;
    this.#coalesceAt = t;
    fn(this.data);
    this.lastAction = label;
    this.#sync();
    this.revision++;
  }

  /** Encerra o agrupamento atual (por exemplo, ao soltar o mouse). */
  endGroup() {
    this.#coalesceKey = null;
  }

  undo() {
    const step = this.#undo.pop();
    if (!step) return null;
    this.#redo.push({ label: step.label, snap: JSON.stringify(this.snapshot()) });
    this.#restore(step.snap);
    return step.label;
  }

  redo() {
    const step = this.#redo.pop();
    if (!step) return null;
    this.#undo.push({ label: step.label, snap: JSON.stringify(this.snapshot()) });
    this.#restore(step.snap);
    return step.label;
  }

  #restore(snap: string) {
    this.data = JSON.parse(snap);
    if (this.selectedId && !this.data.objects.some((o) => o.id === this.selectedId)) this.selectedId = null;
    this.#coalesceKey = null;
    this.#sync();
    this.revision++;
  }

  #sync() {
    this.canUndo = this.#undo.length > 0;
    this.canRedo = this.#redo.length > 0;
  }

  // ---------- Operações comuns ----------

  uniqueName(base: string) {
    const names = new Set(this.data.objects.map((o) => o.name));
    if (!names.has(base)) return base;
    let i = 2;
    while (names.has(base + ' ' + i)) i++;
    return base + ' ' + i;
  }

  add(shape: ShapeKind, x: number, y: number, extra: Partial<SceneObject> = {}) {
    const palette = ['#7c5cff', '#3ddc97', '#ffb84d', '#ff5c7a', '#5cc8ff', '#ff7ad9'];
    const obj: SceneObject = {
      id: newId(),
      name: this.uniqueName(SHAPE_NAMES[shape]),
      shape,
      x: Math.round(x),
      y: Math.round(y),
      w: 96,
      h: 96,
      rotation: 0,
      color: palette[this.data.objects.length % palette.length],
      visible: true,
      locked: false,
      behaviors: [],
      ...extra,
    };
    this.mutate('Criar ' + obj.name, (d) => d.objects.push(obj));
    this.selectedId = obj.id;
    return obj;
  }

  remove(id: string) {
    const obj = this.data.objects.find((o) => o.id === id);
    if (!obj) return;
    this.mutate('Excluir ' + obj.name, (d) => {
      d.objects = d.objects.filter((o) => o.id !== id);
    });
    if (this.selectedId === id) this.selectedId = null;
  }

  duplicate(id: string) {
    const src = this.data.objects.find((o) => o.id === id);
    if (!src) return;
    const copy: SceneObject = {
      ...structuredClone($state.snapshot(src) as SceneObject),
      id: newId(),
      name: this.uniqueName(src.name),
      x: src.x + 32,
      y: src.y + 32,
    };
    this.mutate('Duplicar ' + src.name, (d) => {
      const i = d.objects.findIndex((o) => o.id === id);
      d.objects.splice(i + 1, 0, copy);
    });
    this.selectedId = copy.id;
  }

  update(id: string, patch: Partial<SceneObject>, label = 'Editar', coalesceKey?: string) {
    this.mutate(
      label,
      (d) => {
        const o = d.objects.find((o) => o.id === id);
        if (o) Object.assign(o, patch);
      },
      coalesceKey,
    );
  }

  move(id: string, dir: -1 | 1) {
    const i = this.data.objects.findIndex((o) => o.id === id);
    const j = i + dir;
    if (i < 0 || j < 0 || j >= this.data.objects.length) return;
    this.mutate('Reordenar', (d) => {
      const [o] = d.objects.splice(i, 1);
      d.objects.splice(j, 0, o);
    });
  }
}

export const scene = new SceneState();
