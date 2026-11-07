<script lang="ts">
  import { Play } from '@lucide/svelte';
  import Sheet from '$lib/components/Sheet.svelte';
  import Flag from '$lib/flags/Flag.svelte';
  import { fleet } from '$lib/fleet/fleet.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { sessions } from '$lib/stores/sessions.svelte';
  import { fillVars, parseVars } from './model';
  import { snippets, type PendingRun, type Target } from './snippets.svelte';

  let { run }: { run: PendingRun } = $props();

  // svelte-ignore state_referenced_locally
  const vars = parseVars(run.snippet.body);
  // svelte-ignore state_referenced_locally
  const many = run.mode === 'many';
  let values = $state<Record<string, string>>(
    Object.fromEntries(vars.map((v) => [v.name, v.fallback]))
  );
  // svelte-ignore state_referenced_locally
  let execute = $state(run.execute);
  const preview = $derived(fillVars(run.snippet.body, values));

  const servers = $derived(fleet.servers);
  const connected = $derived(servers.filter((s) => s.phase === 'connected').map((s) => s.id));
  const folders = $derived([...new Set(servers.map((s) => s.folder).filter(Boolean))].sort());
  // svelte-ignore state_referenced_locally
  let picked = $state<string[]>(many ? [...connected] : []);
  const count = $derived(many ? picked.length : 1);
  let q = $state('');
  /** Connected first; at most 200 rows (search narrows large catalogs). */
  const shown = $derived.by(() => {
    const needle = q.trim().toLocaleLowerCase();
    return servers
      .filter(
        (s) => !needle || `${s.label} ${s.address} ${s.folder}`.toLocaleLowerCase().includes(needle)
      )
      .sort((a, b) => Number(b.phase === 'connected') - Number(a.phase === 'connected'))
      .slice(0, 200);
  });

  function toggle(id: string) {
    picked = picked.includes(id) ? picked.filter((x) => x !== id) : [...picked, id];
  }
  function pickFolder(name: string) {
    const ids = servers.filter((s) => s.folder === name).map((s) => s.id);
    const all = ids.every((id) => picked.includes(id));
    picked = all ? picked.filter((x) => !ids.includes(x)) : [...new Set([...picked, ...ids])];
  }

  const close = () => (snippets.pending = null);

  function submit(e: Event) {
    e.preventDefault();
    if (!count) return;
    const targets: Target[] = many
      ? servers
          .filter((s) => picked.includes(s.id))
          .map((s) => ({
            tabId: s.group?.tabs.find((t) => t.kind === 'terminal')?.id ?? null,
            hostId: s.host?.id ?? null
          }))
      : [snippets.currentTarget()];
    close();
    void snippets.send(run.snippet, preview, targets, execute);
  }

  const here = $derived(sessions.active ? sessions.groupOf(sessions.active.id) : null);
</script>

<Sheet title={run.snippet.name} onclose={close}>
  <form id="run-form" class="form" onsubmit={submit}>
    {#if vars.length}
      <section>
        <h3>{m.snip_vars()}</h3>
        {#each vars as v (v.name)}
          <div class="fld">
            <label for="var-{v.name}" class="mono">{v.name}</label>
            <input
              id="var-{v.name}"
              class="input"
              bind:value={values[v.name]}
              placeholder={v.fallback}
              autocomplete="off"
              spellcheck="false"
            />
          </div>
        {/each}
      </section>
    {/if}
    <section>
      <h3>{m.snip_command()}</h3>
      <pre class="preview mono">{preview}</pre>
      <label class="check"
        ><input type="checkbox" bind:checked={execute} />{m.snip_press_enter()}</label
      >
    </section>
    {#if many}
      <section>
        <div class="pickhead">
          <h3>{m.snip_servers()}</h3>
          <button
            type="button"
            class="btn btn-sm btn-ghost"
            onclick={() => (picked = [...connected])}>{m.snip_pick_connected()}</button
          >
          <button type="button" class="btn btn-sm btn-ghost" onclick={() => (picked = [])}
            >{m.snip_pick_none()}</button
          >
        </div>
        {#if folders.length}
          <div class="chips">
            {#each folders as f (f)}
              <button
                type="button"
                class="chip"
                aria-pressed={servers
                  .filter((s) => s.folder === f)
                  .every((s) => picked.includes(s.id))}
                onclick={() => pickFolder(f)}>{f}</button
              >
            {/each}
          </div>
        {/if}
        {#if servers.length > 8}
          <input
            class="input"
            type="search"
            bind:value={q}
            placeholder={m.dash_search()}
            aria-label={m.dash_search()}
          />
        {/if}
        <ul class="servers">
          {#each shown as s (s.id)}
            <li>
              <label>
                <input
                  type="checkbox"
                  checked={picked.includes(s.id)}
                  onchange={() => toggle(s.id)}
                />
                <Flag code={s.flag} size={16} />
                <span class="nm">{s.label}</span>
                <span class="st" data-on={s.phase === 'connected'}
                  >{s.phase === 'connected' ? m.status_connected() : m.snip_will_connect()}</span
                >
              </label>
            </li>
          {/each}
        </ul>
      </section>
    {:else if here}
      <p class="where"><Flag code={here.flag} size={14} />{m.snip_runs_on({ name: here.label })}</p>
    {/if}
  </form>
  {#snippet footer()}
    <button class="btn" type="button" onclick={close}>{m.action_cancel()}</button>
    <button class="btn btn-primary" type="submit" form="run-form" disabled={!count}
      ><Play size={14} />{many ? m.snip_run_on({ count }) : m.snip_run()}</button
    >
  {/snippet}
</Sheet>

<style>
  .preview {
    margin: 0;
    max-height: 160px;
    overflow: auto;
    padding: 10px 12px;
    border-radius: var(--r-control);
    background: var(--surface-2);
    white-space: pre-wrap;
    word-break: break-all;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }
  .pickhead {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .pickhead h3 {
    flex: 1;
  }
  .servers {
    display: grid;
    margin: 0;
    padding: 0;
    list-style: none;
    max-height: 320px;
    overflow: auto;
  }
  .servers label {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 6px;
    border-radius: var(--r-control);
    cursor: pointer;
  }
  .servers label:hover {
    background: var(--hover);
  }
  .nm {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .st {
    font-size: 11px;
    color: var(--muted);
  }
  .st[data-on='true'] {
    color: var(--ok);
  }
  .where {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    color: var(--muted);
  }
</style>
