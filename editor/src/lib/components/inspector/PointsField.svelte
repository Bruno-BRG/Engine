<script lang="ts">
  // Lista editável de pontos de um CollisionPolygon2D (PHY-011): cada ponto é um par x/y, no espaço do nó.
  // O núcleo valida o formato (PropKind::Points, em scene.rs). Aqui só se edita a lista. Edições de número
  // juntam-se num passo de desfazer; adicionar e remover fazem passo próprio.
  import NumberField from '../NumberField.svelte';
  import { Plus, X } from '@lucide/svelte';

  type Point = [number, number];

  let {
    label,
    points,
    onset,
    onend,
    purpose = 'colisao',
  }: {
    label: string;
    points: Point[];
    /** Grava a lista inteira. `coalesce` junta a alteração com a anterior no desfazer. */
    onset: (points: Point[], coalesce: boolean) => void;
    onend: () => void;
    /** What the list is for: a collision polygon needs three points, a path needs two (VPT-005). */
    purpose?: 'colisao' | 'caminho';
  } = $props();

  const minimo = $derived(purpose === 'caminho' ? 2 : 3);
  const semPontos = $derived(
    purpose === 'caminho'
      ? 'Sem pontos. Adicione pelo menos dois para formar o caminho.'
      : 'Sem pontos. Adicione pelo menos três para formar a colisão.',
  );
  const poucosPontos = $derived(
    purpose === 'caminho'
      ? 'Com menos de dois pontos, o caminho não aparece.'
      : 'Com menos de três pontos, o polígono não colide.',
  );

  function edit(i: number, axis: 0 | 1, v: number) {
    const next = points.map((p, j): Point => {
      if (j !== i) return p;
      return axis === 0 ? [v, p[1]] : [p[0], v];
    });
    onset(next, true);
  }

  /**
   * Ponto novo. O primeiro vai a 16 px do zero; os seguintes giram a última aresta em 90°, para a forma
   * fechar em volta (triângulo, quadrado). Assim o padrão não sai em linha reta, que não colide.
   */
  function add() {
    const n = points.length;
    let next: Point = [0, 0];
    if (n === 1) {
      next = [points[0][0] + 16, points[0][1]];
    } else if (n >= 2) {
      const [ax, ay] = points[n - 2];
      const [bx, by] = points[n - 1];
      const dx = bx - ax;
      const dy = by - ay;
      next = dx === 0 && dy === 0 ? [bx + 16, by] : [bx - dy, by + dx];
    }
    onset([...points, next], false);
  }

  function remove(i: number) {
    onset(
      points.filter((_, j) => j !== i),
      false,
    );
  }
</script>

<div class="pts">
  <div class="head">
    <span class="flabel">{label}</span>
    <span class="count">{points.length} {points.length === 1 ? 'ponto' : 'pontos'}</span>
    <button class="mini" type="button" onclick={add}><Plus size={11} /> Ponto</button>
  </div>

  {#each points as p, i (i)}
    <div class="row">
      <span class="idx">{i + 1}</span>
      <NumberField label="X" value={p[0]} step={1} precision={0} onchange={(x) => edit(i, 0, x)} onend={onend} />
      <NumberField label="Y" value={p[1]} step={1} precision={0} onchange={(y) => edit(i, 1, y)} onend={onend} />
      <button class="rm" type="button" title="Remover ponto" aria-label="Remover ponto {i + 1}" onclick={() => remove(i)}>
        <X size={11} />
      </button>
    </div>
  {:else}
    <p class="none">{semPontos}</p>
  {/each}

  {#if points.length > 0 && points.length < minimo}
    <p class="aviso">{poucosPontos}</p>
  {/if}
</div>

<style>
  .pts {
    display: grid;
    gap: 4px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .flabel {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-11);
    color: var(--text-dim);
  }
  .count {
    font-size: var(--fs-11);
    color: var(--text-faint);
    white-space: nowrap;
  }
  .mini {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 20px;
    padding: 0 6px;
    border-radius: var(--r-ctl);
    background: var(--bg-3);
    color: var(--text);
    font-size: var(--fs-11);
  }
  .mini:hover {
    background: var(--bg-active);
  }
  .row {
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr) minmax(0, 1fr) 20px;
    align-items: center;
    gap: 4px;
  }
  .idx {
    font-family: var(--font-mono);
    font-size: var(--fs-11);
    color: var(--text-faint);
    text-align: right;
  }
  .rm {
    width: 20px;
    height: 18px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--r-ctl);
    color: var(--text-faint);
  }
  .rm:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
  .none,
  .aviso {
    margin: 0;
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .aviso {
    color: var(--warning);
  }
</style>
