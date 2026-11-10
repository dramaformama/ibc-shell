<script lang="ts">
  /** Shown while files from the OS are dragged over a terminal. */
  let { text }: { text: string } = $props();
</script>

<div class="drop" aria-hidden="true"><span>{text}</span></div>

<style>
  .drop {
    position: absolute;
    inset: 8px;
    display: grid;
    place-items: center;
    border: 2px dashed color-mix(in oklab, var(--accent) 70%, transparent);
    border-radius: var(--r-card);
    background: color-mix(in oklab, var(--accent) 10%, transparent);
    pointer-events: none;
    z-index: 4;
  }
  .drop span {
    max-width: 80%;
    padding: 8px 14px;
    border-radius: 999px;
    background: var(--surface);
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
    box-shadow: var(--card-shadow);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
