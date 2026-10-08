<script lang="ts">
  import { onMount } from 'svelte';
  import { platform, isTauri } from '$lib/platform';
  import { TEMPLATES } from '$lib/templates';
  import * as act from '$lib/actions';
  import { X, FolderOpen, Check, LoaderCircle } from '@lucide/svelte';

  let { template, onclose }: { template: string; onclose: () => void } = $props();

  let name = $state('Meu Jogo');
  let parent = $state('');
  let selected = $state('');
  let busy = $state(false);
  let error = $state('');
  let nameInput: HTMLInputElement;

  $effect.pre(() => {
    if (!selected) selected = template;
  });

  onMount(async () => {
    parent = await platform.defaultProjectsDir();
    nameInput.focus();
    nameInput.select();
  });

  const sep = $derived(parent.includes('\\') ? '\\' : '/');
  const target = $derived(parent ? parent + sep + name.trim() : '');

  async function browse() {
    const dir = await platform.pickFolder();
    if (dir) parent = dir;
  }

  async function submit(e?: Event) {
    e?.preventDefault();
    if (busy) return;
    error = '';
    if (!name.trim()) {
      error = 'Dê um nome ao projeto.';
      return;
    }
    if (/[<>:"/\\|?*]/.test(name)) {
      error = 'O nome não pode conter < > : " / \\ | ? *';
      return;
    }
    busy = true;
    try {
      await act.createProject(name.trim(), parent, selected);
      onclose();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="backdrop" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && onclose()}>
  <form class="dialog" onsubmit={submit}>
    <header>
      <h3>Novo projeto</h3>
      <button type="button" class="icon-btn" onclick={onclose} aria-label="Fechar"><X size={16} /></button>
    </header>

    <label class="row">
      <span>Nome</span>
      <input class="field" bind:this={nameInput} bind:value={name} spellcheck="false" maxlength="64" />
    </label>

    <div class="row">
      <span>Local</span>
      <div class="loc">
        <input class="field mono" bind:value={parent} spellcheck="false" readonly={!isTauri} />
        {#if isTauri}
          <button type="button" class="btn" onclick={browse}><FolderOpen size={14} /> Escolher</button>
        {/if}
      </div>
      {#if target}<small class="mono">{target}</small>{/if}
    </div>

    <div class="row">
      <span>Modelo</span>
      <div class="choices">
        {#each TEMPLATES as t (t.id)}
          <button type="button" class="choice" class:on={selected === t.id} onclick={() => (selected = t.id)}>
            <span class="check">{#if selected === t.id}<Check size={12} strokeWidth={3} />{/if}</span>
            <strong>{t.name}</strong>
            <em>{t.description}</em>
          </button>
        {/each}
      </div>
    </div>

    {#if error}<p class="error">{error}</p>{/if}

    <footer>
      <button type="button" class="btn ghost" onclick={onclose}>Cancelar</button>
      <button type="submit" class="btn primary" disabled={busy}>
        {#if busy}<LoaderCircle size={14} class="spin" />{/if} Criar projeto
      </button>
    </footer>
  </form>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 120;
    display: grid;
    place-items: center;
    background: var(--scrim);
    backdrop-filter: blur(4px);
    animation: fade var(--t-med) var(--ease);
  }
  .dialog {
    width: min(560px, calc(100vw - 32px));
    padding: 20px 22px 18px;
    border-radius: var(--r-dlg);
    background: var(--bg-2);
    box-shadow: var(--shadow-pop);
    animation: rise 260ms var(--ease);
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h3 {
    margin: 0;
    font-size: var(--fs-18);
    font-weight: 650;
    letter-spacing: -0.015em;
  }
  .row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .row > span {
    font-size: var(--fs-12);
    font-weight: 500;
    color: var(--text-dim);
  }
  .row .field {
    height: 34px;
    font-size: var(--fs-13);
  }
  .loc {
    display: flex;
    gap: 8px;
  }
  .loc .btn {
    height: 34px;
    flex: none;
  }
  small {
    font-size: var(--fs-11);
    color: var(--text-faint);
    overflow-wrap: anywhere;
  }
  .choices {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .choice {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px 12px 10px 36px;
    text-align: left;
    border-radius: var(--r-dlg);
    border: 1px solid var(--line);
    background: var(--bg-1);
    transition: border-color var(--t-fast) var(--ease), background var(--t-fast) var(--ease);
  }
  .choice:hover {
    border-color: var(--text-faint);
  }
  .choice.on {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .check {
    position: absolute;
    left: 12px;
    top: 12px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1.5px solid var(--text-faint);
    display: grid;
    place-items: center;
    color: white;
  }
  .on .check {
    background: var(--accent);
    border-color: var(--accent);
  }
  .choice strong {
    font-weight: 600;
  }
  .choice em {
    font-style: normal;
    font-size: var(--fs-12);
    color: var(--text-dim);
  }
  .error {
    margin: 0;
    padding: 9px 12px;
    border-radius: var(--r-ctl);
    color: var(--danger);
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    font-size: var(--fs-12);
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }
  footer .btn {
    height: 34px;
    padding: 0 16px;
  }
  :global(.spin) {
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(10px) scale(0.98);
    }
  }
</style>
