/** Maps API errors to localized, user-facing text (never includes secrets). */
import type { ApiError } from '$lib/api';
import * as m from '$lib/paraglide/messages.js';

export function errorText(e: ApiError): string {
  switch (e.code) {
    case 'VaultLocked':
      return m.err_vault_locked();
    case 'WrongPassword':
      return m.err_wrong_password();
    case 'NotFound':
      return m.err_not_found();
    case 'Validation':
      return m.err_validation({ detail: e.detail });
    case 'Cycle':
      return m.err_cycle();
    case 'TooDeep':
      return m.err_too_deep();
    case 'Network':
    case 'Timeout':
      return m.err_network();
    default:
      return m.err_generic();
  }
}
