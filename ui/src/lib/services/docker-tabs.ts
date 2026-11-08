import * as m from '$lib/paraglide/messages.js';

/** The six Docker views, in the order they are offered (ADR-012, ADR-016). */
export type DockerTab = 'containers' | 'compose' | 'images' | 'volumes' | 'networks' | 'events';

export const DOCKER_TABS: [DockerTab, () => string][] = [
  ['containers', m.dk_tab_containers],
  ['compose', m.dk_tab_compose],
  ['images', m.dk_tab_images],
  ['volumes', m.dk_tab_volumes],
  ['networks', m.dk_tab_networks],
  ['events', m.dk_tab_events]
];
