<script lang="ts">
  import { api, type FolderId, type IdentityDto } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { tree } from '$lib/stores/tree.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { TERM_THEMES, type DefaultsForm } from '$lib/util/defaults-form';
  import { errorText } from '$lib/util/errors';
  import { inheritedDefault } from '$lib/util/tree-model';
  import InheritField from './InheritField.svelte';

  interface Props {
    form: DefaultsForm;
    /** Folder whose chain provides the inherited values (host's folder / folder's parent). */
    inheritFrom: FolderId | null;
    /** Host being edited (excluded from jump choices). */
    selfHostId?: string | null;
    /** Render only the advanced fields (the host editor owns login / password itself). */
    advancedOnly?: boolean;
  }
  let {
    form = $bindable(),
    inheritFrom,
    selfHostId = null,
    advancedOnly = false
  }: Props = $props();

  let identities = $state.raw<IdentityDto[]>([]);
  void api.identityList().then((r) => r.status === 'ok' && (identities = r.data));

  let creating = $state(false);
  let draft = $state({ label: '', username: '', password: '' });
  let busy = $state(false);

  const identityName = (id: string) => identities.find((i) => i.id === id)?.label ?? id;
  const hostName = (id: string) => tree.host(id)?.label ?? id;

  type K =
    | 'username'
    | 'port'
    | 'identity_id'
    | 'jump_host_id'
    | 'startup_cmd'
    | 'keepalive_s'
    | 'term_theme';
  function hint(field: K, fmt: (v: never) => string, fallback?: string): string {
    const r = inheritedDefault(tree.data, inheritFrom, field);
    if (r) return m.inherited_from({ name: r.from ?? '', value: fmt(r.value as never) });
    return fallback ? m.inherited_default({ value: fallback }) : m.inherited_none();
  }
  const str = (v: unknown) => String(v);

  const themeName = (id: string) => TERM_THEMES.find((t) => t.id === id)?.name ?? id;

  async function createIdentity() {
    if (!draft.label.trim()) return;
    busy = true;
    const r = await api.identityCreate({
      label: draft.label.trim(),
      username: draft.username.trim() || null,
      kind: 'password',
      password: draft.password || null
    });
    busy = false;
    draft.password = '';
    if (r.status === 'error') return ui.toast(errorText(r.error), 'error');
    identities = [...identities, r.data];
    form.identity = r.data.id;
    creating = false;
    draft = { label: '', username: '', password: '' };
  }
</script>

{#if !advancedOnly}
  <section>
    <h3>{m.section_connection()}</h3>
    <InheritField
      label={m.field_port()}
      type="number"
      value={form.port}
      hint={hint('port', str, '22')}
      onchange={(v) => (form.port = v)}
    />
    <InheritField
      label={m.field_username()}
      value={form.username}
      hint={hint('username', str)}
      onchange={(v) => (form.username = v)}
    />
    <InheritField
      label={m.field_identity()}
      type="select"
      value={form.identity}
      hint={hint('identity_id', (v: string) => identityName(v))}
      options={identities.map((i) => ({ value: i.id, label: i.label }))}
      onchange={(v) => (form.identity = v)}
    />
    {#if creating}
      <div class="inline">
        <input
          class="input"
          placeholder={m.field_identity_label()}
          bind:value={draft.label}
          aria-label={m.field_identity_label()}
        />
        <input
          class="input"
          placeholder={m.field_username()}
          bind:value={draft.username}
          aria-label={m.field_username()}
          autocomplete="off"
        />
        <input
          class="input"
          type="password"
          placeholder={m.field_password()}
          bind:value={draft.password}
          aria-label={m.field_password()}
          autocomplete="new-password"
        />
        <div class="row">
          <button type="button" class="btn btn-ghost" onclick={() => (creating = false)}
            >{m.action_cancel()}</button
          >
          <button
            type="button"
            class="btn btn-primary"
            disabled={busy || !draft.label.trim()}
            onclick={createIdentity}>{m.action_create()}</button
          >
        </div>
      </div>
    {:else}
      <button type="button" class="link" onclick={() => (creating = true)}
        >{m.identity_new()}</button
      >
    {/if}
  </section>
{/if}

{#snippet advFields()}
  <div class="adv">
    <InheritField
      label={m.field_jump_host()}
      type="select"
      value={form.jump}
      hint={hint('jump_host_id', (v: string) => hostName(v))}
      options={tree.data.hosts
        .filter((h) => h.id !== selfHostId)
        .map((h) => ({ value: h.id, label: h.label }))}
      onchange={(v) => (form.jump = v)}
    />
    <InheritField
      label={m.field_startup()}
      mono
      value={form.startup}
      hint={hint('startup_cmd', str)}
      onchange={(v) => (form.startup = v)}
    />
    <InheritField
      label={m.field_keepalive()}
      type="number"
      value={form.keepalive}
      hint={hint('keepalive_s', str, '30')}
      onchange={(v) => (form.keepalive = v)}
    />
    <InheritField
      label={m.field_term_theme()}
      type="select"
      value={form.theme}
      hint={hint('term_theme', (v: string) => themeName(v))}
      options={TERM_THEMES.map((t) => ({ value: t.id, label: t.name }))}
      onchange={(v) => (form.theme = v)}
    />
  </div>
{/snippet}

{#if advancedOnly}
  {@render advFields()}
{:else}
  <details>
    <summary>{m.section_advanced()}</summary>
    {@render advFields()}
  </details>
{/if}

<style>
  section,
  .adv {
    display: grid;
    gap: 12px;
  }
  h3 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }
  details summary {
    cursor: pointer;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
    list-style: none;
    padding-bottom: 12px;
  }
  details summary::-webkit-details-marker {
    display: none;
  }
  details summary::before {
    content: '+ ';
  }
  details[open] summary::before {
    content: '− ';
  }
  .inline {
    display: grid;
    gap: 8px;
    padding: 12px;
    border-radius: var(--r-card);
    background: var(--surface-2);
  }
  .row {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .link {
    justify-self: start;
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    font: inherit;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
    text-underline-offset: 2px;
  }
</style>
