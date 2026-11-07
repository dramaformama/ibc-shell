#!/usr/bin/env node
// Gate G0: every command in docs/contracts/ipc.md must be implemented in ui/src/lib/api/mock.ts
// (as a key of `mockApi`), and mock.ts must not invent commands absent from the contract.
// Commands are the backticked snake_case names in the first column of ipc.md tables.
// Exit 1 on any gap. Usage: node scripts/check-mock-coverage.mjs
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const ipc = readFileSync(join(root, 'docs/contracts/ipc.md'), 'utf8');
const mock = readFileSync(join(root, 'ui/src/lib/api/mock.ts'), 'utf8');

const camel = (s) => s.replace(/_([a-z0-9])/g, (_, c) => c.toUpperCase());

const contract = new Set();
for (const line of ipc.split('\n')) {
  const m = /^\|\s*`([a-z][a-z0-9_]*)`\s*\|/.exec(line);
  if (m) contract.add(m[1]);
}

const body = /export const mockApi[^=]*=\s*\{([\s\S]*?)\n\};/.exec(mock);
if (!body) {
  console.error('check-mock-coverage: `export const mockApi = { ... };` not found in mock.ts');
  process.exit(1);
}
const mocked = new Set([...body[1].matchAll(/^\s*([a-zA-Z0-9]+)\s*:/gm)].map((m) => m[1]));

const missing = [...contract].filter((c) => !mocked.has(camel(c)));
const extra = [...mocked].filter((m) => ![...contract].some((c) => camel(c) === m));

if (contract.size === 0) {
  console.error('check-mock-coverage: no commands parsed from ipc.md');
  process.exit(1);
}
if (missing.length || extra.length) {
  if (missing.length) console.error(`missing in mock.ts: ${missing.join(', ')}`);
  if (extra.length) console.error(`not in ipc.md: ${extra.join(', ')}`);
  process.exit(1);
}
console.log(`mock coverage OK: ${contract.size}/${contract.size} ipc.md commands`);
