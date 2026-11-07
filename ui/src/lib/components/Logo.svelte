<script lang="ts">
  /** The app mark (same drawing as the favicon and the menu-bar glyph). */
  let { size = 24 }: { size?: number } = $props();
</script>

<svg
  class="logo"
  width={size}
  height={size}
  viewBox="0 0 100 100"
  aria-hidden="true"
  focusable="false"
>
  <rect width="100" height="100" rx="22" fill="#05070b" />
  <path
    d="M24 33L46 50L24 67"
    fill="none"
    stroke="#f5f6f8"
    stroke-width="6.5"
    stroke-linecap="round"
    stroke-linejoin="round"
  />
  <circle cx="67" cy="66" r="12" fill="#ff8a00" />
</svg>

<style>
  .logo {
    flex: none;
    display: block;
    border-radius: 22%;
    /* Keeps the dark tile visible on a dark background. */
    box-shadow: 0 0 0 0.5px oklch(1 0 0 / 0.12);
  }
</style>
