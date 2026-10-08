<script lang="ts">
  import { projectSettings, editSettings, nameError } from '$lib/state/settings.svelte';
  import NumberField from '../NumberField.svelte';
  import { Plus, X } from '@lucide/svelte';

  let error = $state<string | null>(null);

  function addBus() {
    const names = projectSettings.buses.map((b) => b.name);
    let n = names.length + 1;
    while (names.includes(`Barramento ${n}`)) n++;
    error = null;
    editSettings((s) => {
      s.buses.push({ name: `Barramento ${n}`, volume_db: 0, mute: false });
    });
  }

  function renameBus(i: number, el: HTMLInputElement) {
    const current = projectSettings.buses[i]?.name ?? '';
    const err = nameError(
      projectSettings.buses.map((b) => b.name),
      el.value,
      current,
    );
    if (err) {
      error = err;
      el.value = current;
      return;
    }
    error = null;
    editSettings((s) => {
      if (s.buses[i]) s.buses[i].name = el.value.trim();
    });
  }
</script>

<h3>Barramentos de áudio</h3>
<p class="hint">
  Barramentos são os canais de áudio do projeto, cada um com volume e mudo próprios. O Master fica sempre e não
  pode ser removido. Volume em decibéis: 0 é o original, -80 é silêncio.
</p>
{#if error}<p class="err" role="alert">{error}</p>{/if}

{#if projectSettings.buses.length}
  <table class="buses">
    <thead>
      <tr>
        <th scope="col">Nome</th>
        <th scope="col">Volume</th>
        <th scope="col">Mudo</th>
        <th scope="col"><span class="sr">Remover</span></th>
      </tr>
    </thead>
    <tbody>
      {#each projectSettings.buses as bus, i (i)}
        <tr>
          <td>
            <input
              class="txt"
              value={bus.name}
              aria-label="Nome do barramento {i + 1}"
              onchange={(e) => renameBus(i, e.currentTarget)}
            />
          </td>
          <td class="vol">
            <NumberField
              label="Volume"
              value={bus.volume_db}
              step={0.5}
              precision={1}
              suffix=" dB"
              min={-80}
              max={6}
              onchange={(v) =>
                editSettings((s) => {
                  if (s.buses[i]) s.buses[i].volume_db = v;
                })}
            />
          </td>
          <td class="check">
            <input
              type="checkbox"
              checked={bus.mute}
              aria-label="Mudo: {bus.name}"
              onchange={(e) =>
                editSettings((s) => {
                  if (s.buses[i]) s.buses[i].mute = e.currentTarget.checked;
                })}
            />
          </td>
          <td>
            {#if i > 0}
              <button
                class="icon-btn"
                title="Remover barramento"
                aria-label="Remover barramento {bus.name}"
                onclick={() =>
                  editSettings((s) => {
                    s.buses.splice(i, 1);
                  })}
              >
                <X size={13} />
              </button>
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{:else}
  <p class="none">Nenhum barramento.</p>
{/if}
<button class="btn add" onclick={addBus}><Plus size={13} /> Novo barramento</button>

<style>
  .buses {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--fs-12);
  }
  th {
    padding: 0 6px 4px;
    text-align: left;
    font-weight: 600;
    font-size: var(--fs-11);
    color: var(--text-dim);
  }
  td {
    padding: 3px 6px;
    border-top: 1px solid var(--line-soft);
    vertical-align: middle;
  }
  .vol {
    width: 200px;
  }
  td.check {
    text-align: center;
  }
  td input[type='checkbox'] {
    accent-color: var(--accent);
  }
  .err {
    margin: 0 0 6px;
    font-size: var(--fs-11);
    color: var(--danger);
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
