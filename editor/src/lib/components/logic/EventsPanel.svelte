<script lang="ts">
  import { logic } from '$lib/logic/store.svelte';
  import type { GraphNode } from '$lib/logic/model';
  import { Zap, ChevronDown, ChevronRight } from '@lucide/svelte';

  // Eventos do script aberto (VIS-002). Cada evento é um nó da biblioteca com `event: true`.
  // Os que ainda não têm nó (colisão, ação, sinal, desenho, destruição, temporizador) aparecem desabilitados.
  const DISPONIVEIS: { kind: string; nome: string }[] = [
    { kind: 'OnReady', nome: 'Ao ficar pronto (On Ready)' },
    { kind: 'OnProcess', nome: 'A cada quadro (On Process)' },
    { kind: 'OnPhysicsProcess', nome: 'A cada quadro de física (On Physics Process)' },
    { kind: 'State', nome: 'Estado (State)' },
    { kind: 'StateEnter', nome: 'Ao entrar no estado (On Enter State)' },
    { kind: 'StateExit', nome: 'Ao sair do estado (On Exit State)' },
  ];
  const PENDENTES = [
    'Ao colidir com... (On Collision with)',
    'Ao receber ação (On Input Action)',
    'Ao receber sinal (On Signal)',
    'Ao desenhar (On Draw)',
    'Ao destruir (On Destroy)',
    'Ao esgotar o temporizador (On Timer)',
  ];

  let aberto = $state(false);
  let recolhido = $state(false);

  function adiciona(kind: string) {
    aberto = false;
    logic.addEvent(kind);
  }

  /** Eventos de estado guardam o nome do estado no pino "state". */
  function detalhe(kind: string, valores: Record<string, unknown>): string {
    if (!kind.startsWith('State')) return '';
    const v = valores.state as { String?: string } | undefined;
    return v?.String ? `estado “${v.String}”` : 'sem nome de estado';
  }

  function temFluxo(id: number): boolean {
    return logic.snap.wires.some((w) => w.from.node === id);
  }

  /** Linha de apoio do evento: o nome do estado (se for de estado) e se ele tem fluxo ligado. */
  function subtitulo(n: GraphNode): string {
    const partes: string[] = [];
    const d = detalhe(n.kind, n.values);
    if (d) partes.push(d);
    if (!temFluxo(n.id)) partes.push('sem fluxo ligado');
    return partes.join(' · ');
  }
</script>

<section class="painel">
  <header>
    <button class="titulo" onclick={() => (recolhido = !recolhido)} aria-expanded={!recolhido}>
      {#if recolhido}<ChevronRight size={12} />{:else}<ChevronDown size={12} />{/if}
      <span>Eventos</span>
    </button>
    <span class="contagem mono">{logic.events.length}</span>
  </header>

  {#if !recolhido}
    <div class="corpo">
      {#if logic.active}
        <button class="add" aria-expanded={aberto} onclick={() => (aberto = !aberto)}>
          <Zap size={12} /> Adicionar evento
        </button>
        {#if aberto}
          <ul class="menu" aria-label="Tipos de evento">
            {#each DISPONIVEIS as ev (ev.kind)}
              <li>
                <button onclick={() => adiciona(ev.kind)}>{ev.nome}</button>
              </li>
            {/each}
            {#each PENDENTES as nome (nome)}
              <li>
                <button disabled title="Ainda sem nó na biblioteca de lógica">{nome}</button>
              </li>
            {/each}
          </ul>
        {/if}

        <ul class="lista">
          {#each logic.events as n (n.id)}
            {@const selecionado = logic.selNodes.includes(n.id)}
            <li>
              <button
                class="ev"
                class:sel={selecionado}
                onclick={() => {
                  logic.select([n.id], [], false);
                  logic.frameSelection();
                }}
              >
                <Zap size={11} />
                <span class="nome">{logic.labelOf(n)}</span>
                <span class="sub">{subtitulo(n)}</span>
              </button>
            </li>
          {:else}
            <li class="vazio">Nenhum evento. Um script com eventos roda no jogo.</li>
          {/each}
        </ul>
      {:else}
        <p class="vazio">Abra um script para ver os eventos dele.</p>
      {/if}
    </div>
  {/if}
</section>

<style>
  .painel {
    display: flex;
    flex-direction: column;
    border-bottom: 1px solid var(--line);
    min-height: 0;
  }
  header {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 22px;
    padding: 0 6px 0 4px;
    background: var(--bg-toolbar);
    border-bottom: 1px solid var(--line);
  }
  .titulo {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: var(--fs-12);
    color: var(--text);
  }
  .contagem {
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .corpo {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 6px;
  }
  .add {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 8px;
    border-radius: var(--r-ctl);
    background: var(--bg-3);
    border: 1px solid var(--line);
    font-size: var(--fs-12);
    color: var(--text);
  }
  .add:hover {
    background: var(--bg-active);
  }
  .menu {
    list-style: none;
    margin: 0;
    padding: 4px;
    border-radius: var(--r-ctl);
    background: var(--bg-2);
    border: 1px solid var(--line);
    display: flex;
    flex-direction: column;
  }
  .menu button {
    width: 100%;
    height: 22px;
    padding: 0 6px;
    text-align: left;
    font-size: var(--fs-12);
    color: var(--text);
    border-radius: var(--r-ctl);
  }
  .menu button:hover:not(:disabled) {
    background: var(--accent-soft);
  }
  .menu button:disabled {
    color: var(--text-faint);
    cursor: default;
  }
  .lista {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .ev {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    min-height: 24px;
    padding: 3px 6px;
    border-radius: var(--r-ctl);
    font-size: var(--fs-12);
    color: var(--text);
    text-align: left;
    flex-wrap: wrap;
  }
  .ev:hover {
    background: var(--bg-hover);
  }
  .ev.sel {
    background: var(--accent-soft);
  }
  .nome {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    width: 100%;
    padding-left: 17px;
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .vazio {
    font-size: var(--fs-12);
    color: var(--text-faint);
    line-height: 1.5;
  }
</style>
