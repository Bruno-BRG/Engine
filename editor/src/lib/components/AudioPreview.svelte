<script lang="ts">
  // Botão de pré-visualização de um arquivo de áudio (AUD-002). O Inspector e o painel de Projeto o usam
  // com o UID do asset. Toca pelo barramento padrão, com o volume do mixer, e para sozinho ao terminar.
  import { audio } from '$lib/state/audio.svelte';
  import { assets } from '$lib/state/assets.svelte';
  import { Play, Square } from '@lucide/svelte';

  let { uid, compact = false }: { uid: string; compact?: boolean } = $props();

  const ref = $derived(assets.byUid(uid));
  const playing = $derived(audio.previewUid === uid);
  const label = $derived(playing ? 'Parar a pré-visualização' : 'Ouvir o áudio');
</script>

<button
  type="button"
  class="prev"
  class:on={playing}
  class:compact
  disabled={!ref}
  title={ref ? label : 'Arquivo não encontrado no projeto'}
  aria-label={label}
  aria-pressed={playing}
  onclick={() => audio.togglePreview(uid)}
>
  {#if playing}<Square size={11} />{:else}<Play size={11} />{/if}
  {#if !compact}<span>{playing ? 'Parar' : 'Ouvir'}</span>{/if}
</button>

<style>
  .prev {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 22px;
    padding: 0 8px;
    border-radius: var(--r-ctl);
    background: var(--bg-3);
    color: var(--text);
    font-size: var(--fs-12);
    font-weight: 500;
    transition: background var(--t-fast);
  }
  .prev:hover:not(:disabled) {
    background: var(--bg-active);
  }
  .prev.on {
    background: var(--selection);
    color: var(--sel-text);
  }
  .prev.compact {
    width: 22px;
    padding: 0;
    justify-content: center;
  }
  .prev:disabled {
    opacity: 0.5;
  }
</style>
