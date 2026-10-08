// Script debugger in the editor (VIS-031, DBG-003). The interface keeps the breakpoints of each script, the instance the
// debugger follows, and that instance's trace. The play session (runtime.svelte.ts) sends the breakpoints with
// `set_breakpoints`, chooses the instance with `debug_select`, and reads the state (`debug_state`) and the trace
// (`debug_trace`) only while the debugger is observing. The trace is per instance, keyed by the node UID: two nodes with
// the same script have the same graph ids.

import type { TraceEvent, Value } from './model';

/** State of a script instance in the play session, as `debug_state` sends it. */
export interface InstanciaDebug {
  /** UID of the node that runs the script. Identifies the instance. */
  uid: string;
  /** Paused by the debugger: a breakpoint, a step, or the end of a frame during step-by-step play. */
  pausada: boolean;
  /** Node of the operation where execution stopped. Null when the instance waits for the next frame. */
  node: number | null;
  /** Event of the program of the operation where execution stopped. Null without a pause at an operation. */
  evento: string | null;
}

class LogicDebug {
  /** Breakpoints per script: node ids of the graph. The play session sends them per instance. */
  breaks = $state<Record<string, number[]>>({});
  /** Instances of the play session, in tree order. Empty outside the game. */
  instancias = $state.raw<InstanciaDebug[]>([]);
  /** UID of the instance the debugger follows. Only it records the trace. */
  selecionada = $state<string | null>(null);
  /** Whether the Depurador panel is open. The panel sets it on mount and unmount. */
  painelAberto = $state(false);
  /** Nodes that ran in the last trace segment, per instance (they pulse the flow wires). */
  executados = $state.raw<Record<string, number[]>>({});
  /** Last value each node produced, per instance (shown on hover of the pin). */
  valores = $state.raw<Record<string, Record<number, Value>>>({});
  /** Called when the breakpoints change, so the play session sends them again. Set by the runtime. */
  onPontosMudaram: (() => void) | null = null;

  /** The followed instance, if it exists in the play session. */
  get escolhida(): InstanciaDebug | null {
    return this.instancias.find((i) => i.uid === this.selecionada) ?? null;
  }

  /**
   * Whether the runtime should read the debugger state each frame. It reads only while there is a reason: the panel is
   * open, a script has a breakpoint, or an instance was seen paused (so a pause is not left unseen with the panel closed).
   */
  get observando(): boolean {
    return (
      this.painelAberto ||
      this.instancias.some((i) => i.pausada) ||
      Object.values(this.breaks).some((ids) => ids.length > 0)
    );
  }

  setInstancias(lista: InstanciaDebug[]) {
    this.instancias = lista;
  }

  select(uid: string | null) {
    this.selecionada = uid;
  }

  /** Turns the breakpoints of the nodes on or off. If some node has no breakpoint yet, all of them get one. */
  toggleBreakpoints(script: string, nodes: number[]) {
    const atuais = new Set(this.breaks[script] ?? []);
    const todos = nodes.every((id) => atuais.has(id));
    for (const id of nodes) {
      if (todos) atuais.delete(id);
      else atuais.add(id);
    }
    this.breaks = { ...this.breaks, [script]: [...atuais].sort((a, b) => a - b) };
    this.onPontosMudaram?.();
  }

  breakpointsOf(script: string): number[] {
    return this.breaks[script] ?? [];
  }

  /** Records a trace segment of the instance: the nodes that ran in it and the values they produced. */
  pushTrace(uid: string, eventos: TraceEvent[]) {
    const rodaram = new Set<number>();
    const valores: Record<number, Value> = { ...(this.valores[uid] ?? {}) };
    for (const ev of eventos) {
      if ('Executou' in ev) rodaram.add(ev.Executou.node);
      else valores[ev.Valor.node] = ev.Valor.valor;
    }
    this.executados = { ...this.executados, [uid]: [...rodaram] };
    this.valores = { ...this.valores, [uid]: valores };
  }

  /** Forgets the trace and the instances (the play session stopped). The breakpoints stay. */
  limpar() {
    this.executados = {};
    this.valores = {};
    this.instancias = [];
    this.selecionada = null;
  }
}

/**
 * Whether Continuar and Próxima operação can act on the instance. They need an instance paused at an operation (a
 * breakpoint or a step), not one waiting for the next frame, and the game must be running (not paused with F6).
 */
export function podeAcionar(instancia: InstanciaDebug | null, jogoPausado: boolean): boolean {
  return instancia !== null && instancia.pausada && instancia.node !== null && !jogoPausado;
}

export const logicDebug = new LogicDebug();
