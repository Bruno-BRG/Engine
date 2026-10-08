<script lang="ts">
  import { app } from '$lib/state/app.svelte';
  import { log } from '$lib/state/log.svelte';
  import { COMMANDS, keysOf } from '$lib/commands';
  import { isTauri } from '$lib/platform';
  import { Check } from '@lucide/svelte';

  // Os itens vêm da lista de comandos: menu, paleta e atalhos ficam sempre iguais.
  type Item =
    | { label: string; keys?: string; enabled: () => boolean; checked?: () => boolean; run: () => void }
    | 'sep';

  const find = (id: string) => {
    const c = COMMANDS.find((x) => x.id === id);
    if (!c) throw new Error('Comando desconhecido: ' + id);
    return c;
  };
  const cmd = (id: string, label?: string, checked?: () => boolean): Item => {
    const c = find(id);
    return {
      label: label ?? c.title,
      // Atalho em uso (com as trocas do usuário), não o padrão fixo.
      keys: keysOf(c),
      enabled: () => (c.enabled ? c.enabled() : true),
      checked,
      run: c.run,
    };
  };

  async function openFolder(sub = '') {
    if (!app.project || !isTauri) return;
    const { openPath } = await import('@tauri-apps/plugin-opener');
    openPath(app.project.path + sub);
  }

  const menus: { id: string; label: string; items: () => Item[] }[] = [
    {
      id: 'arquivo',
      label: 'Arquivo',
      items: () => [
        cmd('save', 'Salvar cena'),
        { label: 'Abrir pasta do projeto', enabled: () => isTauri && app.project != null, run: () => openFolder() },
        'sep',
        // Exportação chega na Fase 3 (EXP); o item fica visível para a estrutura do menu ficar completa.
        { label: 'Compilar… (Fase 3)', enabled: () => false, run: () => {} },
        'sep',
        cmd('close', 'Fechar projeto'),
      ],
    },
    {
      id: 'editar',
      label: 'Editar',
      items: () => [
        cmd('undo'),
        cmd('redo'),
        'sep',
        cmd('cut', 'Recortar'),
        cmd('copy', 'Copiar'),
        cmd('paste', 'Colar'),
        cmd('duplicate', 'Duplicar'),
        cmd('delete', 'Excluir'),
        'sep',
        cmd('deselect', 'Limpar seleção'),
        'sep',
        cmd('shortcuts', 'Atalhos…'),
      ],
    },
    {
      id: 'node',
      label: 'Node',
      items: () => [cmd('add-rect', 'Retângulo'), cmd('add-circle', 'Círculo'), cmd('add-diamond', 'Losango')],
    },
    {
      id: 'assets',
      label: 'Assets',
      items: () => [
        cmd('tab-assets', 'Mostrar painel Assets'),
        { label: 'Abrir pasta de assets', enabled: () => isTauri && app.project != null, run: () => openFolder('/assets') },
      ],
    },
    {
      id: 'janela',
      label: 'Janela',
      items: () => [
        cmd('panel-left', 'Hierarquia', () => app.panels.left),
        cmd('panel-right', 'Inspector', () => app.panels.right),
        cmd('panel-bottom', 'Painel inferior', () => app.panels.bottom),
        'sep',
        cmd('ws-cena', 'Cena', () => app.workspace === 'cena'),
        cmd('ws-logica', 'Lógica', () => app.workspace === 'logica'),
        cmd('ws-animacao', 'Animação', () => app.workspace === 'animacao'),
        cmd('ws-sprite', 'Sprite', () => app.workspace === 'sprite'),
        'sep',
        cmd('tab-console', 'Mostrar Console'),
        cmd('tab-debug', 'Mostrar Depurador'),
        cmd('tab-csharp', 'Mostrar C#'),
        'sep',
        cmd('theme', 'Alternar tema claro/escuro'),
      ],
    },
    {
      id: 'ajuda',
      label: 'Ajuda',
      items: () => [
        cmd('palette', 'Paleta de comandos'),
        'sep',
        {
          label: 'Sobre o Nexo',
          enabled: () => true,
          run: () => log.toast('info', 'Nexo 0.1.0', 'Motor em Rust, editor em Tauri e Svelte, viewport em WebGPU'),
        },
      ],
    },
  ];

  let open = $state<string | null>(null);

  function pick(item: Extract<Item, object>) {
    open = null;
    item.run();
  }

</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && (open = null)} />

<nav class="menubar" aria-label="Menu principal">
  {#each menus as m (m.id)}
    <div class="menu-wrap">
      <button
        class="top"
        class:open={open === m.id}
        aria-haspopup="menu"
        aria-expanded={open === m.id}
        onclick={() => (open = open === m.id ? null : m.id)}
        onmouseenter={() => open && (open = m.id)}
      >
        {m.label}
      </button>
      {#if open === m.id}
        <button class="scrim" aria-label="Fechar menu" onclick={() => (open = null)}></button>
        <div class="dropdown" role="menu">
          {#each m.items() as it, i (i)}
            {#if it === 'sep'}
              <div class="sep" role="separator"></div>
            {:else}
              {@const checked = it.checked?.() ?? false}
              <button
                role={it.checked ? 'menuitemcheckbox' : 'menuitem'}
                aria-checked={it.checked ? checked : undefined}
                disabled={!it.enabled()}
                onclick={() => pick(it)}
              >
                <span class="mark">{#if checked}<Check size={12} />{/if}</span>
                <span class="label">{it.label}</span>
                {#if it.keys}<kbd>{it.keys}</kbd>{/if}
              </button>
            {/if}
          {/each}
        </div>
      {/if}
    </div>
  {/each}
</nav>

<style>
  .menubar {
    display: flex;
    align-items: center;
    height: 100%;
  }
  .menu-wrap {
    position: relative;
    height: 100%;
  }
  .top {
    height: 22px;
    padding: 0 8px;
    border-radius: var(--r-ctl);
    color: var(--text);
    transition: background var(--t-fast) var(--ease);
  }
  .top:hover,
  .top.open {
    background: var(--bg-hover);
  }
  .top.open {
    background: var(--bg-active);
  }
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 1;
    cursor: default;
  }
  .dropdown {
    position: absolute;
    top: 26px;
    left: 0;
    z-index: 2;
    min-width: 240px;
    padding: 3px;
    background: var(--bg-2);
    border-radius: var(--r-panel);
    box-shadow: var(--shadow-pop);
    animation: pop var(--t-med) var(--ease);
  }
  .dropdown button {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    height: 24px;
    padding: 0 8px 0 4px;
    border-radius: var(--r-ctl);
    color: var(--text);
    text-align: left;
  }
  .dropdown button:hover:not(:disabled) {
    background: var(--selection);
    color: var(--sel-text);
  }
  .dropdown button:disabled {
    color: var(--text-faint);
    cursor: default;
  }
  .mark {
    flex: none;
    width: 14px;
    display: grid;
    place-items: center;
  }
  .label {
    flex: 1;
    white-space: nowrap;
  }
  .dropdown kbd {
    margin-left: 16px;
  }
  .sep {
    height: 1px;
    margin: 3px 4px;
    background: var(--line);
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.98);
    }
  }
</style>
