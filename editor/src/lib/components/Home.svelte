<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { platform, isTauri, type RecentProject } from '$lib/platform';
  import { TEMPLATES, templateById } from '$lib/templates';
  import * as act from '$lib/actions';
  import { app } from '$lib/state/app.svelte';
  import Logo from './Logo.svelte';
  import SceneThumb from './SceneThumb.svelte';
  import NewProjectDialog from './NewProjectDialog.svelte';
  import { Plus, FolderOpen, LayoutGrid, GraduationCap, Sparkles, Sun, Moon, X, Clock, BookOpen } from '@lucide/svelte';

  let recents = $state<RecentProject[]>([]);
  let loaded = $state(false);
  let dialogTemplate = $state<string | null>(null);

  // Miniaturas geradas uma vez a partir dos modelos.
  const thumbs = Object.fromEntries(TEMPLATES.map((t) => [t.id, t.build()]));

  const hour = new Date().getHours();
  const greeting = hour < 12 ? 'Bom dia' : hour < 18 ? 'Boa tarde' : 'Boa noite';

  onMount(async () => {
    recents = await platform.listRecent();
    loaded = true;
  });

  async function forget(e: MouseEvent, path: string) {
    e.stopPropagation();
    await platform.forgetRecent(path);
    recents = recents.filter((r) => r.path !== path);
  }

  function ago(sec: number) {
    const d = Date.now() / 1000 - sec;
    if (d < 60) return 'agora mesmo';
    if (d < 3600) return 'há ' + Math.floor(d / 60) + ' min';
    if (d < 86400) return 'há ' + Math.floor(d / 3600) + ' h';
    const days = Math.floor(d / 86400);
    return days === 1 ? 'ontem' : 'há ' + days + ' dias';
  }
</script>

