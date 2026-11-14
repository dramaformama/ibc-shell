<script lang="ts">
  import { untrack } from 'svelte';
  import { TriangleAlert } from '@lucide/svelte';
  import type { ForwardKind, HostId, PortForward } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { forwards, isLoopback } from './forwards.svelte';

  /** Inline editor for one forward rule (new when `rule` is null). */
  let {
    hostId,
    rule = null,
    done
  }: { hostId: HostId; rule?: PortForward | null; done: () => void } = $props();

  // Initial values only: the form edits its own copy.
  const init = untrack(() => rule);
  let kind = $state<ForwardKind>(init?.kind ?? 'local');
  let label = $state(init?.label ?? '');
  let bindAddr = $state(init?.bind_addr ?? '127.0.0.1');
  let bindPort = $state(String(init?.bind_port ?? ''));
  let destHost = $state(init?.dest_host ?? (init ? '' : 'localhost'));
  let destPort = $state(init?.dest_port ? String(init.dest_port) : '');
  let autoStart = $state(init?.auto_start ?? false);
  let error = $state<string | null>(null);
  let busy = $state(false);

  const hint = $derived(
    kind === 'local'
      ? m.fwd_hint_local()
      : kind === 'remote'
        ? m.fwd_hint_remote()
        : m.fwd_hint_dynamic()
  );
  const exposed = $derived(kind !== 'remote' && !isLoopback(bindAddr));
  const port = (s: string, min: number) =>
    /^\d+$/.test(s.trim()) && +s >= min && +s <= 65535 ? +s : null;

  async function save(e: SubmitEvent) {
    e.preventDefault();
    const bp = port(bindPort || '0', 0);
    if (bp === null) return void (error = m.fwd_err_port());
    const dp = kind === 'dynamic' ? null : port(destPort, 1);
    if (kind !== 'dynamic' && (dp === null || !destHost.trim()))
      return void (error = m.fwd_err_dest());
    error = null;
    busy = true;
    const fields = {
      kind,
      bind_addr: bindAddr.trim() || '127.0.0.1',
      bind_port: bp,
      dest_host: kind === 'dynamic' ? null : destHost.trim(),
      dest_port: dp,
      auto_start: autoStart,
      label: label.trim() || null
    };
    const ok = rule
      ? await forwards.update(hostId, rule.id, fields)
      : !!(await forwards.create({ host_id: hostId, ...fields }));
    busy = false;
    if (ok) done();
  }
</script>

<form class="fform" onsubmit={save}>
  <div class="seg" role="radiogroup" aria-label={m.fwd_title()}>
    {#each [['local', m.fwd_kind_local()], ['remote', m.fwd_kind_remote()], ['dynamic', m.fwd_kind_dynamic()]] as [k, text] (k)}
      <button
        type="button"
        role="radio"
        aria-checked={kind === k}
        onclick={() => (kind = k as ForwardKind)}>{text}</button
      >
    {/each}
  </div>
  <p class="note">{hint}</p>

  <div class="fld">
    <label for="fw-label">{m.fwd_label()}</label>
    <input id="fw-label" class="input" bind:value={label} placeholder={m.fwd_label_ph()} />
  </div>

  <fieldset>
    <legend>{kind === 'remote' ? m.fwd_bind_remote() : m.fwd_bind_local()}</legend>
    <div class="pair">
      <div class="fld">
        <label for="fw-baddr">{m.fwd_bind_addr()}</label>
        <input id="fw-baddr" class="input mono" bind:value={bindAddr} spellcheck="false" />
      </div>
      <div class="fld">
        <label for="fw-bport">{m.fwd_port()}</label>
        <input
          id="fw-bport"
          class="input mono"
          inputmode="numeric"
          bind:value={bindPort}
          placeholder="0"
          title={m.fwd_port_auto()}
        />
      </div>
    </div>
  </fieldset>

  {#if kind !== 'dynamic'}
    <fieldset>
      <legend>{kind === 'remote' ? m.fwd_dest_remote() : m.fwd_dest_local()}</legend>
      <div class="pair">
        <div class="fld">
          <label for="fw-dhost">{m.fwd_dest_host()}</label>
          <input id="fw-dhost" class="input mono" bind:value={destHost} spellcheck="false" />
        </div>
        <div class="fld">
          <label for="fw-dport">{m.fwd_port()}</label>
          <input id="fw-dport" class="input mono" inputmode="numeric" bind:value={destPort} />
        </div>
      </div>
    </fieldset>
  {/if}

  {#if exposed}
    <p class="warn"><TriangleAlert size={13} />{m.fwd_exposed({ addr: bindAddr.trim() })}</p>
  {/if}

  <label class="check"><input type="checkbox" bind:checked={autoStart} />{m.fwd_auto_start()}</label
  >
  {#if error}<p class="error">{error}</p>{/if}

  <div class="acts">
    <button type="button" class="btn" onclick={done}>{m.action_cancel()}</button>
    <button type="submit" class="btn btn-primary" disabled={busy}
      >{rule ? m.action_save() : m.action_create()}</button
    >
  </div>
</form>

<style>
  .fform {
    display: grid;
    gap: 12px;
    padding: 14px;
    border: 0.5px solid var(--border);
    border-radius: var(--r-field);
    background: var(--surface);
  }
  fieldset {
    margin: 0;
    padding: 0;
    border: 0;
    display: grid;
    gap: 6px;
  }
  legend {
    padding: 0 0 6px;
    font-size: 12px;
    font-weight: 600;
  }
  .pair {
    display: grid;
    grid-template-columns: 1fr 110px;
    gap: 8px;
  }
  .warn {
    display: flex;
    gap: 6px;
    align-items: flex-start;
    margin: 0;
    font-size: 12px;
    color: var(--warn);
  }
  .warn :global(svg) {
    flex: none;
    margin-top: 2px;
  }
  .check {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 13px;
  }
  .acts {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
