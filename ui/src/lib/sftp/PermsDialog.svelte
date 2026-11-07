<script lang="ts">
  import { Dialog } from 'bits-ui';
  import * as m from '$lib/paraglide/messages.js';
  import { parseOctal, toOctal } from './model';

  let {
    target,
    apply,
    cancel
  }: {
    target: { name: string; mode: number } | null;
    apply: (mode: number) => void;
    cancel: () => void;
  } = $props();

  let bits = $state(0);
  let text = $state('');

  $effect(() => {
    if (target) {
      bits = target.mode & 0o7777;
      text = toOctal(bits);
    }
  });

  const rows = [
    { label: () => m.sftp_perm_owner(), shift: 6 },
    { label: () => m.sftp_perm_group(), shift: 3 },
    { label: () => m.sftp_perm_other(), shift: 0 }
  ];
  const flags = [
    { label: () => m.sftp_perm_read(), bit: 4 },
    { label: () => m.sftp_perm_write(), bit: 2 },
    { label: () => m.sftp_perm_exec(), bit: 1 }
  ];

  function toggle(shift: number, bit: number) {
    bits ^= bit << shift;
    text = toOctal(bits);
  }
  function typed() {
    const v = parseOctal(text);
    if (v !== null) bits = v;
  }
  const valid = $derived(parseOctal(text) !== null);
</script>

<Dialog.Root open={target !== null} onOpenChange={(o) => !o && cancel()}>
  <Dialog.Portal>
    <Dialog.Overlay class="overlay" />
    <Dialog.Content class="sf-dialog">
      <Dialog.Title>{m.sftp_permissions()}</Dialog.Title>
      <Dialog.Description class="sf-name" title={target?.name}>{target?.name}</Dialog.Description>
      <table class="sf-perms">
        <thead>
          <tr
            ><td></td>{#each flags as f (f.bit)}<th scope="col">{f.label()}</th>{/each}</tr
          >
        </thead>
        <tbody>
          {#each rows as r (r.shift)}
            <tr>
              <th scope="row">{r.label()}</th>
              {#each flags as f (f.bit)}
                <td>
                  <input
                    type="checkbox"
                    aria-label="{r.label()}: {f.label()}"
                    checked={!!(bits & (f.bit << r.shift))}
                    onchange={() => toggle(r.shift, f.bit)}
                  />
                </td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
      <label class="sf-octal">
        <span>{m.sftp_perm_octal()}</span>
        <input
          class="input mono"
          bind:value={text}
          oninput={typed}
          maxlength="4"
          spellcheck="false"
        />
      </label>
      <div class="sf-actions">
        <button class="btn btn-ghost" onclick={cancel}>{m.action_cancel()}</button>
        <button class="btn btn-primary" disabled={!valid} onclick={() => apply(bits)}
          >{m.action_save()}</button
        >
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