<div class="home">
  <aside>
    <div class="wordmark">
      <Logo size={28} />
      <div>
        <strong>Nexo</strong>
        <span>Engine · 0.1.0</span>
      </div>
    </div>
    <nav>
      <button class="active"><LayoutGrid size={16} /> Projetos</button>
      <button disabled title="Em breve"><GraduationCap size={16} /> Aprender <em>em breve</em></button>
      <button disabled title="Em breve"><Sparkles size={16} /> Exemplos <em>em breve</em></button>
      <button disabled title="Em breve"><BookOpen size={16} /> Documentação <em>em breve</em></button>
    </nav>
    <div class="aside-foot">
      <button class="icon-btn" onclick={act.toggleTheme} title="Alternar tema">
        {#if app.theme === 'dark'}<Sun size={16} />{:else}<Moon size={16} />{/if}
      </button>
      <span>{isTauri ? 'Desktop' : 'Modo navegador'}</span>
    </div>
  </aside>

  <section class="content">
    <div class="glow"></div>
    <header in:fly={{ y: 10, duration: 400 }}>
      <div>
        <h1>{greeting}! O que vamos criar hoje?</h1>
        <p>Monte cenas, dê vida aos objetos com lógica visual e jogue na hora.</p>
      </div>
      <div class="actions">
        <button class="btn" onclick={act.pickAndOpenProject} disabled={!isTauri} title={isTauri ? '' : 'Disponível no app desktop'}>
          <FolderOpen size={15} /> Abrir projeto
        </button>
        <button class="btn primary" onclick={() => (dialogTemplate = 'plataforma')}>
          <Plus size={15} /> Novo projeto
        </button>
      </div>
    </header>

    <h2>Comece com um modelo</h2>
    <div class="templates">
      {#each TEMPLATES as t, i (t.id)}
        <button class="tcard" onclick={() => (dialogTemplate = t.id)} in:fly={{ y: 14, duration: 420, delay: 60 + i * 50 }}>
          <div class="tthumb">
            {#if t.id === 'vazio'}
              <div class="blank"><Plus size={26} /></div>
            {:else}
              <SceneThumb data={thumbs[t.id]} />
            {/if}
          </div>
          <div class="tmeta">
            <strong>{t.name}</strong>
            <span>{t.description}</span>
          </div>
        </button>
      {/each}
    </div>

    <h2>Recentes</h2>
    {#if loaded && recents.length === 0}
      <div class="empty" in:fade>
        <Clock size={20} />
        <div>
          <strong>Nenhum projeto ainda</strong>
          <span>Escolha um modelo acima para criar seu primeiro jogo.</span>
        </div>
      </div>
    {:else}
      <div class="recents">
        {#each recents as r, i (r.path)}
          <div class="rcard" role="button" tabindex="0" onclick={() => act.openProjectAt(r.path)} onkeydown={(e) => e.key === 'Enter' && act.openProjectAt(r.path)} in:fly={{ y: 10, duration: 360, delay: i * 40 }}>
            <div class="rthumb"><SceneThumb data={thumbs[templateById(r.template).id]} /></div>
            <div class="rmeta">
              <strong>{r.name}</strong>
              <span class="mono path" title={r.path}>{r.path}</span>
              <span class="when">{ago(r.openedAt)}</span>
            </div>
            <button class="forget" onclick={(e) => forget(e, r.path)} title="Remover da lista"><X size={14} /></button>
          </div>
        {/each}
      </div>
    {/if}
  </section>
</div>

{#if dialogTemplate}
  <NewProjectDialog template={dialogTemplate} onclose={() => (dialogTemplate = null)} />
{/if}

<style>
  .home {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-columns: 232px 1fr;
  }
  aside {
    display: flex;
    flex-direction: column;
    padding: 20px 14px 14px;
    background: var(--bg-1);
    border-right: 1px solid var(--line-soft);
  }
  .wordmark {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 8px 22px;
  }
  .wordmark div {
    display: flex;
    flex-direction: column;
  }
  .wordmark strong {
    font-size: var(--fs-15);
    letter-spacing: -0.01em;
  }
  .wordmark span {
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  nav button {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 34px;
    padding: 0 10px;
    border-radius: 8px;
    color: var(--text-dim);
    font-weight: 500;
  }
  nav button.active {
    background: var(--accent-soft);
    color: var(--text);
  }
  nav button.active :global(svg) {
    color: var(--accent-hi);
  }
  nav button:disabled {
    cursor: default;
    opacity: 0.6;
  }
  nav em {
    margin-left: auto;
    font-style: normal;
    font-size: 10px;
    color: var(--text-faint);
    border: 1px solid var(--line);
    padding: 1px 6px;
    border-radius: 99px;
  }
  .aside-foot {
    margin-top: auto;
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-faint);
    font-size: var(--fs-12);
  }

  .content {
    position: relative;
    overflow-y: auto;
    padding: 44px 48px 64px;
  }
  .glow {
    position: absolute;
    top: -220px;
    left: -120px;
    width: 720px;
    height: 520px;
    background: radial-gradient(closest-side, rgb(124 92 255 / 0.18), transparent);
    pointer-events: none;
  }
  header {
    position: relative;
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 24px;
    margin-bottom: 36px;
  }
  h1 {
    margin: 0 0 6px;
    font-size: 28px;
    font-weight: 650;
    letter-spacing: -0.025em;
  }
  header p {
    margin: 0;
    color: var(--text-dim);
    font-size: var(--fs-15);
  }
  .actions {
    display: flex;
    gap: 8px;
    flex: none;
  }
  .actions .btn {
    height: 36px;
    padding: 0 16px;
  }
  h2 {
    position: relative;
    margin: 0 0 14px;
    font-size: var(--fs-12);
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-faint);
  }

  .templates {
    position: relative;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 16px;
    margin-bottom: 40px;
  }
  .tcard {
    display: flex;
    flex-direction: column;
    text-align: left;
    border-radius: 12px;
    background: var(--bg-1);
    border: 1px solid var(--line-soft);
    overflow: hidden;
    box-shadow: var(--shadow-card);
    transition: transform 220ms var(--ease), border-color 220ms var(--ease), box-shadow 220ms var(--ease);
  }
  .tcard:hover {
    transform: translateY(-3px);
    border-color: var(--accent-ring);
    box-shadow: var(--shadow-card), 0 0 0 4px var(--accent-soft);
  }
  .tthumb {
    aspect-ratio: 16 / 9;
    background: #12141a;
    border-bottom: 1px solid var(--line-soft);
    overflow: hidden;
  }
  .tcard:hover .tthumb :global(svg.thumb) {
    transform: scale(1.04);
  }
  .tthumb :global(svg.thumb) {
    transition: transform 400ms var(--ease);
  }
  .blank {
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--text-faint);
    background: repeating-linear-gradient(45deg, transparent 0 10px, rgb(255 255 255 / 0.015) 10px 20px);
  }
  .tmeta {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 12px 14px 14px;
  }
  .tmeta strong {
    font-weight: 600;
  }
  .tmeta span {
    color: var(--text-dim);
    font-size: var(--fs-12);
  }

  .recents {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 10px;
  }
  .rcard {
    position: relative;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px;
    border-radius: 12px;
    background: var(--bg-1);
    border: 1px solid var(--line-soft);
    cursor: pointer;
    transition: background var(--t-fast) var(--ease), border-color var(--t-fast) var(--ease);
  }
  .rcard:hover {
    background: var(--bg-2);
    border-color: var(--line);
  }
  .rthumb {
    flex: none;
    width: 96px;
    aspect-ratio: 16/9;
    border-radius: 7px;
    overflow: hidden;
    border: 1px solid var(--line-soft);
  }
  .rmeta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .rmeta strong {
    font-weight: 600;
  }
  .path {
    font-size: var(--fs-11);
    color: var(--text-faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .when {
    font-size: var(--fs-12);
    color: var(--text-dim);
  }
  .forget {
    position: absolute;
    top: 8px;
    right: 8px;
    padding: 3px;
    border-radius: 5px;
    color: var(--text-faint);
    opacity: 0;
    transition: opacity var(--t-fast);
  }
  .rcard:hover .forget {
    opacity: 1;
  }
  .forget:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
  .empty {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 18px 20px;
    border-radius: 12px;
    border: 1px dashed var(--line);
    color: var(--text-faint);
  }
  .empty div {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .empty strong {
    color: var(--text);
    font-weight: 600;
  }
</style>
