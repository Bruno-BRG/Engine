// Script debugger in the editor (VIS-031, DBG-003). The interface keeps the breakpoints of each script, the instance the
// debugger follows, and that instance's trace. The play session (runtime.svelte.ts) sends the breakpoints with
// `set_breakpoints`, chooses the instance with `debug_select`, and reads the state (`debug_state`) and the trace
// (`debug_trace`) only while the debugger is observing.
// The trace is per instance, keyed by the node UID: two nodes with the same script have the same graph ids.

import type { Trace, Value } from './model';

/** How long a node or a flow wire stays lit after it ran, in ms (VIS-030). */
export const LIT_MS = 300;

/** Key of a flow wire (origin node, output pin, destination node), the same in the trace and on the canvas. */
export function chaveDoFio(de: number, pino: string, para: number): string {
  return `${de}|${pino}|${para}`;
}

/** Whether two lists hold the same items in the same order. */
function mesmaLista<T>(a: readonly T[], b: readonly T[]): boolean {
  return a.length === b.length && a.every((x, i) => x === b[i]);
}

/**
 * One frame of the call stack of a paused operation (DBG-003), as `debug_state` sends it. The list goes from the event
 * (the first frame) to the innermost frame. `node` is the node that opened the frame: the event node, the CallFunction
 * node, or the TimerStart node.
 */
export type Frame =
  | { kind: 'event'; node: number; name: string }
  | { kind: 'function'; node: number; name: string }
  | { kind: 'timer'; node: number };

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
  /** Call stack of the operation where execution stopped, from the event to the innermost frame.
   * Empty without a pause. */
  stack: Frame[];
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
  /** Nodes that ran in the last trace segment that had activity, per instance (the panel counts them). */
  executados = $state.raw<Record<string, number[]>>({});
  /** Nodes and flow wires of the followed instance that ran in the last LIT_MS: what lights up on the canvas. */
  lit = $state.raw<{ uid: string | null; nodes: number[]; wires: string[] }>({ uid: null, nodes: [], wires: [] });
  /** Last value each node produced, per instance (shown on hover of the pin). */
  valores = $state.raw<Record<string, Record<number, Value>>>({});
  /** Time (ms) of the last run of each lit node ("n<id>") and wire ("w<key>"). Not reactive: the canvas reads `lit`. */
  #ultimaExecucao = new Map<string, number>();
  /** Instance that `#ultimaExecucao` belongs to. */
  #uidLit: string | null = null;
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

  /** Liga ou desliga os pontos de parada dos nós. Se algum nó ainda não tem ponto, todos ganham. */
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

  /**
   * Records one read of the trace of the followed instance (VIS-030). `trace` is null when there was nothing to read.
   * `agora` is the read time in ms. Nodes and wires stay lit for LIT_MS after they ran, so the highlight also ages
   * on reads that bring nothing new. Each state is assigned only when it changes, so the canvas does not update on
   * every read.
   */
  pushTrace(uid: string, trace: Trace | null, agora: number) {
    if (this.#uidLit !== uid) {
      this.#uidLit = uid;
      this.#ultimaExecucao.clear();
    }
    if (trace) {
      if (trace.nodes.length > 0 && !mesmaLista(this.executados[uid] ?? [], trace.nodes)) {
        this.executados = { ...this.executados, [uid]: trace.nodes };
      }
      for (const no of trace.nodes) this.#ultimaExecucao.set(`n${no}`, agora);
      for (const fio of trace.wires) this.#ultimaExecucao.set(`w${chaveDoFio(fio.from, fio.pin, fio.to)}`, agora);
      this.#atualizaValores(uid, trace.values);
    }
    // Ages the highlight: what ran LIT_MS or more ago goes dark.
    for (const [chave, quando] of this.#ultimaExecucao) {
      if (agora - quando >= LIT_MS) this.#ultimaExecucao.delete(chave);
    }
    const nodes: number[] = [];
    const wires: string[] = [];
    for (const chave of this.#ultimaExecucao.keys()) {
      if (chave.startsWith('n')) nodes.push(Number(chave.slice(1)));
      else wires.push(chave.slice(1));
    }
    if (this.lit.uid !== uid || !mesmaLista(this.lit.nodes, nodes) || !mesmaLista(this.lit.wires, wires)) {
      this.lit = { uid, nodes, wires };
    }
  }

  /** Keeps the last value of each node that the trace reports as changed. */
  #atualizaValores(uid: string, novos: Trace['values']) {
    if (novos.length === 0) return;
    const proximos: Record<number, Value> = { ...(this.valores[uid] ?? {}) };
    let mudou = false;
    for (const { node, value } of novos) {
      if (JSON.stringify(proximos[node]) !== JSON.stringify(value)) {
        proximos[node] = value;
        mudou = true;
      }
    }
    if (mudou) this.valores = { ...this.valores, [uid]: proximos };
  }

  /** Turns the live highlight off, when the debugger stops observing the play session. */
  apagarRealce() {
    this.#ultimaExecucao.clear();
    this.#uidLit = null;
    if (this.lit.uid !== null || this.lit.nodes.length > 0 || this.lit.wires.length > 0) {
      this.lit = { uid: null, nodes: [], wires: [] };
    }
  }

  /** Forgets the trace and the instances (the play session stopped). The breakpoints stay. */
  limpar() {
    this.executados = {};
    this.valores = {};
    this.apagarRealce();
    this.instancias = [];
    this.selecionada = null;
  }
}

/**
 * Whether Continuar and Next operation can act on the instance. They need an instance paused at an operation (a
 * breakpoint or a step), not one waiting for the next frame, and the game must be running (not paused with F6).
 */
export function podeAcionar(instancia: InstanciaDebug | null, jogoPausado: boolean): boolean {
  return instancia !== null && instancia.pausada && instancia.node !== null && !jogoPausado;
}

export const logicDebug = new LogicDebug();
