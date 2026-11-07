<script lang="ts">
  import '$lib/stats/stats-cards.css';
  import CertsCard from './CertsCard.svelte';
  import DockerCard from './DockerCard.svelte';

  /** Docker and certificates under the metrics of the Status page (ADR-012). */
  let { sid, compact = false }: { sid: string; compact?: boolean } = $props();
</script>

<div class="cards svc" class:compact>
  <DockerCard {sid} />
  <CertsCard {sid} />
</div>

<style>
  .svc {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    margin-top: 16px;
  }
  .svc :global(.card) {
    padding: 18px 20px;
  }
  /* Side panel: same cards as its metrics, stacked. */
  .svc.compact {
    gap: 14px;
    margin-top: 14px;
  }
  .svc.compact :global(.card) {
    flex-basis: 100%;
    padding: 16px 18px;
  }
  .svc :global(.dk) {
    flex: 2 1 560px;
  }
  .svc :global(.ssl) {
    flex: 1 1 320px;
  }
</style>
