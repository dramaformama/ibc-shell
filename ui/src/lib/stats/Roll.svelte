<script lang="ts">
  /**
   * A number whose digits roll to the new value like a counter, instead of being swapped.
   * Keyed from the right, so "9 → 10" rolls the units and slides a tens digit in.
   */
  let { value }: { value: string } = $props();
  const chars = $derived([...value]);
  const DIGITS = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
</script>

<span class="roll"
  ><span class="sr">{value}</span
  >{#each chars as c, i (chars.length - i)}{#if c >= '0' && c <= '9'}<span
        class="d"
        aria-hidden="true"
        ><span class="col" style:transform="translateY({-Number(c) * 10}%)"
          >{#each DIGITS as n (n)}<span>{n}</span>{/each}</span
        ></span
      >{:else}<span aria-hidden="true">{c}</span>{/if}{/each}</span
>

<style>
  .roll {
    position: relative;
    display: inline-flex;
    font-variant-numeric: tabular-nums;
    white-space: pre;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
  .d {
    display: inline-block;
    height: 1lh;
    overflow: hidden;
  }
  .col {
    display: flex;
    flex-direction: column;
    transition: transform 520ms var(--spring);
  }
  .col span {
    height: 1lh;
  }
</style>
