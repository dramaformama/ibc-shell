# ADR-014: no master password by default

Status: accepted, 2026-11-07. Amends ADR-003 and ADR-006 for the default behaviour.

## Context
The app runs on the owner's own Mac, nobody else uses it, and nothing leaves the machine.
A password prompt on every start (and after every idle lock) costs more than it protects.
The vault already supports `OsKeychain` mode: the DEK is wrapped by a random key that lives in the
OS keychain (macOS Keychain, Windows Credential Manager, Secret Service).

## Decision
- The UI no longer offers a master password. On first start it creates the vault in `OsKeychain`
  mode and opens it; on every later start it unlocks through the keychain without asking.
- There is no lock screen, no lock button, no idle auto-lock for a keychain vault
  (`autolock::apply_from_store` returns no timeout for it). Data is still encrypted at rest
  (XChaCha20-Poly1305); the key is tied to the user's OS login.
- Existing password vaults are migrated once: the lock screen asks for the old password a last
  time and calls the new command `vault_remove_password(password)`. It checks the password, wraps
  the same DEK with the keychain, proves the keychain round-trips, then switches the mode and
  overwrites the password wrap. Secrets are not re-encrypted. A failure leaves the password vault
  as it was.
- Backups (`.tlvault`) keep their own password: they leave the machine.
- `MasterPassword` mode, `vault_init(MasterPassword)` and `vault_change_password` stay in the
  backend and contract for later (mobile, an opt-in setting) but the desktop UI does not use them.

## Consequences
- Anyone who can use the unlocked macOS account can open the app and see the servers.
  Full-disk encryption (FileVault) and the macOS login are now the protection.
- If the keychain entry is lost (keychain reset, new Mac), the vault cannot be opened. Backups are
  the way back. The README says so.
- Builds without a paid Apple signature are ad-hoc signed, so macOS may ask once for the login
  password ("Always Allow") on the first start and after each update.
