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
    <span class="tile"><Icon size={22} /></span>
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
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    margin-bottom: 6px;
    border-radius: 15px;
    background: var(--surface-2);
    color: var(--muted);
  }
  h3 {
    margin: 0;
    font-size: 14px;
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
