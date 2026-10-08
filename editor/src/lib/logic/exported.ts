// Valores exportados de um script, por nó (VIS-022). O nó guarda só as diferenças do padrão do script, em
// props.script_vars: nome da variável -> valor no formato do grafo. Um UID de nó vai como texto hexadecimal
// ({Node: "00ab..."}), porque um número de 64 bits perde precisão em JavaScript. Texto vazio é nenhum nó (Node(0)).
// Quem confere o mapa é o Rust, ao iniciar a reprodução (nexo-graph, ir.rs). Ver docs/adr/0012.
import type { PinType, Variable } from './model';

/**
 * Valor de uma variável exportada, como o nó o guarda: igual ao `Value` do grafo, exceto Node (texto hexadecimal).
 */
export type StoredValue =
  | { Bool: boolean }
  | { Int: number }
  | { Float: number }
  | { String: string }
  | { Vec2: [number, number] }
  | { Node: string };

/** Mapa `props.script_vars` de um nó: só as variáveis com valor diferente do padrão. */
export type ScriptVars = Record<string, StoredValue>;

/** O mapa de valores de um nó. Valor que não é objeto vale como vazio. */
export function scriptVarsOf(props: Record<string, unknown>): ScriptVars {
  const raw = props.script_vars;
  return raw && typeof raw === 'object' && !Array.isArray(raw) ? (raw as ScriptVars) : {};
}

/** Valor padrão da variável no script, no formato guardado. */
export function defaultOf(variable: Variable): StoredValue {
  const value = variable.default;
  if ('Node' in value) return { Node: value.Node ? value.Node.toString(16).padStart(16, '0') : '' };
  return value;
}

/** Tipo de um valor guardado, pela etiqueta: o mesmo nome do tipo de pino. */
export function typeOf(value: StoredValue): PinType {
  return Object.keys(value)[0] as PinType;
}

/** Igualdade de valores guardados: mesmo tipo e mesmo conteúdo. */
export function sameValue(a: StoredValue, b: StoredValue): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/**
 * Valor que a interface mostra para a variável: o guardado no nó, se for do tipo atual da variável; senão o padrão
 * do script. Um valor de outro tipo (o script mudou depois) é ignorado, como no Rust.
 */
export function valueOf(variable: Variable, vars: ScriptVars): StoredValue {
  const stored = vars[variable.name];
  return stored && typeOf(stored) === variable.ty ? stored : defaultOf(variable);
}

/**
 * True when this node overrides the variable: it holds a value of the variable's type that differs from the default.
 */
export function isOverridden(variable: Variable, vars: ScriptVars): boolean {
  const stored = vars[variable.name];
  return !!stored && typeOf(stored) === variable.ty && !sameValue(stored, defaultOf(variable));
}

/**
 * Mapa com a variável em `value`. Um valor igual ao padrão do script não é guardado: a chave sai do mapa, e o nó
 * volta a seguir o script.
 */
export function withValue(vars: ScriptVars, variable: Variable, value: StoredValue): ScriptVars {
  const next: ScriptVars = { ...vars };
  if (sameValue(value, defaultOf(variable))) delete next[variable.name];
  else next[variable.name] = value;
  return next;
}

// Leitura do valor para cada campo. Só se chama com o valor já do tipo da variável (ver `valueOf`).

export function asNumber(value: StoredValue): number {
  if ('Float' in value) return value.Float;
  if ('Int' in value) return value.Int;
  return 0;
}

export function asBool(value: StoredValue): boolean {
  return 'Bool' in value && value.Bool;
}

export function asText(value: StoredValue): string {
  return 'String' in value ? value.String : '';
}

export function asVec2(value: StoredValue): [number, number] {
  return 'Vec2' in value ? value.Vec2 : [0, 0];
}

export function asNode(value: StoredValue): string {
  return 'Node' in value ? value.Node : '';
}
