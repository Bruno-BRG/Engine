<script lang="ts">
  // Matriz de colisão 32x32 (PHY-021): a linha é a camada, a coluna é a camada que ela enfrenta.
  // Marcar (A, B) também marca (B, A), para a matriz ficar simétrica.
  import { projectSettings, editSettings } from '$lib/state/settings.svelte';
  import { LAYER_COUNT, hasBit, setCollision } from '$lib/settings';

  const indices = Array.from({ length: LAYER_COUNT }, (_, i) => i);

  const name = (i: number) => projectSettings.layers[i] ?? '';
  const label = (i: number) => name(i) || String(i);

  function toggle(a: number, b: number, on: boolean) {
    editSettings((s) => setCollision(s, a, b, on));
  }
</script>

<div class="matrix-wrap">
  <table class="matrix">
    <thead>
      <tr>
        <th class="corner" scope="col"><span class="sr">Camada</span></th>
        {#each indices as b (b)}
          <th scope="col" class="col" class:free={!name(b)} title={label(b)}>
            <span>{label(b)}</span>
          </th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#each indices as a (a)}
        <tr class:free={!name(a)}>
          <th scope="row" class="row-head" title={label(a)}>{label(a)}</th>
          {#each indices as b (b)}
            <td>
              <input
                type="checkbox"
                aria-label="{label(a)} colide com {label(b)}"
                disabled={!name(a) || !name(b)}
                checked={hasBit(projectSettings.layer_matrix[a] ?? 0, b)}
                onchange={(e) => toggle(a, b, e.currentTarget.checked)}
              />
            </td>
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .matrix-wrap {
    overflow: auto;
    max-height: 420px;
    border: 1px solid var(--line-soft);
    border-radius: var(--r-ctl);
  }
  .matrix {
    border-collapse: collapse;
    font-size: var(--fs-11);
  }
  th,
  td {
    padding: 0;
    text-align: center;
  }
  .col {
    position: sticky;
    top: 0;
    z-index: 1;
    height: 84px;
    width: 20px;
    min-width: 20px;
    background: var(--bg-toolbar);
    font-weight: 500;
    color: var(--text);
    vertical-align: bottom;
  }
  .col span {
    display: inline-block;
    writing-mode: vertical-rl;
    transform: rotate(180deg);
    max-height: 78px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 2px 0;
  }
  .row-head {
    position: sticky;
    left: 0;
    z-index: 1;
    min-width: 96px;
    max-width: 96px;
    height: 20px;
    padding: 0 6px;
    background: var(--bg-toolbar);
    text-align: left;
    font-weight: 500;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .corner {
    position: sticky;
    top: 0;
    left: 0;
    z-index: 2;
    background: var(--bg-toolbar);
    min-width: 96px;
  }
  td {
    width: 20px;
    height: 20px;
    border: 1px solid var(--line-soft);
  }
  td input {
    margin: 0;
    width: 13px;
    height: 13px;
    accent-color: var(--accent);
    cursor: pointer;
  }
  td input:disabled {
    opacity: 0.25;
    cursor: default;
  }
  tr.free .row-head,
  .col.free {
    color: var(--text-faint);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
