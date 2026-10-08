<script lang="ts">
  import { scene, type NodeType, type PropDef } from '$lib/state/scene.svelte';
  import { focus, focusPanel } from '$lib/state/focus.svelte';
  import { showContextMenu } from '$lib/state/contextmenu.svelte';
  import { textures } from '$lib/state/textures.svelte';
  import { projectSettings } from '$lib/state/settings.svelte';
  import { namesOf } from '$lib/settings';
  import { OBJECT_PALETTE } from '$lib/palette';
  import PanelHeader from './PanelHeader.svelte';
  import NodeIcon from './NodeIcon.svelte';
  import NumberField from './NumberField.svelte';
  import LayerMaskField from './settings/LayerMaskField.svelte';
  import NameField from './inspector/NameField.svelte';
  import PointsField from './inspector/PointsField.svelte';
  import AssetField from './inspector/AssetField.svelte';
  import NodeRefField from './inspector/NodeRefField.svelte';
  import RangeField from './inspector/RangeField.svelte';
  import ScriptSlot from './logic/ScriptSlot.svelte';
  import { ChevronDown, ChevronRight, Ellipsis, Eye, EyeOff, Lock, LockOpen, Plus, Search, Braces } from '@lucide/svelte';

  const norm = (s: string) => s.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase();

  /** Rótulos em português para os valores de enumeração conhecidos. */
  const ENUM_LABELS: Record<string, Record<string, string>> = {
    shape: { rect: 'Retângulo', circle: 'Círculo', diamond: 'Losango' },
    kind: { spin: 'Girar', float: 'Flutuar', pulse: 'Pulsar' },
    tag: { untagged: 'Sem tag', player: 'Jogador', enemy: 'Inimigo', coin: 'Moeda' },
    layer: { default: 'Padrão', ground: 'Chão', player: 'Jogador', enemy: 'Inimigo' },
  };
  const node = $derived(scene.selected);
  const multi = $derived(scene.selection.length > 1);

  // Nomes válidos para Tag e Layer (SCN-033): vêm das Configurações do Projeto.
  const tagNames = $derived(namesOf(projectSettings, 'tags'));
  const layerNames = $derived(namesOf(projectSettings, 'layers'));

  /** Tipos da cadeia de herança do nó, do mais específico ao `Node` (INS-002). */
  const chain = $derived.by<NodeType[]>(() => {
    const out: NodeType[] = [];
    let t = node ? scene.types.find((x) => x.name === node.kind) : undefined;
    while (t) {
      out.push(t);
      t = t.base ? scene.types.find((x) => x.name === t!.base) : undefined;
    }
    return out;
  });

  let collapsed = $state<Record<string, boolean>>({});
  let addOpen = $state(false);
  let addQuery = $state('');
  let nameDraft = $state('');
  let nameFor = $state<string | null>(null);

  // O nome é editado num campo próprio; sincroniza quando o nó selecionado muda.
  $effect(() => {
    if (node && node.uid !== nameFor) {
      nameFor = node.uid;
      nameDraft = node.name;
    }
  });

  function commitName() {
    if (node && nameDraft.trim() && nameDraft.trim() !== node.name) scene.rename(node.uid, nameDraft.trim());
    else if (node) nameDraft = node.name;
  }

  function setProp(def: PropDef, value: unknown, coalesce = false) {
    if (!node) return;
    scene.setProp(node.uid, def.name, value, coalesce ? 'insp:' + node.uid + ':' + def.name : undefined);
  }

  function endEdit() {
    scene.endGroup();
  }

  function value(def: PropDef) {
    return node?.props[def.name] ?? def.default;
  }

  /** Casas decimais para exibir um número, conforme o passo do campo. */
  const decimals = (step: number | null) => (step == null || step >= 1 ? 0 : step >= 0.1 ? 1 : 2);

  function enumLabel(def: PropDef, v: string) {
    return ENUM_LABELS[def.name]?.[v] ?? v;
  }

  function restoreSection(t: NodeType) {
    if (!node) return;
    const patch: Record<string, unknown> = {};
    // Script and exported-value maps (VIS-022) are edited in the script slot, so restoring the section leaves them alone.
    for (const p of t.props) if (p.name !== 'script' && p.kind !== 'map') patch[p.name] = p.default;
    scene.setProps(node.uid, patch);
  }

  function sectionMenu(e: MouseEvent, t: NodeType) {
    e.stopPropagation();
    showContextMenu(e, [
      { label: 'Restaurar valores', run: () => restoreSection(t) },
      { label: 'Copiar caminho da propriedade', enabled: false, run: () => {} },
      'sep',
      { label: 'Documentação do tipo', enabled: false, run: () => {} },
    ]);
  }

  /** Escolher uma textura também ajusta o tamanho do sprite às dimensões da imagem. */
  function pickTexture(def: PropDef, uid: string) {
    setProp(def, uid);
    if (!uid || !node) return;
    const d = textures.dims(uid);
    if (d) scene.setProp(node.uid, 'size', [d.w, d.h]);
  }

  /** Pontos de um polígono de colisão (PHY-011). Valor ausente ou malformado vira lista vazia. */
  function pointsOf(v: unknown): [number, number][] {
    return Array.isArray(v) ? (v as [number, number][]) : [];
  }

  /** Propriedades que já aparecem no cabeçalho (Tag, Layer e Estático) não se repetem nas seções. */
  const HEADER_GROUPS = new Set(['Identificação', 'Editor']);

  /** Grupos de propriedades de um tipo, na ordem em que aparecem (Transformação, Exibição…). */
  function groupsOf(t: NodeType) {
    const groups: { name: string; props: PropDef[] }[] = [];
    for (const p of t.props) {
      // The script has its own slot (INS-005) in the identification block, instead of a text field. Exported values
      // (the map) are in that slot too (VIS-022).
      if (HEADER_GROUPS.has(p.group) || p.name === 'script' || p.kind === 'map') continue;
      const g = groups.at(-1);
      if (g && g.name === p.group) g.props.push(p);
      else groups.push({ name: p.group, props: [p] });
    }
    return groups;
  }

  /** Tipos que batem com a busca, por nome ou por categoria (INS-004). */
  const addTypes = $derived.by(() => {
    const q = norm(addQuery.trim());
    return scene.types.filter((t) => !q || norm(t.name).includes(q) || norm(t.category).includes(q));
  });

  /** Os tipos da busca agrupados por categoria, na ordem do registro (a do menu de criação). */
  const addGroups = $derived.by(() => {
    const groups: { category: string; types: NodeType[] }[] = [];
    for (const t of addTypes) {
      const g = groups.find((x) => x.category === t.category);
      if (g) g.types.push(t);
      else groups.push({ category: t.category, types: [t] });
    }
    return groups;
  });

  /** Cria um nó do tipo escolhido como filho do nó selecionado (INS-004). */
  function addNode(kind: string) {
    if (!node) return;
    const uid = scene.add(kind, node.uid);
    if (uid) scene.select(uid);
    closeAdd();
  }

  function closeAdd() {
    addOpen = false;
    addQuery = '';
  }
