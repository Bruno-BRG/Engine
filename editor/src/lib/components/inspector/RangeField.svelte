<script lang="ts">
  // Controle deslizante de um número com faixa (INS-003). Mostra o valor ao lado. Arrastar grava em
  // passos de desfazer juntos; soltar fecha o passo. Setas do teclado também funcionam, pelo próprio controle.
  let {
    label,
    value,
    min,
    max,
    step = 0.1,
    precision = 1,
    suffix = '',
    onchange,
    onend,
  }: {
    label: string;
    value: number;
    min: number;
    max: number;
    step?: number;
    precision?: number;
    suffix?: string;
    onchange: (v: number) => void;
    onend?: () => void;
  } = $props();

  const fmt = (v: number) => Number(v.toFixed(precision)).toString().replace('.', ',');
</script>

<div class="rng">
  <span class="flabel">{label}</span>
  <input
    class="slider"
    type="range"
    {min}
    {max}
    {step}
    value={value}
    aria-label={label}
    oninput={(e) => onchange(Number(e.currentTarget.value))}
    onchange={() => onend?.()}
  />
  <output class="out">{fmt(value)}{suffix}</output>
</div>

<style>
  .rng {
    display: grid;
    grid-template-columns: 84px minmax(0, 1fr) 56px;
    align-items: center;
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
  .slider {
    width: 100%;
    min-width: 0;
    height: 14px;
    margin: 0;
    accent-color: var(--accent);
  }
  .out {
    font-family: var(--font-mono);
    font-size: var(--fs-11);
    color: var(--text);
    text-align: right;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
