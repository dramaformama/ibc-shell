/**
 * Mock: SSH sessions with a fake echo terminal.
 * Hosts whose label contains "new key" ask TOFU, "changed key" fail with HostKeyChanged,
 * "2FA" ask keyboard-interactive. Shell commands: help, colors, whites, clear, flood, exit.
 */
import type { Channel } from '@tauri-apps/api/core';
import type { Commands, RawCommands, SessionEvent } from '../types';
import { db, delay, notFound, now, ok, uuid } from './state';

interface Session {
  id: string;
  label: string;
  user: string;
  hostname: string;
  onData: Channel<ArrayBuffer>;
  onEvent: Channel<SessionEvent>;
  line: string;
  open: boolean;
}

const sessions = new Map<string, Session>();
const pending = new Map<string, { sessionId: string; resolve: (v: unknown) => void }>();
const enc = new TextEncoder();
const dec = new TextDecoder();

function out(s: Session, text: string): void {
  if (s.open) s.onData.onmessage(enc.encode(text).buffer as ArrayBuffer);
}
const event = (s: Session, e: SessionEvent) => s.onEvent.onmessage(e);
const prompt = (s: Session) => out(s, `\x1b[1;32m${s.user}@${s.label}\x1b[0m:\x1b[1;34m~\x1b[0m$ `);

function ask<T>(s: Session, e: (pendingId: string) => SessionEvent): Promise<T | null> {
  const id = uuid();
  return new Promise((resolve) => {
    pending.set(id, { sessionId: s.id, resolve: resolve as (v: unknown) => void });
    event(s, e(id));
  });
}

function banner(s: Session): void {
  out(s, '\x1b[2J\x1b[H');
  out(s, `\x1b[1;36m  iBC Shell mock terminal\x1b[0m  \x1b[2m(no real SSH)\x1b[0m\r\n`);
  out(
    s,
    `  host \x1b[33m${s.hostname}\x1b[0m · user \x1b[35m${s.user}\x1b[0m · type \x1b[1mhelp\x1b[0m\r\n\r\n`
  );
  prompt(s);
}

/** Demo failures by name, so every explanation can be seen: `dns`, `refused`, `timeout`,
 * `dropped`, `badauth`, `slow` (a slow but successful connect). */
async function scripted(s: Session): Promise<boolean> {
  const key = `${s.label} ${s.hostname}`;
  const step = async (
    name: 'Resolving' | 'Connecting' | 'Handshake' | 'Authenticating',
    ms = 160
  ) => {
    event(s, { type: 'Connecting', step: name, hop: s.label });
    await delay(ms);
  };
  if (key.includes('dns')) {
    await step('Resolving');
    event(s, { type: 'Failed', error: { code: 'Network', detail: 'Uncategorized' } });
    return true;
  }
  if (key.includes('refused')) {
    await step('Resolving');
    await step('Connecting');
    event(s, {
      type: 'Attempt',
      hop: s.label,
      addr: '203.0.113.7:22',
      failure: 'ConnectionRefused'
    });
    event(s, { type: 'Failed', error: { code: 'Network', detail: 'ConnectionRefused' } });
    return true;
  }
  if (key.includes('timeout')) {
    await step('Resolving');
    await step('Connecting', 2600);
    event(s, { type: 'Attempt', hop: s.label, addr: '[2001:db8::7]:22', failure: 'Timeout' });
    event(s, { type: 'Attempt', hop: s.label, addr: '203.0.113.7:22', failure: 'Timeout' });
    event(s, { type: 'Failed', error: { code: 'Timeout' } });
    return true;
  }
  if (key.includes('dropped')) {
    await step('Resolving');
    await step('Connecting');
    event(s, { type: 'Attempt', hop: s.label, addr: '203.0.113.7:22', failure: null });
    await step('Handshake');
    event(s, { type: 'Failed', error: { code: 'Network', detail: 'ConnectionReset' } });
    return true;
  }
  if (key.includes('badauth')) {
    await step('Resolving');
    await step('Connecting');
    event(s, { type: 'Attempt', hop: s.label, addr: '203.0.113.7:22', failure: null });
    await step('Handshake');
    await step('Authenticating');
    event(s, {
      type: 'Failed',
      error: { code: 'AuthFailed', detail: { methods_left: ['publickey', 'password'] } }
    });
    return true;
  }
  if (key.includes('slow')) {
    await step('Resolving');
    await step('Connecting', 3200);
    event(s, { type: 'Attempt', hop: s.label, addr: '203.0.113.7:22', failure: null });
    return false;
  }
  return false;
}

