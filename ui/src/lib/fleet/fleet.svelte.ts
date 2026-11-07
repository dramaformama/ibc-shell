/** Server-first projection shared by the overview, cards and sidebar. */
import { sessions } from '$lib/stores/sessions.svelte';
import { tree } from '$lib/stores/tree.svelte';
import { buildFleet } from './model';
import { reachability } from './reachability.svelte';

class FleetStore {
  now = $state(Date.now());
  get servers() {
    return buildFleet(
      tree.data,
      sessions.groups,
      (g) => sessions.groupPhase(g),
      reachability.data,
      this.now
    );
  }
}
export const fleet = new FleetStore();
