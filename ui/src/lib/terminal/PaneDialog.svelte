<script lang="ts">
  import { onMount, type Snippet } from 'svelte';

  let {
    title,
    tone = 'default',
    onescape,
    children,
    actions
  }: {
    title: string;
    tone?: 'default' | 'danger';
    /** Esc; omit for dialogs that must be answered with a button (changed host key). */
    onescape?: () => void;
    children: Snippet;
    actions: Snippet;
  } = $props();

  let card: HTMLDivElement;
  const uid = $props.id();

  const FOCUSABLE =
    'button:not(:disabled), input:not(:disabled), [href], [tabindex]:not([tabindex="-1"])';

  onMount(() => {
    const first =
      card.querySelector<HTMLElement>('[data-autofocus]') ??
      card.querySelector<HTMLElement>(FOCUSABLE);
    first?.focus();
  });

  /** Focus stays inside the card; the terminal behind is inert. */
  function onkeydown(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === 'Escape' && onescape) {
      e.preventDefault();
      onescape();
      return;
    }
    if (e.key !== 'Tab') return;
    const items = [...card.querySelectorAll<HTMLElement>(FOCUSABLE)];
    if (!items.length) return;
    const first = items[0]!;
    const last = items[items.length - 1]!;
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  }
</script>

<div class="scrim">
  <div
    bind:this={card}
    class="card"
    class:danger={tone === 'danger'}
    role={tone === 'danger' ? 'alertdialog' : 'dialog'}
    aria-modal="true"
    aria-labelledby="{uid}-t"
    tabindex="-1"
    {onkeydown}
  >
    <h2 id="{uid}-t">{title}</h2>
    <div class="body">{@render children()}</div>
    <div class="actions">{@render actions()}</div>
  </div>
</div>

<style>
  .scrim {
    position: absolute;
    inset: 0;
    z-index: 20;
    display: grid;
    place-items: center;
    padding: 16px;
    background: color-mix(in oklab, var(--bg) 72%, transparent);
    animation: rise-fade 140ms var(--ease);
  }
  .card {
    width: min(500px, 100%);
    max-height: 100%;
    overflow: auto;
    display: grid;
    gap: 14px;
    padding: 20px;
    border: 0.5px solid var(--border);
    border-radius: var(--r-card);
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow);
    animation: rise 140ms var(--ease);
  }
  .card.danger {
    border-color: color-mix(in oklab, var(--danger) 45%, var(--border));
    background: color-mix(in oklab, var(--danger) 5%, var(--surface));
  }
  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.005em;
    text-wrap: balance;
  }
  .danger h2 {
    color: var(--danger);
  }
  .body {
    display: grid;
    gap: 12px;
    color: var(--muted);
    text-wrap: pretty;
  }
  .body :global(p) {
    margin: 0;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
    margin-top: 2px;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
  }
  @keyframes rise-fade {
    from {
      opacity: 0;
    }
  }
</style>