async function connect(s: Session): Promise<void> {
  if (await scripted(s)) return;
  const steps = ['Resolving', 'Connecting', 'Handshake'] as const;
  for (const step of steps) {
    event(s, { type: 'Connecting', step, hop: s.label });
    if (step === 'Connecting')
      event(s, { type: 'Attempt', hop: s.label, addr: `${s.hostname}:22`, failure: null });
    await delay(120);
  }
  if (s.label.includes('changed key')) {
    const kh = db.knownHosts.find((k) => k.host_pattern === s.hostname);
    event(s, {
      type: 'Failed',
      error: {
        code: 'HostKeyChanged',
        detail: {
          host: s.hostname,
          port: 22,
          key_type: 'ssh-ed25519',
          expected_fingerprint: kh?.fingerprint ?? 'SHA256:unknown',
          actual_fingerprint: 'SHA256:NewNewNewNewNewNewNewNewNewNewNewNewNewNewN',
          known_host_id: kh?.id ?? ''
        }
      }
    });
    return;
  }
  if (s.label.includes('new key') && !db.knownHosts.some((k) => k.host_pattern === s.hostname)) {
    const info = {
      host: s.hostname,
      port: 22,
      key_type: 'ssh-ed25519',
      fingerprint: 'SHA256:TofuTofuTofuTofuTofuTofuTofuTofuTofuTofuTof',
      randomart:
        '+--[ED25519 256]--+\n|    .o+*=.       |\n|     +=o+.       |\n+----[SHA256]-----+'
    };
    const accepted = await ask<boolean>(s, (pending_id) => ({
      type: 'HostKeyPrompt',
      pending_id,
      info
    }));
    if (!accepted) return;
  }
  event(s, { type: 'Connecting', step: 'Authenticating', hop: s.label });
  if (s.label.includes('2FA')) {
    const answers = await ask<string[]>(s, (pending_id) => ({
      type: 'KbdInteractive',
      pending_id,
      name: 'Two-factor',
      instruction: 'Any 6 digits work in the mock.',
      prompts: [{ text: 'Verification code: ', echo: false }]
    }));
    if (!answers || !/^\d{6}$/.test(answers[0] ?? '')) {
      event(s, {
        type: 'Failed',
        error: { code: 'AuthFailed', detail: { methods_left: ['keyboard-interactive'] } }
      });
      return;
    }
  }
  await delay(100);
  event(s, { type: 'Connected', server_banner: 'Authorized access only.' });
  const host = db.hosts.find((h) => h.label === s.label);
  if (host) host.last_connected_at = now();
  banner(s);
}

function runLine(s: Session, line: string): void {
  const cmd = line.trim();
  if (cmd === 'exit') {
    out(s, 'logout\r\n');
    event(s, { type: 'ExitStatus', code: 0 });
    event(s, { type: 'Disconnected', reason: 'Remote shell exited', can_reconnect: true });
    return;
  }
  if (cmd === 'whites') {
    // Colours that real tools print assuming a dark screen.
    out(
      s,
      '\x1b[1;97mService  IP  HTTP\x1b[0m\r\n\x1b[37mwhite 37\x1b[0m \x1b[97mbright 97\x1b[0m '
    );
    out(
      s,
      '\x1b[2;97mdim 2;97\x1b[0m \x1b[38;5;255mxterm 255\x1b[0m \x1b[38;2;255;255;255mrgb white\x1b[0m\r\n'
    );
    return prompt(s);
  }
  if (cmd === 'help')
    out(s, 'commands: help, colors, whites, clear, flood, exit — anything else is echoed\r\n');
  else if (cmd === 'clear') out(s, '\x1b[2J\x1b[H');
  else if (cmd === 'colors') {
    for (let i = 0; i < 16; i++) out(s, `\x1b[48;5;${i}m  ${String(i).padStart(2)}  \x1b[0m`);
    out(s, '\r\n');
  } else if (cmd === 'flood') {
    for (let i = 0; i < 5000; i++) out(s, `line ${i} \x1b[3${i % 8}m████████████████\x1b[0m\r\n`);
  } else if (cmd) out(s, `${cmd}\r\n`);
  prompt(s);
}

