// Atalhos definidos pelo usuário (EDT-022) e estado do diálogo Atalhos.
// Os padrões ficam em commands.ts. Aqui só as trocas: { [commandId]: 'Ctrl+K' | null }, null = atalho removido.

const KEY = 'nexo.keymap.v1';

export type KeyOverrides = Record<string, string | null>;

function readOverrides(): KeyOverrides {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? '{}');
    const out: KeyOverrides = {};
    if (v && typeof v === 'object') {
      for (const [id, keys] of Object.entries(v)) {
        if (typeof keys === 'string' || keys === null) out[id] = keys;
      }
    }
    return out;
  } catch {
    // Sem armazenamento: vale os padrões.
    return {};
  }
}

class KeymapState {
  overrides = $state<KeyOverrides>(readOverrides());

  set(id: string, keys: string | null) {
    this.overrides[id] = keys;
    this.#save();
  }

  reset(id: string) {
    delete this.overrides[id];
    this.#save();
  }

  resetAll() {
    for (const id of Object.keys(this.overrides)) delete this.overrides[id];
    this.#save();
  }

  #save() {
    try {
      localStorage.setItem(KEY, JSON.stringify(this.overrides));
    } catch {
      // Sem armazenamento: a troca vale só nesta sessão.
    }
  }
}

export const keymap = new KeymapState();

/** Diálogo Atalhos aberto. Editor.svelte monta o diálogo enquanto estiver aberto. */
export const shortcutsOpen = $state({ open: false });
