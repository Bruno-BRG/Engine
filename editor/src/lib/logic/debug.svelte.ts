// Depurador da lógica (VIS-031, DBG-003). A interface guarda os pontos de parada por script, a instância que o
// Depurador acompanha e o rastro dela. A reprodução (runtime.svelte.ts) envia os pontos com `set_breakpoints`,
// escolhe a instância com `debug_select` e lê o estado (`debug_state`) e o rastro (`debug_trace`) a cada quadro.
// O rastro é por instância, com o UID do nó como chave: dois nós com o mesmo script têm os mesmos ids de grafo.

import type { TraceEvent, Value } from './model';

/** Estado de uma instância de script na reprodução, como `debug_state` a envia. */
export interface InstanciaDebug {
  /** UID do nó que roda o script. Identifica a instância. */
  uid: string;
  /** Parada por depuração: um ponto de parada, um passo, ou o fim de um quadro durante o passo a passo. */
  pausada: boolean;
  /** Nó da operação onde a execução parou. Null quando a pausa é o início de um quadro. */
  node: number | null;
  /** Evento do programa da operação onde a execução parou. Null sem pausa numa operação. */
  evento: string | null;
}

class LogicDebug {
  /** Pontos de parada por script: ids dos nós do grafo. A reprodução os envia por instância. */
  breaks = $state<Record<string, number[]>>({});
  /** Instâncias da reprodução, na ordem da árvore. Vazio fora do jogo. */
  instancias = $state.raw<InstanciaDebug[]>([]);
  /** UID da instância que o Depurador acompanha. Só ela grava o rastro. */
  selecionada = $state<string | null>(null);
  /** Whether the Depurador panel is open. The panel sets it on mount and unmount. */
  painelAberto = $state(false);
  /** Nós que rodaram no último trecho com rastro, por instância (pulsam os fios de fluxo). */
  executados = $state.raw<Record<string, number[]>>({});
  /** Último valor produzido por cada nó, por instância (aparece no hover do pino). */
  valores = $state.raw<Record<string, Record<number, Value>>>({});
  /** Avisado quando os pontos mudam: a reprodução os reenvia. Ligado pelo runtime. */
  onPontosMudaram: (() => void) | null = null;

  /** A instância escolhida, se ela existe na reprodução. */
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

  /** Registra um trecho de rastro da instância: os nós que rodaram nele e os valores que produziram. */
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

  /** Esquece o rastro e as instâncias (a reprodução parou). Os pontos de parada ficam. */
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