export const sessionOpen: Commands['sessionOpen'] = async (
  hostId,
  _cols,
  _rows,
  onData,
  onEvent
) => {
  await delay();
  const h = db.hosts.find((x) => x.id === hostId);
  if (!h) return notFound();
  const s: Session = {
    id: uuid(),
    label: h.label,
    user: h.username ?? 'root',
    hostname: h.hostname,
    onData,
    onEvent,
    line: '',
    open: true
  };
  sessions.set(s.id, s);
  void connect(s);
  return ok(s.id);
};

export const sessionOpenAdhoc: Commands['sessionOpenAdhoc'] = async (
  target,
  _cols,
  _rows,
  onData,
  onEvent
) => {
  await delay();
  const s: Session = {
    id: uuid(),
    label: target.hostname,
    user: target.user ?? 'root',
    hostname: target.hostname,
    onData,
    onEvent,
    line: '',
    open: true
  };
  sessions.set(s.id, s);
  void connect(s);
  return ok(s.id);
};

export const shellWrite: RawCommands['shellWrite'] = async (sessionId, data) => {
  const s = sessions.get(sessionId);
  if (!s) return notFound();
  for (const ch of dec.decode(data)) {
    if (ch === '\r') {
      out(s, '\r\n');
      runLine(s, s.line);
      s.line = '';
    } else if (ch === '\x7f') {
      if (s.line) {
        s.line = s.line.slice(0, -1);
        out(s, '\b \b');
      }
    } else if (ch === '\x03') {
      s.line = '';
      out(s, '^C\r\n');
      prompt(s);
    } else if (ch >= ' ') {
      s.line += ch;
      out(s, ch);
    }
  }
  return ok(null);
};

export const shellAck: Commands['shellAck'] = async (sessionId) =>
  sessions.has(sessionId) ? ok(null) : notFound();

export const shellResize: Commands['shellResize'] = async (sessionId) =>
  sessions.has(sessionId) ? ok(null) : notFound();

export const sessionClose: Commands['sessionClose'] = async (sessionId) => {
  const s = sessions.get(sessionId);
  if (!s) return notFound();
  for (const [id, p] of pending) {
    if (p.sessionId !== sessionId) continue;
    pending.delete(id);
    p.resolve(null);
  }
  s.open = false;
  sessions.delete(sessionId);
  return ok(null);
};

export const sessionReconnect: Commands['sessionReconnect'] = async (sessionId) => {
  const s = sessions.get(sessionId);
  if (!s) return notFound();
  s.open = true;
  void connect(s);
  return ok(null);
};

function reply(pendingId: string, value: unknown) {
  const p = pending.get(pendingId);
  if (!p) return notFound();
  pending.delete(pendingId);
  p.resolve(value);
  return ok(null);
}

export const hostkeyAccept: Commands['hostkeyAccept'] = async (pendingId, remember) => {
  const p = pending.get(pendingId);
  const s = p && sessions.get(p.sessionId);
  if (s && remember) {
    db.knownHosts.push({
      id: uuid(),
      host_pattern: s.hostname,
      key_type: 'ssh-ed25519',
      fingerprint: 'SHA256:TofuTofuTofuTofuTofuTofuTofuTofuTofuTofuTof',
      added_at: now()
    });
  }
  return reply(pendingId, true);
};

export const kbdInteractiveReply: Commands['kbdInteractiveReply'] = async (pendingId, answers) =>
  reply(pendingId, answers);

export const passphraseReply: Commands['passphraseReply'] = async (pendingId, passphrase) =>
  reply(pendingId, passphrase);

/** Used by mock SFTP to validate `{kind:"session"}` targets. */
export const hasSession = (id: string): boolean => sessions.has(id);
