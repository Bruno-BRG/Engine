// Console e toasts. Cada mensagem pode apontar para um objeto da cena (nodeUid), e o Console leva até ele.
import { app } from './app.svelte';

export type LogLevel = 'info' | 'success' | 'warn' | 'error';

export interface LogOpts {
  /** Uid do objeto da cena de onde veio a mensagem. Clicar duas vezes no Console seleciona o objeto. */
  nodeUid?: string;
}

export interface LogEntry {
  id: number;
  level: LogLevel;
  text: string;
  time: Date;
  nodeUid?: string;
}

export interface Toast {
  id: number;
  level: LogLevel;
  title: string;
  detail?: string;
}

/** Máximo de mensagens guardadas. As mais antigas saem primeiro. */
const MAX_ENTRIES = 1000;
const PREFS_KEY = 'nexo.console.v1';

let seq = 0;

interface ConsolePrefs {
  clearOnPlay: boolean;
  errorPause: boolean;
}

function readPrefs(): Partial<ConsolePrefs> {
  try {
    const v = JSON.parse(localStorage.getItem(PREFS_KEY) ?? '{}');
    return v && typeof v === 'object' ? v : {};
  } catch {
    // Sem armazenamento (janela privada, dados bloqueados): usa os padrões.
    return {};
  }
}

function writePrefs(p: ConsolePrefs) {
  try {
    localStorage.setItem(PREFS_KEY, JSON.stringify(p));
  } catch {
    // Sem armazenamento: a preferência vale só nesta sessão.
  }
}

class Log {
  entries = $state<LogEntry[]>([]);
  toasts = $state<Toast[]>([]);
  /** Limpa o Console ao iniciar o jogo (Console: Limpar ao jogar). */
  clearOnPlay = $state(false);
  /** Pausa o jogo quando um erro é registrado durante a execução (Console: Pausar no erro). */
  errorPause = $state(false);

  push(level: LogLevel, text: string, opts?: LogOpts) {
    this.entries.push({ id: ++seq, level, text, time: new Date(), nodeUid: opts?.nodeUid });
    if (this.entries.length > MAX_ENTRIES) this.entries.splice(0, this.entries.length - MAX_ENTRIES);
    if (level === 'error' && this.errorPause && app.playing) app.paused = true;
  }
  info = (t: string, opts?: LogOpts) => this.push('info', t, opts);
  success = (t: string, opts?: LogOpts) => this.push('success', t, opts);
  warn = (t: string, opts?: LogOpts) => this.push('warn', t, opts);
  error = (t: string, opts?: LogOpts) => this.push('error', t, opts);

  toast(level: LogLevel, title: string, detail?: string, ms = 3200) {
    const id = ++seq;
    this.toasts.push({ id, level, title, detail });
    this.push(level, detail ? title + ': ' + detail : title);
    setTimeout(() => this.dismiss(id), ms);
  }
  dismiss(id: number) {
    const i = this.toasts.findIndex((t) => t.id === id);
    if (i >= 0) this.toasts.splice(i, 1);
  }
  clear() {
    this.entries = [];
  }
  /** Chamar ao iniciar o jogo, antes da primeira mensagem de "jogo iniciado". */
  onPlayStart() {
    if (this.clearOnPlay) this.clear();
  }
}

export const log = new Log();

// Preferências do Console ficam por visualizador (localStorage). Qualquer mudança é gravada.
const prefs = readPrefs();
log.clearOnPlay = prefs.clearOnPlay === true;
log.errorPause = prefs.errorPause === true;
$effect.root(() => {
  $effect(() => {
    writePrefs({ clearOnPlay: log.clearOnPlay, errorPause: log.errorPause });
  });
});
