<script lang="ts">
  import { slide } from 'svelte/transition';
  import { scene, BEHAVIORS, SHAPE_NAMES, type BehaviorKind, type ShapeKind } from '$lib/state/scene.svelte';
  import { app } from '$lib/state/app.svelte';
  import PanelHeader from './PanelHeader.svelte';
  import NumberField from './NumberField.svelte';
  import ShapeIcon from './ShapeIcon.svelte';
  import { ChevronDown, Eye, EyeOff, Lock, LockOpen, Plus, Trash2, Copy, Sparkles, MousePointerClick, Workflow } from '@lucide/svelte';

  const o = $derived(scene.selected);
  let open = $state({ transform: true, look: true, behaviors: true });
  let addOpen = $state(false);
  let nameText = $state('');

  $effect(() => {
    nameText = o?.name ?? '';
  });

  const SWATCHES = ['#7c5cff', '#5cc8ff', '#3ddc97', '#ffd84d', '#ffb84d', '#ff5c7a', '#ff7ad9', '#e8eaf0', '#4a5287', '#343a58'];
  const FUTURE = ['Plataforma', 'Movimento 8 direções', 'Sólido', 'Projétil', 'Câmera que segue'];

  function set(field: string, v: unknown, label: string) {
    if (!o) return;
    scene.update(o.id, { [field]: v }, label, 'field:' + o.id + ':' + field);
  }
  const end = () => scene.endGroup();

  function rename() {
    if (o && nameText.trim() && nameText.trim() !== o.name) scene.update(o.id, { name: nameText.trim() }, 'Renomear');
    else nameText = o?.name ?? '';
  }

  function setColor(hex: string) {
    if (!/^#[0-9a-f]{6}$/i.test(hex)) return;
    set('color', hex.toLowerCase(), 'Mudar cor');
  }

  function addBehavior(kind: BehaviorKind) {
    addOpen = false;
    if (!o) return;
    scene.mutate('Adicionar ' + BEHAVIORS[kind].name, (d) => {
      d.objects.find((x) => x.id === o.id)?.behaviors.push({ kind, speed: BEHAVIORS[kind].default });
    });
  }
  function setBehaviorSpeed(i: number, v: number) {
    if (!o) return;
    scene.mutate(
      'Editar behavior',
      (d) => {
        const b = d.objects.find((x) => x.id === o.id)?.behaviors[i];
        if (b) b.speed = v;
      },
      'beh:' + o.id + ':' + i,
    );
  }
  function removeBehavior(i: number) {
    if (!o) return;
    scene.mutate('Remover behavior', (d) => {
      d.objects.find((x) => x.id === o.id)?.behaviors.splice(i, 1);
    });
  }

  const available = $derived(
    (Object.keys(BEHAVIORS) as BehaviorKind[]).filter((k) => !o?.behaviors.some((b) => b.kind === k)),
  );
</script>

<PanelHeader title="Inspector">
  {#if o}
    <button class="icon-btn" title="Duplicar (Ctrl+D)" onclick={() => o && scene.duplicate(o.id)} disabled={app.playing}><Copy size={14} /></button>
    <button class="icon-btn" title="Excluir (Delete)" onclick={() => o && scene.remove(o.id)} disabled={app.playing}><Trash2 size={14} /></button>
  {/if}
</PanelHeader>

<div class="scroll">
  {#if o}
    <div class="head">
      <div class="tile" style="--c: {o.color}"><ShapeIcon shape={o.shape} color="white" size={16} /></div>
      <input class="name" bind:value={nameText} onblur={rename} onkeydown={(e) => e.key === 'Enter' && (e.currentTarget as HTMLInputElement).blur()} spellcheck="false" />
      <button class="icon-btn" class:active={!o.visible} title={o.visible ? 'Ocultar' : 'Mostrar'} onclick={() => o && scene.update(o.id, { visible: !o.visible }, 'Visibilidade')}>
        {#if o.visible}<Eye size={14} />{:else}<EyeOff size={14} />{/if}
      </button>
      <button class="icon-btn" class:active={o.locked} title={o.locked ? 'Destravar' : 'Travar no viewport'} onclick={() => o && scene.update(o.id, { locked: !o.locked }, 'Travar')}>
        {#if o.locked}<Lock size={14} />{:else}<LockOpen size={14} />{/if}
      </button>
    </div>

    <section>
      <button class="sec" onclick={() => (open.transform = !open.transform)}>
        <ChevronDown size={14} class={open.transform ? '' : 'closed'} /> Transformação
      </button>
      {#if open.transform}
        <div class="grid" transition:slide={{ duration: 180 }}>
          <NumberField label="X" value={o.x} onchange={(v) => set('x', v, 'Mover')} onend={end} />
          <NumberField label="Y" value={o.y} onchange={(v) => set('y', v, 'Mover')} onend={end} />
          <NumberField label="L" value={o.w} min={1} onchange={(v) => set('w', v, 'Redimensionar')} onend={end} />
          <NumberField label="A" value={o.h} min={1} onchange={(v) => set('h', v, 'Redimensionar')} onend={end} />
          <div class="full">
            <NumberField label="Rot" value={o.rotation} precision={1} suffix="°" onchange={(v) => set('rotation', v, 'Girar')} onend={end} />
          </div>
        </div>
      {/if}
    </section>

    <section>
      <button class="sec" onclick={() => (open.look = !open.look)}>
        <ChevronDown size={14} class={open.look ? '' : 'closed'} /> Aparência
      </button>
      {#if open.look}
        <div class="stack" transition:slide={{ duration: 180 }}>
          <div class="prop">
            <span>Forma</span>
            <div class="seg">
              {#each [0, 1, 2] as const as s}
                <button class:on={o.shape === s} title={SHAPE_NAMES[s]} onclick={() => set('shape', s as ShapeKind, 'Mudar forma')}>
                  <ShapeIcon shape={s} size={13} />
                </button>
              {/each}
            </div>
          </div>
          <div class="prop">
            <span>Cor</span>
            <div class="color">
              <label class="swatch" style="--c: {o.color}">
                <input type="color" value={o.color} oninput={(e) => setColor(e.currentTarget.value)} onchange={end} />
              </label>
              <input class="field mono hex" value={o.color.toUpperCase()} onchange={(e) => setColor(e.currentTarget.value.startsWith('#') ? e.currentTarget.value : '#' + e.currentTarget.value)} spellcheck="false" />
            </div>
          </div>
          <div class="swatches">
            {#each SWATCHES as c}
              <button style="--c: {c}" class:on={o.color === c} title={c} aria-label={c} onclick={() => { setColor(c); end(); }}></button>
            {/each}
          </div>
        </div>
      {/if}
    </section>

    <section>
      <div class="sec-row">
        <button class="sec" onclick={() => (open.behaviors = !open.behaviors)}>
          <ChevronDown size={14} class={open.behaviors ? '' : 'closed'} /> Behaviors
        </button>
        <div class="add-wrap">
          <button class="icon-btn" title="Adicionar behavior" onclick={() => (addOpen = !addOpen)}><Plus size={14} /></button>
          {#if addOpen}
            <button class="scrim" aria-label="Fechar" onclick={() => (addOpen = false)}></button>
            <div class="menu">
              {#each available as k}
                <button onclick={() => addBehavior(k)}>
                  <Sparkles size={13} />
                  <div><strong>{BEHAVIORS[k].name}</strong><em>{BEHAVIORS[k].description}</em></div>
                </button>
              {/each}
              <div class="menu-sep">Chegam na Fase 2</div>
              {#each FUTURE as f}
                <button disabled><Sparkles size={13} /><div><strong>{f}</strong></div></button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
      {#if open.behaviors}
        <div class="stack" transition:slide={{ duration: 180 }}>
          {#each o.behaviors as b, i (b.kind)}
            <div class="beh" transition:slide={{ duration: 160 }}>
              <div class="beh-head">
                <Sparkles size={13} />
                <strong>{BEHAVIORS[b.kind].name}</strong>
                <button class="icon-btn" title="Remover" onclick={() => removeBehavior(i)}><Trash2 size={13} /></button>
              </div>
              <NumberField label="Vel" value={b.speed} step={b.kind === 'girar' ? 1 : 0.05} precision={b.kind === 'girar' ? 0 : 2} suffix={BEHAVIORS[b.kind].unit} onchange={(v) => setBehaviorSpeed(i, v)} onend={end} />
            </div>
          {:else}
            <p class="hint">Behaviors são comportamentos prontos. Adicione um e aperte <strong>Jogar</strong> para ver.</p>
          {/each}
        </div>
      {/if}
    </section>

    <button class="logic" onclick={() => (app.workspace = 'logica')}>
      <Workflow size={14} /> Abrir lógica visual deste objeto
    </button>
  {:else}
    <section>
      <div class="sec static">Cena</div>
      <div class="stack">
        <div class="prop">
          <span>Nome</span>
          <input class="field" value={scene.data.name} onchange={(e) => scene.mutate('Renomear cena', (d) => (d.name = e.currentTarget.value.trim() || d.name))} />
        </div>
        <div class="prop">
          <span>Tela do jogo</span>
          <div class="grid tight">
            <NumberField label="L" value={scene.data.screen[0]} min={16} onchange={(v) => scene.mutate('Resolução', (d) => (d.screen[0] = v), 'screen-w')} onend={end} />
            <NumberField label="A" value={scene.data.screen[1]} min={16} onchange={(v) => scene.mutate('Resolução', (d) => (d.screen[1] = v), 'screen-h')} onend={end} />
          </div>
        </div>
      </div>
    </section>
    <div class="none">
      <MousePointerClick size={22} />
      <strong>Nada selecionado</strong>
      <span>Clique num objeto no viewport ou na hierarquia para editar suas propriedades.</span>
    </div>
  {/if}
</div>

<style>
  .scroll {
    flex: 1;
    overflow-y: auto;
    padding-bottom: 16px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 10px 12px 14px;
    border-bottom: 1px solid var(--line-soft);
  }
  .tile {
    flex: none;
    width: 30px;
    height: 30px;
    border-radius: 8px;
    display: grid;
    place-items: center;
    background: linear-gradient(160deg, color-mix(in srgb, var(--c) 100%, white 15%), color-mix(in srgb, var(--c) 80%, black));
    box-shadow: 0 4px 12px color-mix(in srgb, var(--c) 35%, transparent);
  }
  .name {
    flex: 1;
    min-width: 0;
    height: 30px;
    padding: 0 8px;
    border-radius: var(--r-ctl);
    border: 1px solid transparent;
    background: none;
    outline: none;
    font-size: var(--fs-15);
    font-weight: 600;
  }
  .name:hover {
    border-color: var(--line);
  }
  .name:focus {
    border-color: var(--accent);
    background: var(--bg-2);
  }
  section {
    padding: 6px 12px 10px 14px;
    border-bottom: 1px solid var(--line-soft);
  }
  .sec {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    font-size: var(--fs-12);
    font-weight: 600;
    color: var(--text-dim);
  }
  .sec.static {
    padding-left: 2px;
  }
  .sec :global(svg) {
    transition: transform var(--t-med) var(--ease);
  }
  .sec :global(svg.closed) {
    transform: rotate(-90deg);
  }
  .sec-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
    padding: 4px 0 2px;
  }
  .grid.tight {
    padding: 0;
  }
  .full {
    grid-column: 1 / -1;
  }
  .stack {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 4px 0 2px;
  }
  .prop {
    display: grid;
    grid-template-columns: 76px 1fr;
    align-items: center;
    gap: 8px;
  }
  .prop > span {
    font-size: var(--fs-12);
    color: var(--text-dim);
  }
  .seg {
    display: flex;
    padding: 2px;
    gap: 2px;
    border-radius: 7px;
    background: var(--bg-0);
    border: 1px solid var(--line-soft);
  }
  .seg button {
    flex: 1;
    height: 24px;
    display: grid;
    place-items: center;
    border-radius: 5px;
    color: var(--text-faint);
    transition: background var(--t-fast), color var(--t-fast);
  }
  .seg button:hover {
    color: var(--text);
  }
  .seg button.on {
    background: var(--bg-3);
    color: var(--accent-hi);
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.25);
  }
  .color {
    display: flex;
    gap: 6px;
  }
  .swatch {
    position: relative;
    flex: none;
    width: 28px;
    height: 28px;
    border-radius: var(--r-ctl);
    background: var(--c);
    box-shadow: 0 0 0 1px rgb(255 255 255 / 0.12) inset;
    cursor: pointer;
  }
  .swatch input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
  }
  .hex {
    font-size: var(--fs-12);
  }
  .swatches {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding-left: 84px;
  }
  .swatches button {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--c);
    box-shadow: 0 0 0 1px rgb(255 255 255 / 0.1) inset;
    transition: transform var(--t-fast) var(--ease);
  }
  .swatches button:hover {
    transform: scale(1.15);
  }
  .swatches button.on {
    box-shadow: 0 0 0 2px var(--bg-1), 0 0 0 4px var(--accent);
  }
  .add-wrap {
    position: relative;
  }
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 10;
    cursor: default;
  }
  .menu {
    position: absolute;
    right: 0;
    top: 32px;
    z-index: 11;
    width: 250px;
    padding: 5px;
    border-radius: var(--r-panel);
    background: var(--bg-2);
    box-shadow: var(--shadow-pop);
    animation: pop var(--t-med) var(--ease);
  }
  .menu button {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    width: 100%;
    padding: 7px 10px;
    border-radius: var(--r-ctl);
    text-align: left;
  }
  .menu button :global(svg) {
    margin-top: 2px;
    color: var(--accent-hi);
    flex: none;
  }
  .menu button:hover:not(:disabled) {
    background: var(--accent-soft);
  }
  .menu button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .menu button div {
    display: flex;
    flex-direction: column;
  }
  .menu strong {
    font-weight: 600;
  }
  .menu em {
    font-style: normal;
    font-size: var(--fs-12);
    color: var(--text-dim);
  }
  .menu-sep {
    margin: 6px 10px 2px;
    font-size: var(--fs-11);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-faint);
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
  .beh {
    padding: 8px;
    border-radius: 9px;
    background: var(--bg-2);
    border: 1px solid var(--line-soft);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .beh-head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-left: 2px;
  }
  .beh-head :global(svg:first-child) {
    color: var(--accent-hi);
  }
  .beh-head strong {
    flex: 1;
    font-weight: 600;
  }
  .beh-head .icon-btn {
    width: 24px;
    height: 24px;
  }
  .hint {
    margin: 0;
    font-size: var(--fs-12);
    line-height: 1.5;
    color: var(--text-faint);
  }
  .hint strong {
    color: var(--text-dim);
  }
  .logic {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    margin: 14px 12px 0 14px;
    width: calc(100% - 26px);
    height: 34px;
    border-radius: 8px;
    border: 1px dashed var(--line);
    color: var(--text-dim);
    font-weight: 500;
    transition: border-color var(--t-fast), color var(--t-fast), background var(--t-fast);
  }
  .logic:hover {
    border-color: var(--accent);
    color: var(--text);
    background: var(--accent-soft);
  }
  .none {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 32px 24px;
    text-align: center;
    color: var(--text-faint);
  }
  .none strong {
    color: var(--text-dim);
    font-weight: 600;
  }
  .none span {
    font-size: var(--fs-12);
    line-height: 1.5;
  }
</style>
