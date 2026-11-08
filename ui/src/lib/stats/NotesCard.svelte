<script lang="ts">
  import { NotebookPen } from '@lucide/svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { tree } from '$lib/stores/tree.svelte';

  /** Free-form notes about one server; saved when the field loses focus. */
  let { hostId }: { hostId: string } = $props();
  const saved = $derived(tree.host(hostId)?.notes ?? '');
  let draft = $state<string | null>(null);
  const value = $derived(draft ?? saved);

  async function commit() {
    if (draft === null || draft === saved) {
      draft = null;
      return;
    }
    const next = draft.trim();
    if (await tree.updateHost(hostId, { notes: next || null })) draft = null;
  }
</script>

<section class="card notes" aria-label={m.section_notes()}>
  <h3><NotebookPen size={14} />{m.section_notes()}</h3>
  <textarea
    class="input"
    rows="3"
    placeholder={m.notes_placeholder()}
    {value}
    oninput={(e) => (draft = e.currentTarget.value)}
    onblur={commit}></textarea>
</section>

<style>
  .notes {
    padding: var(--card-pad);
    margin-top: var(--grid-gap);
  }
  h3 {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 0 0 10px;
    font-size: 13px;
    font-weight: 600;
    color: var(--muted);
  }
  textarea {
    width: 100%;
    resize: vertical;
    min-height: 72px;
    field-sizing: content;
  }
</style>
