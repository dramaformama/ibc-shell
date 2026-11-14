<script lang="ts">
  import type { Component, Snippet } from 'svelte';

  /**
   * The one empty / waiting state: an icon tile, a title, one line of help, and at most one
   * main action (the children). Used wherever a list or page has nothing to show yet.
   */
  let {
    icon: Icon,
    title,
    hint = '',
    busy = false,
    children
  }: {
    icon?: Component<{ size?: number | string }>;
    title: string;
    hint?: string;
    /** A pulsing dot instead of the icon: the state is temporary. */
    busy?: boolean;
    children?: Snippet;
  } = $props();
</script>

<div class="empty-state" role="status">
  {#if busy}
    <span class="tile"><span class="pulse"></span></span>
  {:else if Icon}
    <!-- A small stacked illustration: two faint sheets behind the icon. -->
    <span class="art" aria-hidden="true"><span class="tile"><Icon size={26} /></span></span>
  {/if}
  <h3>{title}</h3>
  {#if hint}<p>{hint}</p>{/if}
  {#if children}<div class="act">{@render children()}</div>{/if}
</div>

<style>
  .empty-state {
    display: grid;
    justify-items: center;
    gap: 6px;
    padding: 40px 20px;
    text-align: center;
    color: var(--muted);
  }
  .tile {
    position: relative;
    display: grid;
    place-items: center;
    width: 60px;
    height: 60px;
    border-radius: 18px;
    background: linear-gradient(160deg, var(--surface), var(--surface-2));
    box-shadow:
      0 0 0 0.5px var(--border),
      0 10px 24px -12px rgb(0 0 0 / 0.25);
    color: var(--muted);
  }
  .empty-state > .tile {
    margin-bottom: 10px;
  }
  .art {
    position: relative;
    display: grid;
    place-items: center;
    width: 96px;
    height: 80px;
    margin-bottom: 8px;
  }
  .art::before,
  .art::after {
    content: '';
    position: absolute;
    width: 56px;
    height: 56px;
    border-radius: 17px;
    background: var(--fill-2);
  }
  .art::before {
    transform: translate(-22px, 6px) rotate(-12deg);
  }
  .art::after {
    transform: translate(22px, 6px) rotate(12deg);
  }
  h3 {
    margin: 0;
    font-size: var(--fs-head);
    font-weight: 600;
    letter-spacing: -0.01em;
    color: var(--text);
  }
  p {
    max-width: 300px;
    margin: 0;
    font-size: 13px;
    line-height: 1.45;
    text-wrap: pretty;
  }
  .act {
    margin-top: 10px;
  }
  .pulse {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.25;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .pulse {
      animation: none;
    }
  }
</style>
