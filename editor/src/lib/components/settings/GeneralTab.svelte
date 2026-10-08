<script lang="ts">
  import { app } from '$lib/state/app.svelte';
  import { templateById } from '$lib/templates';

  const modelo = $derived(app.project ? templateById(app.project.template).name : '');
</script>

<h3>Projeto</h3>
<dl class="info">
  <dt>Nome</dt>
  <dd>{app.project?.name ?? ''}</dd>
  <dt>Modelo</dt>
  <dd>{modelo}</dd>
  <dt>Versão do engine</dt>
  <dd class="mono">{app.project?.engineVersion ?? ''}</dd>
  <dt>Pasta</dt>
  <dd class="mono path" title={app.project?.path}>{app.project?.path ?? ''}</dd>
  <dt>Configurações</dt>
  <dd class="mono">settings.ron, na raiz do projeto</dd>
</dl>
<p class="hint">O nome é só leitura: ele vem do arquivo do projeto.</p>

<style>
  .info {
    display: grid;
    grid-template-columns: 140px minmax(0, 1fr);
    gap: 6px 12px;
    margin: 0;
    font-size: var(--fs-12);
  }
  dt {
    color: var(--text-dim);
  }
  dd {
    margin: 0;
    min-width: 0;
    color: var(--text);
  }
  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mono {
    font-family: var(--font-mono);
    font-size: var(--fs-11);
  }
  .hint {
    margin-top: 14px;
  }
</style>
