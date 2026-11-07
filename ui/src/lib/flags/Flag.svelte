<script lang="ts">
  import { normalizeCode } from './flags';

  interface Props {
    /** Flag code, alias or null (renders the grey placeholder). */
    code?: string | null;
    /** Square edge in CSS pixels. */
    size?: number;
    /** Accessible label; decorative when omitted. */
    title?: string;
    class?: string;
  }

  let { code = null, size = 16, title, class: className = '' }: Props = $props();

  const resolved = $derived(normalizeCode(code));
  const src = $derived(resolved ? `/flags/${resolved}.svg` : null);
  // Remember which source failed instead of resetting state on each change.
  let failedSrc = $state<string | null>(null);
  const failed = $derived(src !== null && failedSrc === src);
  const label = $derived((resolved ?? code ?? '').slice(0, 2).toUpperCase());
</script>

{#if src && !failed}
  <img
    class="flag {className}"
    {src}
    width={size}
    height={size}
    alt={title ?? ''}
    loading="lazy"
    decoding="async"
    onerror={() => (failedSrc = src)}
  />
{:else}
  <span
    class="flag flag-fallback {className}"
    style:width="{size}px"
    style:height="{size}px"
    style:font-size="{Math.round(size * 0.42)}px"
    role={title ? 'img' : undefined}
    aria-label={title}
    aria-hidden={title ? undefined : 'true'}>{label}</span
  >
{/if}

<style>
  .flag {
    display: inline-block;
    flex: none;
    border-radius: 50%;
    vertical-align: middle;
    object-fit: cover;
  }
  .flag-fallback {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: #9ca3af;
    color: #f9fafb;
    font-weight: 600;
    line-height: 1;
    letter-spacing: 0.02em;
    user-select: none;
  }
</style>
