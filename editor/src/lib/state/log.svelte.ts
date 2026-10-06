export type LogLevel = 'info' | 'success' | 'warn' | 'error';

export interface LogEntry {
  id: number;
  level: LogLevel;
  text: string;
  time: Date;
}

export interface Toast {
  id: number;
  level: LogLevel;
  title: string;
  detail?: string;
}

let seq = 0;

class Log {
  entries = $state<LogEntry[]>([]);
  toasts = $state<Toast[]>([]);

  push(level: LogLevel, text: string) {
    this.entries.push({ id: ++seq, level, text, time: new Date() });
    if (this.entries.length > 500) this.entries.splice(0, this.entries.length - 500);
  }
  info = (t: string) => this.push('info', t);
  success = (t: string) => this.push('success', t);
  warn = (t: string) => this.push('warn', t);
  error = (t: string) => this.push('error', t);

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
}

export const log = new Log();
