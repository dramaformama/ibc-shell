<script lang="ts">
  import { Dialog } from 'bits-ui';
  import * as m from '$lib/paraglide/messages.js';

  let {
    open = $bindable(false),
    title,
    initial = '',
    submit,
    confirm
  }: {
    open: boolean;
    title: string;
    initial?: string;
    submit: (name: string) => void;
    confirm: string;
  } = $props();

  let value = $state('');
  let input = $state<HTMLInputElement>();
  const bad = $derived(!value.trim() || /[/\\]/.test(value) || value === '.' || value === '..');

  $effect(() => {
    if (open) {
      value = initial;
      queueMicrotask(() => {
        input?.focus();
        const dot = initial.lastIndexOf('.');
        input?.setSelectionRange(0, dot > 0 ? dot : initial.length);
      });
    }
  });

  function go(e: Event) {
    e.preventDefault();
    if (bad) return;
    open = false;
    submit(value.trim());
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content class="sf-dialog" onOpenAutoFocus={(e) => e.preventDefault()}>
      <Dialog.Title>{title}</Dialog.Title>
      <Dialog.Description class="sr">{m.sftp_name_hint()}</Dialog.Description>
      <form onsubmit={go}>
        <input
          class="input"
          bind:this={input}
          bind:value
          spellcheck="false"
          autocomplete="off"
          aria-label={m.sftp_name()}
        />
        <div class="sf-actions">
          <Dialog.Close class="btn btn-ghost" type="button">{m.action_cancel()}</Dialog.Close>
          <button class="btn btn-primary" type="submit" disabled={bad}>{confirm}</button>
        </div>
      </form>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
