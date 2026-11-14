<script lang="ts">
  import Skeleton from '$lib/components/Skeleton.svelte';
  import type { Snippet } from 'svelte';
  import type { DockerAccess } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';

  /** Loading / no-access notes shared by the Docker resource tabs; `children` draws the rows. */
  let {
    access,
    count,
    none,
    children
  }: {
    access: DockerAccess | null;
    count: number;
    none: string;
    children: Snippet;
  } = $props();
</script>

{#if access === null}
  <Skeleton rows={3} label={m.dk_res_loading()} />
{:else if access === 'Missing'}
  <p class="note">{m.dk_missing()}</p>
{:else if access === 'Denied'}
  <p class="note">{m.dk_denied()}</p>
{:else if access === 'Down'}
  <p class="note">{m.dk_down()}</p>
{:else if count === 0}
  <p class="note">{none}</p>
{:else}
  {@render children()}
{/if}

<style>
  .note {
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.45;
  }
</style>
