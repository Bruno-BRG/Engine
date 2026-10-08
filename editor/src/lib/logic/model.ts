// Tipos do grafo de lógica como o WASM (GraphEditor) os envia em JSON. Os nomes seguem o Rust: `ty` e `kind`
// são os nomes dos tipos, e `Value` é a forma enumerada do serde ({"Float": 2.5}).

/** Tipo de um pino. `Flow` é a ordem de execução. */
export type PinType = 'Flow' | 'Bool' | 'Int' | 'Float' | 'String' | 'Vec2' | 'Node';

/** Valor embutido, de um padrão ou de uma variável. */
export type Value =
  | { Bool: boolean }
  | { Int: number }
  | { Float: number }
  | { String: string }
  | { Vec2: [number, number] }
  | { Node: number };

export interface PinDef {
  name: string;
  label: string;
  /** null quando o tipo segue a variável nomeada ou o fio que chega (reroute). */
  ty: PinType | null;
  default: Value | null;
}

/** Tipo de nó da biblioteca (library.rs). */
export interface NodeDef {
  kind: string;
  category: string;
  label: string;
  description: string;
  event: boolean;
  inputs: PinDef[];
  outputs: PinDef[];
}

export interface GraphNode {
  id: number;
  kind: string;
  position: [number, number];
  /** Valores embutidos. Ausente quando o nó não tem nenhum: o Rust omite o campo vazio. */
  values?: Record<string, Value>;
}

export interface PinRef {
  node: number;
  pin: string;
}

export interface Wire {
  from: PinRef;
  to: PinRef;
}

export interface GraphComment {
  id: number;
  text: string;
  position: [number, number];
  size: [number, number];
}

export interface Variable {
  name: string;
  ty: PinType;
  default: Value;
  exported: boolean;
}

/** Tipo resolvido de um pino (VIS-013). `ty` null quer dizer que ainda não tem tipo. */
export interface ResolvedPin {
  node: number;
  pin: string;
  output: boolean;
  ty: PinType | null;
}

/** Estado completo do grafo, para a interface. */
export interface GraphSnapshot {
  nodes: GraphNode[];
  wires: Wire[];
  comments: GraphComment[];
  variables: Variable[];
  pins: ResolvedPin[];
}

/** Erro de validação do grafo. `suggestion` é null quando não há um jeito claro de corrigir. */
export interface GraphError {
  node: number | null;
  message: string;
  suggestion: string | null;
}

/** Motivo de uma ligação recusada (VIS-013): o que aconteceu e, quando há, como corrigir. */
export interface Recusa {
  message: string;
  suggestion: string | null;
}

/** Texto de um motivo para o toast e para os títulos: o que aconteceu, seguido da sugestão. */
export function textoRecusa(r: Recusa): string {
  return r.suggestion ? `${r.message}. ${r.suggestion}` : r.message;
}

/** Trecho copiado (copy_fragment): o que vai para a área de transferência, no mesmo formato do Rust. */
export interface Fragment {
  nodes: GraphNode[];
  wires: Wire[];
  comments: GraphComment[];
  variables: Variable[];
}

/** Ids novos depois de colar (paste_fragment). */
export interface Colados {
  nodes: number[];
  comments: number[];
}

/** Evento de rastro do VM (vm.rs, TraceEvent): nó executado, ou valor produzido pelo nó. */
export type TraceEvent = { Executou: { evento: string; node: number } } | { Valor: { node: number; valor: Value } };

/** Forma visual de um pino: cada tipo tem uma cor e uma forma (VIS-013, docs/04-visual-programming.md). */
export type PinShape = 'tri' | 'diamond' | 'square' | 'circle' | 'hex' | 'ring';

export interface PinStyle {
  color: string;
  shape: PinShape;
  label: string;
}

const ESTILO: Record<PinType, PinStyle> = {
  Flow: { color: 'var(--type-exec)', shape: 'tri', label: 'Fluxo' },
  Bool: { color: 'var(--type-bool)', shape: 'diamond', label: 'Booleano' },
  Int: { color: 'var(--type-int)', shape: 'square', label: 'Inteiro' },
  Float: { color: 'var(--type-float)', shape: 'circle', label: 'Decimal' },
  String: { color: 'var(--type-text)', shape: 'hex', label: 'Texto' },
  Vec2: { color: 'var(--type-vec)', shape: 'hex', label: 'Vetor 2D' },
  Node: { color: 'var(--type-object)', shape: 'ring', label: 'Nó' },
};

/** Cor e forma do pino de um tipo. Pino sem tipo ainda usa a cor neutra do fluxo. */
export function pinStyle(ty: PinType | null | undefined): PinStyle {
  return ty ? ESTILO[ty] : { color: 'var(--text-faint)', shape: 'circle', label: 'Sem tipo' };
}

/** Valor de um tipo, a partir do texto digitado. Devolve null quando o texto não serve para o tipo. */
export function valueFromText(ty: PinType, text: string): Value | null {
  switch (ty) {
    case 'Bool':
      return { Bool: text === 'true' };
    case 'Int': {
      const n = Number(text);
      return Number.isFinite(n) ? { Int: Math.round(n) } : null;
    }
    case 'Float': {
      const n = Number(text.replace(',', '.'));
      return Number.isFinite(n) ? { Float: n } : null;
    }
    case 'String':
      return { String: text };
    case 'Node': {
      const n = Number(text);
      return Number.isFinite(n) ? { Node: Math.round(n) } : null;
    }
    default:
      return null;
  }
}

/** Valor de um tipo com o zero dele (padrão ao criar uma variável). */
export function zeroOf(ty: PinType): Value {
  switch (ty) {
    case 'Bool':
      return { Bool: false };
    case 'Int':
      return { Int: 0 };
    case 'String':
      return { String: '' };
    case 'Vec2':
      return { Vec2: [0, 0] };
    case 'Node':
      return { Node: 0 };
    default:
      return { Float: 0 };
  }
}

/** Texto de exibição de um valor (sem o nome do tipo). */
export function textOf(v: Value | null | undefined): string {
  if (!v) return '';
  if ('Bool' in v) return v.Bool ? 'verdadeiro' : 'falso';
  if ('Int' in v) return String(v.Int);
  if ('Float' in v) return String(Math.round(v.Float * 1000) / 1000);
  if ('String' in v) return v.String;
  if ('Vec2' in v) return `(${v.Vec2[0]}, ${v.Vec2[1]})`;
  return `nó ${v.Node}`;
}

/** Nome do tipo em português, para as mensagens. */
export const TIPO_NOME: Record<PinType, string> = {
  Flow: 'fluxo',
  Bool: 'booleano',
  Int: 'inteiro',
  Float: 'decimal',
  String: 'texto',
  Vec2: 'vetor 2D',
  Node: 'nó',
};

/** Tipos que uma variável pode ter (fluxo não). */
export const TIPOS_VARIAVEL: PinType[] = ['Float', 'Int', 'Bool', 'String', 'Vec2', 'Node'];

/** Chave estável de um pino, para mapas e atributos: "nó.pino". */
export const pinKey = (node: number, pin: string) => `${node}.${pin}`;
