// Tarefas em segundo plano (importação, recarga de assets). Mostradas na barra de status (EDT-013).

export type TaskStatus = 'running' | 'done' | 'error';

export interface Task {
  id: number;
  label: string;
  /** 0 a 1, ou null quando o tamanho do trabalho é desconhecido. */
  progress: number | null;
  status: TaskStatus;
  message: string;
}

export interface TaskHandle {
  progress(p: number | null): void;
  done(message?: string): void;
  fail(message: string): void;
}

class TaskState {
  list = $state<Task[]>([]);
  #next = 1;

  /** Tarefas em execução, da mais antiga para a mais nova. */
  running = $derived(this.list.filter((t) => t.status === 'running'));

  start(label: string): TaskHandle {
    const id = this.#next++;
    this.list.push({ id, label, progress: null, status: 'running', message: '' });
    const find = () => this.list.find((t) => t.id === id);
    const finish = (status: TaskStatus, message: string) => {
      const t = find();
      if (!t) return;
      t.status = status;
      t.message = message;
      t.progress = status === 'done' ? 1 : t.progress;
      // Tarefas concluídas somem da lista depois de um tempo, para não poluir a barra.
      setTimeout(() => {
        this.list = this.list.filter((x) => x.id !== id);
      }, status === 'error' ? 8000 : 2500);
    };
    return {
      progress: (p) => {
        const t = find();
        if (t) t.progress = p == null ? null : Math.min(1, Math.max(0, p));
      },
      done: (message = '') => finish('done', message),
      fail: (message) => finish('error', message),
    };
  }
}

export const tasks = new TaskState();
