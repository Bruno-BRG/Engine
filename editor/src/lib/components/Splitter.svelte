<script lang="ts">
  let {
    dir,
    value,
    min,
    max,
    invert = false,
    onchange,
    onend,
  }: {
    dir: 'x' | 'y';
    value: number;
    min: number;
    max: number;
    invert?: boolean;
    onchange: (v: number) => void;
    onend?: () => void;
  } = $props();

  let active = $state(false);

  function down(e: PointerEvent) {
    e.preventDefault();
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    active = true;
    const start = dir === 'x' ? e.clientX : e.clientY;
    const startValue = value;
    const move = (ev: PointerEvent) => {
      const d = (dir === 'x' ? ev.clientX : ev.clientY) - start;
      onchange(Math.min(max, Math.max(min, startValue + (invert ? -d : d))));
    };
    const up = () => {
      active = false;
      el.removeEventListener('pointermove', move);
      el.removeEventListener('pointerup', up);
      onend?.();
    };
    el.addEventListener('pointermove', move);
    el.addEventListener('pointerup', up);
  }
</script>

<div class="split {dir}" class:active role="separator" aria-orientation={dir === 'x' ? 'vertical' : 'horizontal'} onpointerdown={down}></div>

<style>
  .split {
    position: relative;
    z-index: 5;
    flex: none;
  }
  .split::after {
    content: '';
    position: absolute;
    background: var(--accent);
    opacity: 0;
    transition: opacity var(--t-fast) var(--ease);
  }
  .split:hover::after,
  .split.active::after {
    opacity: 1;
    transition-delay: 80ms;
  }
  .x {
    width: 2px;
    cursor: col-resize;
    background: var(--bg-0);
  }
  .x::after {
    inset: 0 -2px;
  }
  .x::before {
    content: '';
    position: absolute;
    inset: 0 -4px;
  }
  .y {
    height: 2px;
    cursor: row-resize;
    background: var(--bg-0);
  }
  .y::after {
    inset: -2px 0;
  }
  .y::before {
    content: '';
    position: absolute;
    inset: -4px 0;
  }
</style>