</script>

<section class="insp" use:focusPanel={'inspector'} class:focused={focus.panel === 'inspector'} aria-label="Inspector">
  <PanelHeader title="Inspector" />

  {#if !node}
    <div class="none">
      <strong>Nada selecionado</strong>
      <span>Escolha um nó na hierarquia ou no viewport para ver as propriedades.</span>
    </div>
  {:else if multi}
    <div class="none">
      <strong>{scene.selection.length} nós selecionados</strong>
      <span>A edição de vários nós ao mesmo tempo chega na próxima etapa. Selecione um só para editar.</span>
    </div>
  {:else}
    <div class="head">
      <button
        class="tick"
        class:off={!node.visible}
        title={node.visible ? 'Ativo: visível no jogo' : 'Inativo: oculto'}
        aria-label="Ativo"
        onclick={() => scene.setProp(node!.uid, 'visible', !node!.visible)}
      >
        {#if node.visible}<Eye size={13} />{:else}<EyeOff size={13} />{/if}
      </button>
      <input
        class="name"
        bind:value={nameDraft}
        aria-label="Nome do nó"
        onblur={commitName}
        onkeydown={(e) => {
          if (e.key === 'Enter') (e.currentTarget as HTMLInputElement).blur();
          if (e.key === 'Escape') {
            nameDraft = node!.name;
            (e.currentTarget as HTMLInputElement).blur();
          }
        }}
      />
      <button
        class="static"
        class:on={!!node.props.locked}
        title="Estático: trava a posição no editor"
        onclick={() => scene.setProp(node!.uid, 'locked', !node!.props.locked)}
      >
        {#if node.props.locked}<Lock size={12} />{:else}<LockOpen size={12} />{/if}
        Estático
      </button>
    </div>
    <div class="kind-line">
      <NodeIcon kind={node.kind} size={12} />
      <span class="mono">{node.kind}</span>
      <span class="sub mono">{chain.map((t) => t.name).join(' › ')}</span>
    </div>
    <div class="ident">
      <NameField
        compact
        label="Tag"
        value={String(node.props.tag ?? 'untagged')}
        options={tagNames}
        labels={ENUM_LABELS.tag}
        onchange={(tag) => scene.setProp(node!.uid, 'tag', tag)}
      />
      <NameField
        compact
        label="Layer"
        value={String(node.props.layer ?? 'default')}
        options={layerNames}
        labels={ENUM_LABELS.layer}
        onchange={(layer) => scene.setProp(node!.uid, 'layer', layer)}
      />
    </div>
    <ScriptSlot {node} />

    <div class="body">
      {#each chain as t (t.name)}
        {#if t.props.length}
          <div class="section">
            <div class="sec-head">
              <button class="fold" onclick={() => (collapsed[t.name] = !collapsed[t.name])} aria-expanded={!collapsed[t.name]}>
                {#if collapsed[t.name]}<ChevronRight size={12} />{:else}<ChevronDown size={12} />{/if}
                <NodeIcon kind={t.name} size={11} />
                <span>{t.name}</span>
              </button>
              <button class="kebab" aria-label="Opções de {t.name}" onclick={(e) => sectionMenu(e, t)}>
                <Ellipsis size={13} />
              </button>
            </div>

            {#if !collapsed[t.name]}
              <div class="props">
                {#each groupsOf(t) as g (g.name)}
                  <div class="group">{g.name}</div>
                  {#each g.props as def (def.name)}
                    {@const v = value(def)}
                    <div class="prop" class:changed={node.props[def.name] !== undefined}>
                      {#if def.kind === 'float' && def.slider && def.min !== null && def.max !== null}
                        <RangeField
                          label={def.label}
                          value={Number(v)}
                          min={def.min}
                          max={def.max}
                          step={def.step ?? 0.1}
                          precision={decimals(def.step)}
                          suffix={def.unit ? ' ' + def.unit : ''}
                          onchange={(x) => setProp(def, x, true)}
                          onend={endEdit}
                        />
                      {:else if def.kind === 'float' || def.kind === 'int'}
                        <NumberField
                          label={def.label}
                          value={Number(v)}
                          step={def.step ?? (def.kind === 'int' ? 1 : 0.1)}
                          precision={decimals(def.step)}
                          suffix={def.unit ?? ''}
                          min={def.min ?? -Infinity}
                          max={def.max ?? Infinity}
                          onchange={(x) => setProp(def, def.kind === 'int' ? Math.round(x) : x, true)}
                          onend={endEdit}
                        />
                      {:else if def.kind === 'vec2'}
                        {@const pair = (Array.isArray(v) ? v : [0, 0]) as number[]}
                        <div class="vec">
                          <span class="vlabel">{def.label}</span>
                          <NumberField
                            label="X"
                            value={pair[0]}
                            step={def.step ?? 1}
                            precision={decimals(def.step)}
                            onchange={(x) => setProp(def, [x, pair[1]], true)}
                            onend={endEdit}
                          />
                          <NumberField
                            label="Y"
                            value={pair[1]}
                            step={def.step ?? 1}
                            precision={decimals(def.step)}
                            onchange={(y) => setProp(def, [pair[0], y], true)}
                            onend={endEdit}
                          />
                        </div>
                      {:else if def.kind === 'bool'}
                        <label class="check">
                          <input type="checkbox" checked={!!v} onchange={(e) => setProp(def, e.currentTarget.checked)} />
                          <span>{def.label}</span>
                        </label>
                      {:else if def.kind === 'color'}
                        <div class="campo">
                          <span class="flabel">{def.label}</span>
                          <div class="swatches">
                            {#each OBJECT_PALETTE as c (c)}
                              <button
                                class="swatch"
                                class:on={String(v).toLowerCase() === c}
                                style="background: {c}"
                                title={c}
                                aria-label="Cor {c}"
                                onclick={() => setProp(def, c)}
                              ></button>
                            {/each}
                            <input
                              class="hex mono"
                              value={String(v)}
                              aria-label="Cor em hexadecimal"
                              onchange={(e) => setProp(def, e.currentTarget.value)}
                            />
                          </div>
                        </div>
                      {:else if def.kind === 'mask'}
                        <LayerMaskField label={def.label} value={Number(v)} onchange={(m) => setProp(def, m)} />
                      {:else if def.kind === 'enum'}
                        <label class="campo">
                          <span class="flabel">{def.label}</span>
                          <select class="sel" value={String(v)} onchange={(e) => setProp(def, e.currentTarget.value)}>
                            {#each def.options as o (o)}
                              <option value={o}>{enumLabel(def, o)}</option>
                            {/each}
                          </select>
                        </label>
                      {:else if def.kind === 'points'}
                        <PointsField
                          label={def.label}
                          points={pointsOf(v)}
                          onset={(pts, coalesce) => setProp(def, pts, coalesce)}
                          onend={endEdit}
                        />
                      {:else if def.kind === 'uid' && def.asset}
                        <AssetField
                          label={def.label}
                          kind={def.asset === 'audio' ? 'audio' : 'texture'}
                          value={String(v)}
                          onchange={(uid) => (def.asset === 'texture' ? pickTexture(def, uid) : setProp(def, uid))}
                        />
                      {:else if def.kind === 'uid'}
                        <NodeRefField label={def.label} value={String(v)} self={node.uid} onchange={(uid) => setProp(def, uid)} />
                      {:else if def.names}
                        <NameField
                          label={def.label}
                          value={String(v)}
                          options={namesOf(projectSettings, def.names)}
                          labels={ENUM_LABELS[def.name]}
                          emptyLabel={def.default === '' ? 'Nenhum' : null}
                          onchange={(name) => setProp(def, name)}
                        />
                      {:else}
                        <label class="campo">
                          <span class="flabel">{def.label}</span>
                          <input class="txt" value={String(v)} onchange={(e) => setProp(def, e.currentTarget.value)} />
                        </label>
                      {/if}
                    </div>
                  {/each}
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      {/each}

      <div class="section script">
        <div class="sec-head"><span class="script-title"><Braces size={12} /> Script visual</span></div>
        <p>Nenhum script. Disponível na Fase 2, com a programação visual.</p>
      </div>
    </div>

    <div class="foot">
      <button class="add" onclick={() => (addOpen ? closeAdd() : (addOpen = true))} aria-expanded={addOpen}>
        <Plus size={13} /> Adicionar nó
      </button>
      {#if addOpen}
        <div class="pop" role="menu">
          <label class="pop-search">
            <Search size={11} />
            <input
              bind:value={addQuery}
              placeholder="Buscar tipo ou categoria"
              aria-label="Buscar tipo de nó"
              onkeydown={(e) => {
                if (e.key === 'Escape') closeAdd();
              }}
            />
          </label>
          {#each addGroups as g (g.category)}
            <div class="pop-cat" role="presentation">{g.category}</div>
            {#each g.types as t (t.name)}
              <button class="pop-item" role="menuitem" onclick={() => addNode(t.name)}>
                <NodeIcon kind={t.name} size={12} />
                <span>{t.name}</span>
              </button>
            {/each}
          {:else}
            <div class="pop-empty">Nenhum tipo encontrado</div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .insp {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--bg-1);
  }
  .none {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 16px 14px;
    font-size: var(--fs-12);
    color: var(--text-faint);
  }
  .none strong {
    color: var(--text-dim);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 8px 4px;
  }
  .tick {
    width: 22px;
    height: 22px;
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--r-ctl);
    color: var(--success);
  }
  .tick.off {
    color: var(--text-faint);
  }
  .name {
    flex: 1;
    min-width: 0;
    height: 22px;
    padding: 0 6px;
    border-radius: var(--r-ctl);
    background: var(--bg-2);
    color: var(--text);
    font-size: var(--fs-12);
    font-weight: 600;
    box-shadow: inset 0 0 0 1px var(--line);
    outline: none;
  }
  .name:focus {
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .static {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 22px;
    padding: 0 6px;
    border-radius: var(--r-ctl);
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .static:hover,
  .static.on {
    color: var(--text);
    background: var(--bg-hover);
  }
  .kind-line {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px 8px 36px;
    font-size: var(--fs-11);
    color: var(--text-dim);
    border-bottom: 1px solid var(--line-soft);
  }
  .ident {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--line-soft);
  }
  .kind-line .sub {
    color: var(--text-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding-bottom: 8px;
  }
  .section {
    border-bottom: 1px solid var(--line-soft);
  }
  .sec-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 24px;
    padding: 0 4px 0 6px;
    background: var(--bg-toolbar);
  }
  .fold {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-12);
    font-weight: 600;
    color: var(--text);
  }
  .kebab {
    width: 20px;
    height: 20px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--r-ctl);
    color: var(--text-faint);
  }
  .kebab:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
  .props {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 4px 8px 8px;
  }
  .group {
    margin-top: 4px;
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .prop {
    min-width: 0;
  }
  .prop.changed :global(.label) {
    font-weight: 600;
    color: var(--text);
  }
  .vec {
    display: grid;
    grid-template-columns: 84px minmax(0, 1fr) minmax(0, 1fr);
    gap: 4px;
    align-items: center;
  }
  .vlabel,
  .flabel {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-11);
    color: var(--text-dim);
  }
  .campo {
    display: grid;
    grid-template-columns: 84px minmax(0, 1fr);
    align-items: start;
    gap: 6px;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-11);
    color: var(--text-dim);
    padding-left: 80px;
  }
  .sel,
  .txt,
  .hex {
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
  .sel:focus,
  .txt:focus,
  .hex:focus {
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .swatches {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px;
    min-height: 20px;
  }
  .swatch {
    width: 14px;
    height: 14px;
    border-radius: var(--r-ctl);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .swatch.on {
    box-shadow: 0 0 0 2px var(--bg-1), 0 0 0 3px var(--accent-hi);
  }
  .hex {
    width: 76px;
  }
  .script {
    padding-bottom: 6px;
  }
  .script-title {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-12);
    font-weight: 600;
    color: var(--text-dim);
  }
  .script p {
    margin: 0;
    padding: 4px 10px 0;
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .foot {
    flex: none;
    position: relative;
    padding: 6px;
    border-top: 1px solid var(--line);
  }
  .add {
    width: 100%;
    height: 24px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    border-radius: var(--r-ctl);
    background: var(--bg-2);
    color: var(--text);
    font-size: var(--fs-12);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .add:hover {
    background: var(--bg-hover);
  }
  .pop {
    position: absolute;
    left: 6px;
    right: 6px;
    bottom: 34px;
    max-height: 280px;
    overflow: auto;
    padding: 4px;
    border-radius: var(--r-dlg);
    background: var(--bg-overlay);
    backdrop-filter: blur(12px);
    box-shadow: 0 0 0 1px var(--line), var(--shadow-pop);
    z-index: 5;
  }
  .pop-search {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 6px;
    color: var(--text-faint);
    border-bottom: 1px solid var(--line-soft);
    margin-bottom: 4px;
  }
  .pop-search input {
    flex: 1;
    background: none;
    border: 0;
    outline: none;
    color: var(--text);
    font-size: var(--fs-12);
  }
  .pop-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 24px;
    padding: 0 6px;
    border-radius: var(--r-ctl);
    font-size: var(--fs-12);
    color: var(--text);
    text-align: left;
  }
  .pop-item:hover {
    background: var(--bg-hover);
  }
  .pop-cat {
    padding: 6px 6px 2px;
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .pop-empty {
    padding: 8px;
    font-size: var(--fs-12);
    color: var(--text-faint);
  }
</style>
