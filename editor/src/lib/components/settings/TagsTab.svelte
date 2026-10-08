<script lang="ts">
  import {
    projectSettings,
    editSettings,
    nameError,
    openProjectSettings,
    renameInScene,
  } from '$lib/state/settings.svelte';
  import { Plus, X } from '@lucide/svelte';

  let newTag = $state('');
  let error = $state<string | null>(null);

  function addTag(e: SubmitEvent) {
    e.preventDefault();
    const name = newTag.trim();
    error = nameError(projectSettings.tags, name);
    if (error) return;
    editSettings((s) => {
      s.tags.push(name);
    });
    newTag = '';
  }

  /** Renomeia a tag e faz os nós que a usavam acompanharem o novo nome (SCN-033). */
  function renameTag(i: number, el: HTMLInputElement) {
    const old = projectSettings.tags[i] ?? '';
    const next = el.value.trim();
    error = nameError(projectSettings.tags, next, old);
    if (error) {
      el.value = old;
      return;
    }
    editSettings((s) => {
      s.tags[i] = next;
    });
    if (old !== next) renameInScene('tag', old, next);
  }
</script>

<h3>Tags do projeto</h3>
<p class="hint">
  Tags identificam os nós para a lógica (inimigos, moedas, jogador). O campo Tag do Inspector usa esta lista.
  Renomear uma tag atualiza os nós que a usam. Remover não muda os nós: eles mantêm o nome que tinham.
</p>

{#if error}<p class="err" role="alert">{error}</p>{/if}

<ul class="tags">
  {#each projectSettings.tags as tag, i (i)}
    <li>
      <input class="txt" value={tag} aria-label="Nome da tag {i + 1}" onchange={(e) => renameTag(i, e.currentTarget)} />
      <button
        class="icon-btn"
        title="Remover tag"
        aria-label="Remover tag {tag}"
        onclick={() =>
          editSettings((s) => {
            s.tags.splice(i, 1);
          })}
      >
        <X size={13} />
      </button>
    </li>
  {:else}
    <li class="none">Nenhuma tag.</li>
  {/each}
</ul>

<form class="new" onsubmit={addTag}>
  <input class="txt" placeholder="nova_tag" aria-label="Nome da nova tag" bind:value={newTag} />
  <button class="btn" type="submit"><Plus size={13} /> Adicionar tag</button>
</form>

<h3>Camadas de colisão</h3>
<p class="hint">Os nomes das 32 camadas e a matriz de colisão ficam em Física 2D.</p>
<button class="btn" onclick={() => openProjectSettings('fisica')}>Ir para Física 2D</button>

<style>
  .tags {
    display: grid;
    gap: 4px;
    margin: 0 0 10px;
    padding: 0;
    list-style: none;
    max-width: 420px;
  }
  li {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  li .txt {
    flex: 1;
  }
  li.none {
    color: var(--text-faint);
    font-size: var(--fs-12);
  }
  .new {
    display: flex;
    gap: 6px;
    max-width: 420px;
  }
  .new .txt {
    flex: 1;
  }
  .err {
    margin: 0 0 8px;
    font-size: var(--fs-11);
    color: var(--danger);
  }
</style>
