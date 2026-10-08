<script lang="ts">
  // Seletor do tipo ou do nome de um nó de propriedade (VIS-021). O valor é o que está gravado no nó, mesmo quando
  // ele não está na lista (um nome que o tipo não tem), para o usuário ver o que precisa corrigir.
  let {
    valor,
    opcoes,
    vazio,
    onchange,
  }: {
    valor: string;
    opcoes: { value: string; label: string }[];
    /** Texto da opção vazia: "sem tipo" para o tipo, "escolha" para o nome. */
    vazio: string;
    onchange: (v: string) => void;
  } = $props();

  const pare = (e: Event) => e.stopPropagation();
  const lista = $derived(
    valor && !opcoes.some((o) => o.value === valor) ? [...opcoes, { value: valor, label: valor }] : opcoes,
  );
</script>

<select
  class="campo"
  value={valor}
  onpointerdown={pare}
  onchange={(e) => onchange(e.currentTarget.value)}
  aria-label={vazio}
>
  <option value="">{vazio}</option>
  {#each lista as o (o.value)}
    <option value={o.value}>{o.label}</option>
  {/each}
</select>

<style>
  .campo {
    height: 18px;
    min-width: 0;
    flex: 0 1 110px;
    max-width: 110px;
    padding: 0 2px;
    border-radius: var(--r-ctl);
    background: var(--bg-0);
    border: 1px solid var(--line);
    outline: none;
    font-family: var(--font-mono);
    font-size: var(--fs-11);
    color: var(--text);
  }
  .campo:focus {
    border-color: var(--accent);
  }
</style>
