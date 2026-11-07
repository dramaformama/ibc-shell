<script lang="ts">
  import { RotateCcw } from '@lucide/svelte';
  import * as m from '$lib/paraglide/messages.js';

  interface Option {
    value: string;
    label: string;
  }
  interface Props {
    label: string;
    /** '' = inherit. */
    value: string;
    /** Text shown while inheriting, e.g. "inherited from Germany: root". */
    hint: string;
    type?: 'text' | 'number' | 'select' | 'textarea';
    options?: Option[];
    mono?: boolean;
    onchange: (v: string) => void;
  }
  let { label, value, hint, type = 'text', options = [], mono = false, onchange }: Props = $props();
  const id = $props.id();
  const inheriting = $derived(value === '');
</script>

<div class="field">
  <div class="head">
    <label for={id}>{label}</label>
    {#if !inheriting}
      <button type="button" class="reset" onclick={() => onchange('')} title={m.reset_to_inherit()}>
        <RotateCcw size={12} />{m.reset_to_inherit()}
      </button>
    {/if}
  </div>
  {#if type === 'select'}
    <select
      {id}
      class="input"
      class:inh={inheriting}
      {value}
      onchange={(e) => onchange(e.currentTarget.value)}
    >
      <option value="">{hint}</option>
      {#each options as o (o.value)}<option value={o.value}>{o.label}</option>{/each}
    </select>
  {:else if type === 'textarea'}
    <textarea
      {id}
      class="input"
      class:mono
      rows="3"
      placeholder={hint}
      {value}
      oninput={(e) => onchange(e.currentTarget.value)}></textarea>
  {:else}
    <input
      {id}
      class="input"
      class:mono
      type={type === 'number' ? 'number' : 'text'}
      inputmode={type === 'number' ? 'numeric' : undefined}
      autocomplete="off"
      spellcheck="false"
      placeholder={hint}
      {value}
      oninput={(e) => onchange(e.currentTarget.value)}
    />
  {/if}
</div>

<style>
  .field {
    display: grid;
    gap: 4px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 18px;
  }
  label {
    color: var(--muted);
    font-size: 12px;
  }
  .reset {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .reset:hover {
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .mono {
    font-family: var(--font-mono);
    font-size: 12px;
  }
  select.inh {
    color: var(--muted);
  }
</style>
