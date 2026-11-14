<script lang="ts">
  import { ArrowRightLeft, Copy, Pencil, Play, Plus, Square, Trash2 } from '@lucide/svelte';
  import type { HostId, PortForward, SessionId } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { ui } from '$lib/stores/ui.svelte';
  import ForwardForm from './ForwardForm.svelte';
  import { forwards, isLoopback, route } from './forwards.svelte';

  /** A server's port forward rules, with start / stop for its open session. */
  let { hostId, sid }: { hostId: HostId; sid: SessionId | null } = $props();

  /** `'new'`, the id of the rule being edited, or null. */
  let editing = $state<string | null>(null);
  const rules = $derived(forwards.rules[hostId] ?? []);

  $effect(() => {
    void forwards.load(hostId);
  });
  $effect(() => {
    if (sid) void forwards.refresh(sid);
  });

  const kindName = (f: PortForward) =>
    f.kind === 'local'
      ? m.fwd_kind_local()
      : f.kind === 'remote'
        ? m.fwd_kind_remote()
        : m.fwd_kind_dynamic();

  async function copy(f: PortForward, port: number) {
    const host = isLoopback(f.bind_addr) ? '127.0.0.1' : f.bind_addr;
    try {
      await navigator.clipboard.writeText(`${host}:${port}`);
      ui.toast(m.fwd_copied());
    } catch {
      /* clipboard unavailable: nothing to do */
    }
  }
</script>

<section class="card fwd" aria-label={m.fwd_title()}>
  <header>
    <h3><ArrowRightLeft size={14} />{m.fwd_title()}</h3>
    {#if editing !== 'new'}
      <button class="btn btn-sm" onclick={() => (editing = 'new')}
        ><Plus size={13} />{m.fwd_add()}</button
      >
    {/if}
  </header>

  {#if editing === 'new'}
    <ForwardForm {hostId} done={() => (editing = null)} />
  {:else if rules.length === 0}
    <p class="note">{m.fwd_empty()}</p>
  {/if}

  {#if rules.length}
    <ul>
      {#each rules as f (f.id)}
        {#if editing === f.id}
          <li class="edit"><ForwardForm {hostId} rule={f} done={() => (editing = null)} /></li>
        {:else}
          {@const port = forwards.port(sid, f.id)}
          {@const failed = forwards.errors[f.id]}
          <li>
            <span
              class="dot"
              data-on={port !== null}
              data-failed={!port && failed !== undefined}
              aria-hidden="true"
            ></span>
            <div class="what">
              <div class="name">
                {f.label ?? kindName(f)}
                <span class="kind">{kindName(f)}</span>
                {#if f.auto_start}<span class="kind">{m.fwd_auto_badge()}</span>{/if}
              </div>
              <div class="route mono">{route(f, port)}</div>
              {#if port !== null}
                <div class="state ok">{m.fwd_running()}</div>
              {:else if failed !== undefined}
                <div class="state bad">{m.fwd_failed()}{failed ? `: ${failed}` : ''}</div>
              {/if}
            </div>
            <div class="acts">
              {#if port !== null && f.kind !== 'remote'}
                <button
                  class="icon-btn"
                  aria-label={m.fwd_copy()}
                  title={m.fwd_copy()}
                  onclick={() => copy(f, port)}><Copy size={14} /></button
                >
              {/if}
              {#if port !== null && sid}
                <button class="btn btn-sm" onclick={() => forwards.stop(sid, f.id)}
                  ><Square size={12} />{m.fwd_stop()}</button
                >
              {:else}
                <button
                  class="btn btn-sm"
                  disabled={!sid}
                  title={sid ? undefined : m.fwd_need_session()}
                  onclick={() => sid && forwards.start(sid, f.id)}
                  ><Play size={12} />{m.fwd_start()}</button
                >
              {/if}
              <button
                class="icon-btn"
                aria-label={m.menu_edit()}
                title={m.menu_edit()}
                onclick={() => (editing = f.id)}><Pencil size={14} /></button
              >
              <button
                class="icon-btn"
                aria-label={m.action_delete()}
                title={m.action_delete()}
                onclick={() => forwards.remove(hostId, f.id)}><Trash2 size={14} /></button
              >
            </div>
          </li>
        {/if}
      {/each}
    </ul>
  {/if}
</section>

<style>
  .fwd {
    padding: var(--card-pad);
    margin-top: var(--grid-gap);
    display: grid;
    gap: 10px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
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
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
  }
  li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 0;
    border-top: 0.5px solid var(--border);
  }
  li:first-child {
    border-top: 0;
  }
  li.edit {
    display: block;
  }
  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--border);
  }
  .dot[data-on='true'] {
    background: var(--ok);
  }
  .dot[data-failed='true'] {
    background: var(--danger);
  }
  .what {
    flex: 1;
    min-width: 0;
  }
  .name {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    font-weight: 500;
  }
  .kind {
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--hover);
    color: var(--muted);
    font-size: 11px;
    font-weight: 400;
  }
  .route {
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .state {
    font-size: 12px;
  }
  .ok {
    color: var(--ok);
  }
  .bad {
    color: var(--danger);
  }
  .acts {
    display: flex;
    align-items: center;
    gap: 4px;
  }
</style>
