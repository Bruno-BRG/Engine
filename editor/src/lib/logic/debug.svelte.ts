// Depurador da lógica (VIS-030, VIS-031, DBG-003). A interface guarda os pontos de parada, mostra o rastro e a
// pausa e oferece Continuar e Passo. A reprodução (o agendador do VM) é quem alimenta o rastro com `pushTrace`,
// informa a pausa com `setPausa` e liga os controles com `attach`. Sem reprodução, nada disso aparece.

import type { TraceEvent, Value } from './model';

/** Onde a execução parou: o script, o nó da op que vai rodar e o evento do programa (Pausa de scheduler.rs). */
export interface Pausa {
  script: string;
  node: number;
  evento: string;
}

/** Controles da reprodução: continuar até a próxima pausa, ou rodar uma op. */
export interface ControlesDepurador {
  continuar: () => void;
  passo: () => void;
}

class LogicDebug {
  /** Pontos de parada por script: ids dos nós. A reprodução os copia para o agendador. */
  breaks = $state<Record<string, number[]>>({});
  /** Nós que rodaram no último quadro, por script (pulsam os fios de fluxo). */
  executados = $state.raw<Record<string, number[]>>({});
  /** Último valor produzido por cada nó, por script (aparece no hover do pino). */
  valores = $state.raw<Record<string, Record<number, Value>>>({});
  /** Pausa atual, quando um ponto de parada interrompeu a execução. */
  pausa = $state<Pausa | null>(null);
  /** Controles da reprodução. Null quando não há reprodução com depurador. */
  controles = $state.raw<ControlesDepurador | null>(null);

  attach(c: ControlesDepurador | null) {
    this.controles = c;
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
  }

  breakpointsOf(script: string): number[] {
    return this.breaks[script] ?? [];
  }

  /** Registra o rastro de um quadro (ou de um intervalo) de um script. Substitui o anterior do mesmo script. */
  pushTrace(script: string, eventos: TraceEvent[]) {
    const rodaram = new Set<number>();
    const valores: Record<number, Value> = { ...(this.valores[script] ?? {}) };
    for (const ev of eventos) {
      if ('Executou' in ev) rodaram.add(ev.Executou.node);
      else valores[ev.Valor.node] = ev.Valor.valor;
    }
    this.executados = { ...this.executados, [script]: [...rodaram] };
    this.valores = { ...this.valores, [script]: valores };
  }

  setPausa(p: Pausa | null) {
    this.pausa = p;
  }

  /** Esquece o rastro e a pausa (a reprodução parou). Os pontos de parada ficam. */
  limpar() {
    this.executados = {};
    this.valores = {};
    this.pausa = null;
  }
}

export const logicDebug = new LogicDebug();
