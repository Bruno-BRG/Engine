<script lang="ts">
  let {
    label,
    value,
    step = 1,
    precision = 0,
    suffix = '',
    min = -Infinity,
    max = Infinity,
    disabled = false,
    onchange,
    onend,
  }: {
    label: string;
    value: number;
    step?: number;
    precision?: number;
    suffix?: string;
    min?: number;
    max?: number;
    disabled?: boolean;
    onchange: (v: number) => void;
    onend?: () => void;
  } = $props();

  let editing = $state(false);
  let text = $state('');
  let scrubbing = $state(false);
  let input: HTMLInputElement;
  let cancel = false;

  const clamp = (v: number) => Math.min(max, Math.max(min, v));
  const round = (v: number) => Number(v.toFixed(precision));
  const fmt = (v: number) => round(v).toString().replace('.', ',');

  /** Aceita números e expressões simples como 32*4 ou 100/3. */
  function evaluate(s: string): number | null {
    const t = s.replace(/,/g, '.').trim();
    if (!t || !/^[\d\s+\-*/().]+$/.test(t)) return null;
    try {
      const v = Function('"use strict";return (' + t + ')')();
      return typeof v === 'number' && Number.isFinite(v) ? v : null;
    } catch {
      return null;
    }
  }

  function scrub(e: PointerEvent) {
    if (disabled || e.button !== 0) return;
    e.preventDefault();
    const el = e.currentTarget as HTMLElement;
    let acc = value;
    let moved = false;
    el.requestPointerLock?.();
    scrubbing = true;
    const move = (ev: PointerEvent) => {
      if (!ev.movementX) return;
      const m = ev.shiftKey ? 10 : ev.altKey ? 0.1 : 1;
      acc += ev.movementX * step * m;
      moved = true;
      onchange(round(clamp(acc)));
    };
    const up = () => {
      scrubbing = false;
      document.exitPointerLock?.();
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      onend?.();
      if (!moved) input.focus();
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  }

  function commit() {
    if (!cancel) {
      const v = evaluate(text);
      if (v != null && round(clamp(v)) !== round(value)) onchange(round(clamp(v)));
      onend?.();
    }
    cancel = false;
    editing = false;
  }

  function key(e: KeyboardEvent) {
    if (e.key === 'Enter') input.blur();
    else if (e.key === 'Escape') {
      cancel = true;
      input.blur();
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.preventDefault();
      const m = e.shiftKey ? 10 : e.altKey ? 0.1 : 1;
      const v = round(clamp((evaluate(text) ?? value) + (e.key === 'ArrowUp' ? 1 : -1) * step * m));
      onchange(v);
      text = fmt(v);
    }
  }
</script>

<div class="nf" class:scrubbing class:disabled>
  <!-- svelte-ignore a11y_no_static_element_interactions (o campo de texto ao lado cobre o teclado) -->
  <span class="lbl" onpointerdown={scrub} title="Arraste para alterar · Shift ×10 · Alt ×0,1">{label}</span>
  <input
    bind:this={input}
    class="mono"
    value={editing ? text : fmt(value)}
    {disabled}
    spellcheck="false"
    onfocus={() => {
      editing = true;
      text = fmt(value);
      requestAnimationFrame(() => input.select());
    }}
    oninput={(e) => (text = e.currentTarget.value)}
    onblur={commit}
    onkeydown={key}
  />
  {#if suffix}<span class="suf">{suffix}</span>{/if}
</div>

<style>
  .nf {
    display: flex;
    align-items: center;
    height: 18px;
    border-radius: var(--r-ctl);
    background: var(--bg-2);
    border: 1px solid var(--line);
    overflow: hidden;
    transition: border-color var(--t-fast) var(--ease);
  }
  .nf:hover {
    border-color: var(--text-faint);
  }
  .nf:focus-within,
  .nf.scrubbing {
    border-color: var(--accent);
  }
  .lbl {
    flex: none;
    min-width: 22px;
    height: 100%;
    padding: 0 6px;
    display: grid;
    place-items: center;
    font-size: var(--fs-11);
    font-weight: 500;
    color: var(--text-dim);
    cursor: ew-resize;
    border-right: 1px solid var(--line);
    background: var(--bg-toolbar);
    transition: color var(--t-fast), background var(--t-fast);
  }
  .lbl:hover,
  .scrubbing .lbl {
    color: var(--text);
    background: var(--selection);
  }
  input {
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0 8px;
    background: none;
    border: none;
    outline: none;
    font-size: var(--fs-12);
    text-align: right;
  }
  .suf {
    padding-right: 8px;
    font-size: var(--fs-11);
    color: var(--text-faint);
  }
  .disabled {
    opacity: 0.5;
  }
  .disabled .lbl {
    cursor: default;
  }
</style>
