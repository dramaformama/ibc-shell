<script lang="ts">
  import { Dialog } from 'bits-ui';
  import * as m from '$lib/paraglide/messages.js';

  let {
    names,
    folders,
    confirm,
    cancel
  }: { names: string[] | null; folders: number; confirm: () => void; cancel: () => void } =
    $props();
</script>

<Dialog.Root open={names !== null} onOpenChange={(o) => !o && cancel()}>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content class="sf-dialog">
      {#if names}
        <Dialog.Title>
          {names.length === 1
            ? m.delete_title({ name: names[0] ?? '' })
            : m.sftp_delete_n({ count: names.length })}
        </Dialog.Title>
        <Dialog.Description>
          {folders > 0 ? m.sftp_delete_body_dirs({ folders }) : m.sftp_delete_body()}
        </Dialog.Description>
      {/if}
      <div class="sf-actions">
        <button class="btn btn-ghost" onclick={cancel}>{m.action_cancel()}</button>
        <button class="btn btn-primary btn-danger-fill" onclick={confirm}
          >{m.action_delete()}</button
        >
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
