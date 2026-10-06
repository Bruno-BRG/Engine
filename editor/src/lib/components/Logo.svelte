<script lang="ts">
  let { size = 20 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="64 64 896 896" aria-label="Nexo">
  <defs>
    <linearGradient id="nexo-logo-bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#9A82FF" />
      <stop offset="1" stop-color="#5132E8" />
    </linearGradient>
  </defs>
  <rect x="64" y="64" width="896" height="896" rx="216" fill="url(#nexo-logo-bg)" />
  <path
    d="M312 704 V352 C312 300 352 296 384 336 L640 688 C672 728 712 724 712 672 V320"
    fill="none"
    stroke="#fff"
    stroke-width="88"
    stroke-linecap="round"
    stroke-linejoin="round"
  />
  <circle cx="312" cy="712" r="72" fill="#fff" />
  <circle cx="312" cy="712" r="30" fill="#6A4CF5" />
  <circle cx="712" cy="312" r="72" fill="#fff" />
  <circle cx="712" cy="312" r="30" fill="#6A4CF5" />
</svg>
