// Biblioteca de nós na interface: cor de cada família, nomes em inglês para a busca (VIS-011) e a busca difusa
// com filtro pelo tipo do pino onde o fio foi solto.

import { fuzzyScore } from '$lib/fuzzy';
import type { NodeDef, PinType } from './model';

/** Cor de um nó pela categoria. As cores são as da Fase 0 (tokens.css). */
export function familyOf(category: string): string {
  switch (category) {
    case 'Eventos':
      return 'var(--node-event)';
    case 'Fluxo':
    case 'Organização':
      return 'var(--node-flow)';
    case 'Matemática':
    case 'Comparação':
    case 'Vetor 2D':
    case 'Texto':
    case 'Variáveis':
      return 'var(--node-data)';
    case 'Física':
      return 'var(--node-physics)';
    case 'Áudio':
      return 'var(--node-audio)';
    case 'Entrada':
    case 'Depuração':
      return 'var(--node-ui)';
    default:
      return 'var(--node-object)';
  }
}

/** Nomes em inglês e sinônimos, para achar um nó pelo nome que o usuário conhece (EN e PT). */
const EN: Record<string, string> = {
  OnReady: 'on ready create start awake begin',
  OnProcess: 'on process update step frame tick every frame',
  OnPhysicsProcess: 'on physics process fixed step physics tick',
  OnCollision: 'on collision collide hit touch contact with',
  OnBodyEntered: 'on body entered enter area overlap trigger coin',
  Branch: 'if branch condition',
  Sequence: 'sequence then order',
  ForEach: 'for each loop repeat',
  While: 'while loop repeat',
  Wait: 'wait delay sleep pause',
  Tween: 'tween interpolate lerp animate',
  MoveTo: 'move to target position',
  AwaitSignal: 'await signal wait for',
  EmitSignal: 'emit signal send fire',
  TimerStart: 'timer start countdown delay',
  Add: 'add plus sum +',
  Subtract: 'subtract minus difference -',
  Multiply: 'multiply times product *',
  Divide: 'divide quotient /',
  Greater: 'greater than bigger >',
  Less: 'less than smaller <',
  Equal: 'equal same ==',
  MakeVector2: 'make vector2 vec2 xy',
  AddVector2: 'add vector2 vec2 sum',
  ScaleVector2: 'scale vector2 vec2 multiply',
  VectorLength: 'length magnitude vector2 size',
  Concat: 'concat join text string append',
  Print: 'print log debug show write',
  Warn: 'warn warning alert',
  Error: 'error fail exception',
  GetProperty: 'get property read value',
  SetProperty: 'set property write assign',
  GetNode: 'get node find by name',
  InstanceScene: 'instance spawn scene create copy',
  Destroy: 'destroy remove delete kill',
  ActionPressed: 'action pressed input key button held',
  ActionJustPressed: 'action just pressed input key button',
  ActionReleased: 'action released input key button up',
  GetAxis: 'axis input negative positive',
  GetVector: 'vector input movement direction arrows wasd',
  MousePositionWorld: 'mouse position world cursor',
  MousePositionScreen: 'mouse position screen cursor pixels',
  MoveAndSlide: 'move and slide character body velocity walk',
  PlaySound: 'play sound audio music effect',
  StopSound: 'stop sound audio',
  FadeSound: 'fade sound volume audio',
  SetBusVolume: 'bus volume audio mixer',
  GetVariable: 'get variable read value',
  SetVariable: 'set variable write assign',
  Reroute: 'reroute dot wire pass through',
  Function: 'function define def method',
  CallFunction: 'call function invoke run',
  Return: 'return exit stop end',
  State: 'state update while in',
  StateEnter: 'state enter start begin',
  StateExit: 'state exit leave end',
  GoToState: 'go to state change switch transition',
};

/** Nome visível de um tipo de nó na busca (rótulo em português mais os nomes em inglês). */
function textoDe(def: NodeDef): string {
  return `${def.label} ${def.kind} ${def.category} ${EN[def.kind] ?? ''}`;
}

/** Filtro de busca pelo pino onde o fio foi solto (VIS-011). */
export interface PinFilter {
  ty: PinType;
  /** True quando o fio sai de uma saída: o nó novo precisa de uma entrada compatível. */
  precisaEntrada: boolean;
}

/** Um nó que casa com a busca. */
export interface Hit {
  def: NodeDef;
  score: number;
}

/** O nó tem pino compatível com o filtro? Pino sem tipo (variável, reroute) serve para qualquer tipo. */
export function compativel(def: NodeDef, f: PinFilter): boolean {
  if (def.kind === 'Reroute') return true;
  const pinos = f.precisaEntrada ? def.inputs : def.outputs;
  return pinos.some((p) => p.ty === null || p.ty === f.ty);
}

/** Nós da busca, do melhor para o pior casamento. Sem texto, devolve todos (na ordem da biblioteca). */
export function buscaNos(defs: NodeDef[], query: string, filtro: PinFilter | null): Hit[] {
  const pool = defs.filter((d) => !filtro || compativel(d, filtro));
  const q = query.trim();
  if (!q) return pool.map((def) => ({ def, score: 0 }));
  const hits: Hit[] = [];
  for (const def of pool) {
    const score = Math.max(
      fuzzyScore(q, def.label),
      fuzzyScore(q, textoDe(def)) - 40,
      fuzzyScore(q, def.description) - 200,
    );
    if (score >= 0) hits.push({ def, score });
  }
  return hits.sort((a, b) => b.score - a.score);
}

/** Nó da biblioteca pelo tipo, ou undefined. */
export function defOf(defs: NodeDef[], kind: string): NodeDef | undefined {
  return defs.find((d) => d.kind === kind);
}

/** Categorias na ordem em que aparecem no menu. */
export function categoriasDe(hits: Hit[]): { nome: string; itens: Hit[] }[] {
  const grupos = new Map<string, Hit[]>();
  for (const h of hits) {
    const lista = grupos.get(h.def.category) ?? [];
    lista.push(h);
    grupos.set(h.def.category, lista);
  }
  return [...grupos].map(([nome, itens]) => ({ nome, itens }));
}
