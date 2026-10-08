<script lang="ts">
  import {
    projectSettings,
    editSettings,
    nameError,
    renameInScene,
  } from '$lib/state/settings.svelte';
  import { LAYER_COUNT } from '$lib/settings';
  import NumberField from '../NumberField.svelte';
  import CollisionMatrix from './CollisionMatrix.svelte';
  import { Plus, X } from '@lucide/svelte';

  const indices = Array.from({ length: LAYER_COUNT }, (_, i) => i);

  let layerError = $state<string | null>(null);
  let materialError = $state<string | null>(null);

  /** Renomeia uma camada. Texto vazio libera a posição. Nós que usavam o nome antigo acompanham. */
  function renameLayer(i: number, el: HTMLInputElement) {
    const old = projectSettings.layers[i] ?? '';
    const next = el.value.trim();
    if (next === old) {
      el.value = old;
      return;
    }
    if (next && projectSettings.layers.some((n, j) => j !== i && n === next)) {
      layerError = `"${next}" já existe.`;
      el.value = old;
      return;
    }
    layerError = null;
    editSettings((s) => {
      s.layers[i] = next;
    });
    // Nós que usavam a camada liberada voltam para a camada padrão.
    if (old && old !== next) renameInScene('layer', old, next || 'default');
    el.value = next;
  }

  function addMaterial() {
    const names = projectSettings.physics_materials.map((m) => m.name);
    let n = names.length + 1;
    while (names.includes(`Material ${n}`)) n++;
    materialError = null;
    editSettings((s) => {
      s.physics_materials.push({ name: `Material ${n}`, friction: 0.5, bounce: 0, rough: false, absorbent: false });
    });
  }

  function renameMaterial(i: number, el: HTMLInputElement) {
    const current = projectSettings.physics_materials[i]?.name ?? '';
    const names = projectSettings.physics_materials.map((m) => m.name);
    const err = nameError(names, el.value, current);
    if (err) {
      materialError = err;
      el.value = current;
      return;
    }
    materialError = null;
    editSettings((s) => {
      const m = s.physics_materials[i];
      if (m) m.name = el.value.trim();
    });
  }
</script>

<h3>Gravidade</h3>
<div class="row">
  <NumberField
    label="X"
    value={projectSettings.gravity[0]}
    step={10}
    precision={0}
    suffix=" px/s²"
    onchange={(x) =>
      editSettings((s) => {
        s.gravity[0] = x;
      })}
  />
  <NumberField
    label="Y"
    value={projectSettings.gravity[1]}
    step={10}
    precision={0}
    suffix=" px/s²"
    onchange={(y) =>
      editSettings((s) => {
        s.gravity[1] = y;
      })}
  />
</div>
<p class="hint">Em pixels por segundo², com Y para baixo. O padrão é 0 e 980.</p>

<h3>Nomes das camadas de colisão</h3>
<p class="hint">
  As 32 camadas são bits. Deixe o nome vazio para liberar a posição: os nós dela voltam para a camada padrão. Renomear uma camada atualiza os nós que a usam.
</p>
{#if layerError}<p class="err" role="alert">{layerError}</p>{/if}
<div class="layers">
  {#each indices as i (i)}
    <label class="lrow">
      <span class="idx">{i}</span>
      <input
        class="txt"
        value={projectSettings.layers[i] ?? ''}
        placeholder="(livre)"
        aria-label="Nome da camada {i}"
        onchange={(e) => renameLayer(i, e.currentTarget)}
      />
    </label>
  {/each}
</div>

<h3>Matriz de colisão</h3>
<p class="hint">Marque as camadas que colidem. A matriz é simétrica: marcar A com B também marca B com A.</p>
<CollisionMatrix />

<h3>Materiais de física</h3>
<p class="hint">
  Atrito e quique de uma superfície. Áspero usa o maior atrito entre os dois corpos; absorvente usa o menor quique.
  Um corpo usa o material pelo nome.
</p>
{#if materialError}<p class="err" role="alert">{materialError}</p>{/if}
{#if projectSettings.physics_materials.length}
  <table class="mats">
    <thead>
      <tr>
        <th scope="col">Nome</th>
        <th scope="col">Atrito</th>
        <th scope="col">Quique</th>
        <th scope="col">Áspero</th>
        <th scope="col">Absorvente</th>
        <th scope="col"><span class="sr">Remover</span></th>
      </tr>
    </thead>
    <tbody>
      {#each projectSettings.physics_materials as m, i (i)}
        <tr>
          <td>
            <input class="txt" value={m.name} aria-label="Nome do material {i + 1}" onchange={(e) => renameMaterial(i, e.currentTarget)} />
          </td>
          <td>
            <NumberField
              label="Atrito"
              value={m.friction}
              step={0.05}
              precision={2}
              min={0}
              max={5}
              onchange={(v) =>
                editSettings((s) => {
                  if (s.physics_materials[i]) s.physics_materials[i].friction = v;
                })}
            />
          </td>
          <td>
            <NumberField
              label="Quique"
              value={m.bounce}
              step={0.05}
              precision={2}
              min={0}
              max={1}
              onchange={(v) =>
                editSettings((s) => {
                  if (s.physics_materials[i]) s.physics_materials[i].bounce = v;
                })}
            />
          </td>
          <td class="check">
            <input
              type="checkbox"
              checked={m.rough}
              aria-label="Áspero: {m.name}"
              onchange={(e) =>
                editSettings((s) => {
                  if (s.physics_materials[i]) s.physics_materials[i].rough = e.currentTarget.checked;
                })}
            />
          </td>
          <td class="check">
            <input
              type="checkbox"
              checked={m.absorbent}
              aria-label="Absorvente: {m.name}"
              onchange={(e) =>
                editSettings((s) => {
                  if (s.physics_materials[i]) s.physics_materials[i].absorbent = e.currentTarget.checked;
                })}
            />
          </td>
          <td>
            <button
              class="icon-btn"
              title="Remover material"
              aria-label="Remover material {m.name}"
              onclick={() =>
                editSettings((s) => {
                  s.physics_materials.splice(i, 1);
                })}
            >
              <X size={13} />
            </button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{:else}
  <p class="none">Nenhum material. Crie o primeiro abaixo.</p>
{/if}
<button class="btn add" onclick={addMaterial}><Plus size={13} /> Novo material</button>

<style>
  .row {
    display: flex;
    gap: 16px;
  }
  .row > :global(*) {
    flex: 1;
  }
  .layers {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 4px 16px;
  }
  .lrow {
    display: grid;
    grid-template-columns: 24px minmax(0, 1fr);
    align-items: center;
    gap: 6px;
  }
  .idx {
    font-family: var(--font-mono);
    font-size: var(--fs-11);
    color: var(--text-faint);
    text-align: right;
  }
  .err {
    margin: 0 0 6px;
    font-size: var(--fs-11);
    color: var(--danger);
  }
  .mats {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--fs-12);
  }
  th {
    padding: 0 6px 4px;
    text-align: left;
    font-weight: 600;
    color: var(--text-dim);
    font-size: var(--fs-11);
  }
  td {
    padding: 3px 6px;
    border-top: 1px solid var(--line-soft);
    vertical-align: middle;
  }
  td.check {
    text-align: center;
  }
  td input[type='checkbox'] {
    accent-color: var(--accent);
  }
  .none {
    margin: 0 0 8px;
    color: var(--text-faint);
    font-size: var(--fs-12);
  }
  .add {
    margin-top: 10px;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
