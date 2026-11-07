/**
 * In-memory mock of EVERY command in docs/contracts/ipc.md, with signatures identical to the
 * generated bindings (`mockApi: Api` is checked by `pnpm -C ui check`; coverage against
 * ipc.md by `node scripts/check-mock-coverage.mjs`).
 *
 * Seed: folders Germany (de) › Databases, Netherlands (nl), Finland (fi); 12 hosts; 2 identities.
 * Vault: starts Locked, password `ibc-shell` (`?vault=new` → onboarding). Terminal: echo shell.
 */
import * as catalog from './mock/catalog';
import * as geo from './mock/geo';
import * as sftp from './mock/sftp';
import * as stats from './mock/stats';
import * as term from './mock/terminal';
import * as tree from './mock/tree';
import * as vault from './mock/vault';
import type { Api } from './types';

export { mockEvents } from './mock/vault';
export { resetMockDb } from './mock/state';

/**
 * Outside Tauri, `new Channel()` needs `window.__TAURI_INTERNALS__.transformCallback`.
 * Install a no-op shim so UI code can create channels; the mock delivers messages by calling
 * `channel.onmessage(...)` directly.
 */
function installChannelShim(): void {
  if (typeof window === 'undefined') return;
  const w = window as unknown as { __TAURI_INTERNALS__?: Record<string, unknown> };
  const internals = (w.__TAURI_INTERNALS__ ??= {});
  let next = 1;
  internals.transformCallback ??= () => next++;
  internals.unregisterCallback ??= () => undefined;
}
installChannelShim();

export const mockApi: Api = {
  // store / tree
  treeList: tree.treeList,
  folderCreate: tree.folderCreate,
  folderUpdate: tree.folderUpdate,
  folderMove: tree.folderMove,
  folderDelete: tree.folderDelete,
  hostCreate: tree.hostCreate,
  hostUpdate: tree.hostUpdate,
  hostDuplicate: tree.hostDuplicate,
  hostDelete: tree.hostDelete,
  hostMove: tree.hostMove,
  hostResolve: tree.hostResolve,
  hostSearch: tree.hostSearch,
  identityList: catalog.identityList,
  identityCreate: catalog.identityCreate,
  identityUpdate: catalog.identityUpdate,
  identityDelete: catalog.identityDelete,
  identityImportFile: catalog.identityImportFile,
  tagList: catalog.tagList,
  tagCreate: catalog.tagCreate,
  tagUpdate: catalog.tagUpdate,
  tagDelete: catalog.tagDelete,
  snippetList: catalog.snippetList,
  snippetCreate: catalog.snippetCreate,
  snippetUpdate: catalog.snippetUpdate,
  snippetDelete: catalog.snippetDelete,
  forwardList: catalog.forwardList,
  forwardCreate: catalog.forwardCreate,
  forwardUpdate: catalog.forwardUpdate,
  forwardDelete: catalog.forwardDelete,
  knownHostList: catalog.knownHostList,
  knownHostDelete: catalog.knownHostDelete,
  settingsGet: catalog.settingsGet,
  settingsSet: catalog.settingsSet,
  // vault
  vaultStatus: vault.vaultStatus,
  vaultInit: vault.vaultInit,
  vaultUnlock: vault.vaultUnlock,
  vaultLock: vault.vaultLock,
  vaultChangePassword: vault.vaultChangePassword,
  vaultExport: vault.vaultExport,
  vaultImport: vault.vaultImport,
  // sessions
  sessionOpen: term.sessionOpen,
  sessionOpenAdhoc: term.sessionOpenAdhoc,
  shellWrite: term.shellWrite,
  shellAck: term.shellAck,
  shellResize: term.shellResize,
  sessionClose: term.sessionClose,
  sessionReconnect: term.sessionReconnect,
  hostkeyAccept: term.hostkeyAccept,
  kbdInteractiveReply: term.kbdInteractiveReply,
  passphraseReply: term.passphraseReply,
  // sftp / local / transfers
  sftpOpen: sftp.sftpOpen,
  sftpClose: sftp.sftpClose,
  sftpList: sftp.sftpList,
  sftpStat: sftp.sftpStat,
  sftpMkdir: sftp.sftpMkdir,
  sftpRename: sftp.sftpRename,
  sftpRemove: sftp.sftpRemove,
  sftpChmod: sftp.sftpChmod,
  sftpRealpath: sftp.sftpRealpath,
  localHome: sftp.localHome,
  localList: sftp.localList,
  localMkdir: sftp.localMkdir,
  localRename: sftp.localRename,
  localRemove: sftp.localRemove,
  transferEnqueue: sftp.transferEnqueue,
  transferPause: sftp.transferPause,
  transferResume: sftp.transferResume,
  transferCancel: sftp.transferCancel,
  transferRetry: sftp.transferRetry,
  transferConflictReply: sftp.transferConflictReply,
  transfersSubscribe: sftp.transfersSubscribe,
  // stats
  statsSample: stats.statsSample,
  hostProbe: stats.hostProbe,
  hostLookupIp: geo.hostLookupIp,
  // app
  appInfo: vault.appInfo,
  openExternal: vault.openExternal
};
