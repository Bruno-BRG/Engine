<script lang="ts">
  // Campo que aponta para um asset do projeto pelo UID: textura (Sprite2D.texture) ou áudio
  // (AudioStreamPlayer.stream). Lista só os assets do tipo pedido (AUD-002). Para áudio, o botão ao lado
  // ouve o arquivo, com o mesmo AudioPreview do painel de Projeto.
  // Um asset arrastado do painel Projeto e solto aqui grava a mesma referência que a lista (INS-003).
  import { assets } from '$lib/state/assets.svelte';
  import { assetSlot, dragState } from '../project/drag.svelte';
  import AudioPreview from '../AudioPreview.svelte';

  let {
    label,
    kind,
    value,
    onchange,
  }: {
    label: string;
    kind: 'texture' | 'audio';
    value: string;
    onchange: (uid: string) => void;
  } = $props();

  const list = $derived(kind === 'audio' ? assets.audios : assets.textures);
  const missing = $derived(value !== '' && !assets.byUid(value));

  let pick = $state<HTMLElement>();
</script>

<div class="campo">
  <span class="flabel">{label}</span>
  <div
    class="pick"
    class:drop={pick !== undefined && dragState.overSlot === pick}
    bind:this={pick}
    use:assetSlot={{ kind, drop: (uid) => onchange(uid) }}
  >
    <select
      class="sel"
      aria-label={label}
      value={value}
      onchange={(e) => onchange(e.currentTarget.value)}
    >
      <option value="">{kind === 'audio' ? 'Nenhum áudio' : 'Nenhuma'}</option>
      {#each list as a (a.uid)}
        <option value={a.uid}>{a.name}</option>
      {/each}
      {#if missing}
        <option value={value}>{kind === 'audio' ? 'Áudio ausente' : 'Textura ausente'}</option>
      {/if}
    </select>
    {#if kind === 'audio' && value && !missing}
      <AudioPreview uid={value} compact />
    {/if}
  </div>
</div>

<style>
  .campo {
    display: grid;
    grid-template-columns: 84px minmax(0, 1fr);
    align-items: start;
    gap: 6px;
  }
  .flabel {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-11);
    color: var(--text-dim);
  }
  .pick {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }
  .sel {
    flex: 1;
    min-width: 0;
    height: 20px;
    padding: 0 4px;
    border-radius: var(--r-ctl);
    background: var(--bg-2);
    color: var(--text);
    font-size: var(--fs-12);
    box-shadow: inset 0 0 0 1px var(--line);
    outline: none;
  }
  .sel:focus {
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .pick.drop .sel {
    box-shadow: inset 0 0 0 1px var(--accent);
    background: var(--accent-soft);
  }
</style>
