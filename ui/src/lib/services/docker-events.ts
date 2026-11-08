/** Event actions of the Docker daemon: a short plain label and a colour tone (ADR-016). */
import * as m from '$lib/paraglide/messages.js';

export type EventTone = 'ok' | 'danger' | 'warn' | 'off';

const TONE: Record<string, EventTone> = {
  start: 'ok',
  restart: 'ok',
  unpause: 'ok',
  die: 'danger',
  kill: 'danger',
  oom: 'danger',
  stop: 'warn',
  pause: 'warn'
};

export const eventTone = (action: string): EventTone => TONE[action] ?? 'off';

const LABEL: Record<string, () => string> = {
  start: m.dk_ev_start,
  stop: m.dk_ev_stop,
  restart: m.dk_ev_restart,
  die: m.dk_ev_die,
  kill: m.dk_ev_kill,
  oom: m.dk_ev_oom,
  create: m.dk_ev_create,
  destroy: m.dk_ev_destroy,
  pull: m.dk_ev_pull,
  delete: m.dk_ev_destroy,
  pause: m.dk_ev_pause,
  unpause: m.dk_ev_unpause,
  connect: m.dk_ev_connect,
  disconnect: m.dk_ev_disconnect
};

/** Known actions in the user's language, anything else as the daemon wrote it. */
export const eventLabel = (action: string): string => LABEL[action]?.() ?? action;

const KIND: Record<string, () => string> = {
  container: m.dk_kind_container,
  image: m.dk_kind_image,
  volume: m.dk_kind_volume,
  network: m.dk_kind_network
};
export const kindLabel = (kind: string): string => KIND[kind]?.() ?? kind;
