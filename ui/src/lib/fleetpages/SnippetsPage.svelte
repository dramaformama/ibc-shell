<script lang="ts">
  import { scrollEdge } from '$lib/util/scroll-edge.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import SnippetsPanel from '$lib/snippets/SnippetsPanel.svelte';
  import './page.css';

  /** The snippet library without opening a server; running asks where to send. */
</script>

<section class="fpage" aria-label={m.panel_snippets()} use:scrollEdge>
  <header>
    <h1>{m.panel_snippets()}</h1>
    <p class="subtitle">{m.fleet_snip_hint()}</p>
  </header>
  <div class="lib"><SnippetsPanel /></div>
</section>

<style>
  .lib {
    max-width: 760px;
  }
</style>
