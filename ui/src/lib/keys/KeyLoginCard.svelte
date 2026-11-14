<script lang="ts">
  import { KeyRound } from '@lucide/svelte';
  import { api, type HostId, type HostPatch, type IdentityDto, type SessionId } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { tree } from '$lib/stores/tree.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { errorText } from '$lib/util/errors';

  /** Switch a server to key sign-in: create or pick a key, install it, use it (ssh-copy-id). */
  let { hostId, sid }: { hostId: HostId; sid: SessionId | null } = $props();

  let idents = $state.raw<IdentityDto[]>([]);
  let choice = $state('new');
  let busy = $state(false);

  void api.identityList().then((r) => {
    if (r.status === 'ok') idents = r.data;
  });

  const host = $derived(tree.host(hostId));
  const current = $derived(idents.find((i) => i.id === host?.identity_id) ?? null);
  const usesKey = $derived(!!current?.public_key);
  const keys = $derived(idents.filter((i) => i.public_key));

  async function install(id: string, switchTo: boolean) {
    if (!sid || !host) return;
    const r = await api.identityInstall(sid, id);
    if (r.status === 'error') return ui.toast(errorText(r.error), 'error');
    if (switchTo) {
      const patch: HostPatch = { identity_id: id };
      // The login may have lived on the password identity only: keep it on the host.
      if (!host.username && current?.username) patch.username = current.username;
      if (!(await tree.updateHost(hostId, patch))) return;
    }
    ui.toast(r.data ? m.keylogin_done() : m.keylogin_already());
  }

  async function run() {
    if (!host) return;
    busy = true;
    let id = choice;
    if (choice === 'new') {
      const g = await api.identityGenerate(host.label, 'ed25519');
      if (g.status === 'error') {
        busy = false;
        return ui.toast(errorText(g.error), 'error');
      }
      idents = [...idents, g.data];
      id = g.data.id;
      choice = id;
    }
    await install(id, true);
    busy = false;
  }
</script>

{#if host}
  <section class="card klog" aria-label={m.keylogin_title()}>
    <h3><KeyRound size={14} />{m.keylogin_title()}</h3>
    {#if usesKey && current}
      <div class="row">
        <p>{m.keylogin_now_key({ name: current.label })}</p>
        <button
          class="btn btn-sm"
          disabled={!sid || busy}
          title={sid ? undefined : m.keylogin_need_session()}
          onclick={async () => {
            busy = true;
            await install(current.id, false);
            busy = false;
          }}>{m.keylogin_reinstall()}</button
        >
      </div>
    {:else}
      <p>{current ? m.keylogin_now_password() : m.keylogin_now_none()}</p>
      <div class="row">
        <select class="input" bind:value={choice} aria-label={m.keylogin_pick()}>
          <option value="new">{m.keylogin_new()}</option>
          {#each keys as k (k.id)}
            <option value={k.id}>{k.label}{k.key_algo ? ` · ${k.key_algo}` : ''}</option>
          {/each}
        </select>
        <button
          class="btn btn-primary btn-sm"
          disabled={!sid || busy}
          title={sid ? undefined : m.keylogin_need_session()}
          onclick={run}>{m.keylogin_run()}</button
        >
      </div>
      <p class="note">{sid ? m.keylogin_note() : m.keylogin_need_session()}</p>
    {/if}
  </section>
{/if}

<style>
  .klog {
    padding: var(--card-pad);
    margin-top: var(--grid-gap);
    display: grid;
    gap: 10px;
  }
  h3 {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 0;
    font-size: var(--fs-body);
    font-weight: 600;
    color: var(--text);
  }
  p {
    margin: 0;
    font-size: 13px;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .row p {
    flex: 1;
  }
  select {
    flex: 1 1 200px;
    max-width: 320px;
  }
</style>
