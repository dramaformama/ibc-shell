<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { api } from '$lib/api';
  import * as m from '$lib/paraglide/messages.js';
  import { sessions, type Tab } from '$lib/stores/sessions.svelte';
  import { isApple } from '$lib/util/platform';
  import { Connection } from './connection.svelte';
  import { createEngine, type Engine } from './engine';
  import { terminalPrefs } from './prefs.svelte';
  import Prompts from './Prompts.svelte';
  import SearchBar from './SearchBar.svelte';
  import StatusLayer from './StatusLayer.svelte';
  import {
    binaryBytes,
    needsPasteConfirm,
    pasteLineCount,
    safeLink,
    sanitizePaste,
    sanitizeTitle,
    utf8
  } from './text';
  import { Highlighter } from './highlight';
  import { resolveTheme } from './themes';
  import { registerTerminal } from './registry';

  let { tab, active }: { tab: Tab; active: boolean } = $props();

  let xt: HTMLDivElement;
  let engine = $state.raw<Engine | null>(null);
  let themeId = $state<string | null>(null);
  let appDark = $state(false);
  let title = $state('');
  let searchOpen = $state(false);
  let link = $state<string | null>(null);
  let paste = $state<{ text: string; lines: number } | null>(null);

  const hl = new Highlighter();
  const decoder = new TextDecoder();
  // svelte-ignore state_referenced_locally
  const conn = new Connection(tab, (data, done) => {
    if (!engine) return done();
    if (!terminalPrefs.highlight) return engine.term.write(data, done);
    engine.term.write(hl.feed(decoder.decode(data, { stream: true })), done);
  });
  const theme = $derived(resolveTheme(themeId, appDark));
  const modalOpen = $derived(!!conn.prompt || !!link || !!paste);

  function focusTerm() {
    void tick().then(() => engine?.term.focus());
  }

  function refit() {
    if (!engine || !xt || xt.clientWidth === 0 || xt.clientHeight === 0) return;
    engine.fit.fit();
  }

  function requestPaste(raw: string) {
    const text = sanitizePaste(raw);
    if (!text || !engine) return;
    if (needsPasteConfirm(text)) paste = { text, lines: pasteLineCount(text) };
    else engine.term.paste(text);
  }

  async function pasteFromClipboard() {
    try {
      requestPaste(await navigator.clipboard.readText());
    } catch {
      /* clipboard denied: the native paste event still works */
    }
  }

  function onPasteEvent(e: ClipboardEvent) {
    e.preventDefault();
    e.stopPropagation();
    const text = e.clipboardData?.getData('text/plain');
    if (text) requestPaste(text);
  }

  function onKey(e: KeyboardEvent): boolean {
    if (e.type !== 'keydown' || !engine) return true;
    const chord = isApple ? e.metaKey && !e.ctrlKey : e.ctrlKey && e.shiftKey;
    if (!chord || e.altKey) return true;
    const k = e.key.toLowerCase();
    const stop = () => {
      e.preventDefault();
      return false;
    };
    if (k === 'f' && !(isApple && e.shiftKey)) {
      searchOpen = true;
      return stop();
    }
    if (k === 'c' && engine.term.hasSelection()) {
      void navigator.clipboard.writeText(engine.term.getSelection()).catch(() => {});
      return stop();
    }
    if (k === 'v' && !isApple) {
      void pasteFromClipboard();
      return stop();
    }
    if (k === '=' || k === '+') {
      terminalPrefs.zoom(1);
      return stop();
    }
    if (k === '-' || k === '_') {
      terminalPrefs.zoom(-1);
      return stop();
    }
    if (k === '0') {
      terminalPrefs.zoom('reset');
      return stop();
    }
    return true;
  }

  /** On Windows / Linux plain Ctrl+key belongs to the remote shell, not to app shortcuts. */
  function keepCtrlForShell(e: KeyboardEvent) {
    if (!isApple && e.ctrlKey && !e.shiftKey && !e.altKey && e.key !== 'Tab') e.stopPropagation();
  }

  function onInput(data: string) {
    if (conn.phase === 'disconnected') {
      if (data === '\r') void conn.reconnect();
      return;
    }
    conn.write(utf8(data));
  }

  function closeTab() {
    sessions.close(tab.id);
  }

  onMount(() => {
    let dead = false;
    let ro: ResizeObserver | undefined;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const root = document.documentElement;
    const readDark = () => (appDark = root.dataset.theme === 'dark');
    readDark();
    const mo = new MutationObserver(readDark);
    mo.observe(root, { attributes: true, attributeFilter: ['data-theme'] });

    const onExternalPaste = (e: Event) => {
      const d = (e as CustomEvent<{ tabId: string; text: string }>).detail;
      if (d?.tabId === tab.id) requestPaste(d.text);
    };
    window.addEventListener('ibc:paste', onExternalPaste);
    const unregister = registerTerminal(tab.id, {
      send: (text) => conn.write(utf8(text)),
      phase: () => conn.phase,
      focus: focusTerm
    });

    void (async () => {
      const [, resolved] = await Promise.all([
        terminalPrefs.load(),
        tab.hostId ? api.hostResolve(tab.hostId) : Promise.resolve(null)
      ]);
      if (resolved?.status === 'ok') themeId = resolved.data.term_theme.value;
      if (dead) return;
      const eng = await createEngine(xt, {
        theme: resolveTheme(themeId, appDark),
        fontSize: terminalPrefs.fontSize,
        scrollback: terminalPrefs.scrollback,
        onLink: (url) => {
          const safe = safeLink(url);
          if (safe) link = safe;
        }
      });
      if (dead) {
        eng.dispose();
        return;
      }
      engine = eng;
      eng.term.attachCustomKeyEventHandler(onKey);
      eng.term.onData(onInput);
      eng.term.onBinary((s) => conn.write(binaryBytes(s)));
      eng.term.onResize(({ cols, rows }) => conn.resize(cols, rows));
      eng.term.onTitleChange((t) => (title = sanitizeTitle(t)));
      ro = new ResizeObserver(() => {
        clearTimeout(timer);
        timer = setTimeout(refit, 50);
      });
      ro.observe(xt);
      refit();
      if (active) eng.term.focus();
      await conn.open(eng.term.cols, eng.term.rows);
    })();

    return () => {
      dead = true;
      ro?.disconnect();
      mo.disconnect();
      clearTimeout(timer);
      window.removeEventListener('ibc:paste', onExternalPaste);
      unregister();
      conn.close();
      engine?.dispose();
      engine = null;
    };
  });

  $effect(() => {
    if (engine) engine.term.options.theme = theme;
  });

  $effect(() => sessions.setPhase(tab.id, conn.phase));
  $effect(() => sessions.setSessionId(tab.id, conn.phase === 'connected' ? conn.id : null));

  $effect(() => {
    const size = terminalPrefs.fontSize;
    if (!engine || engine.term.options.fontSize === size) return;
    engine.term.options.fontSize = size;
    refit();
  });

  $effect(() => {
    if (!active || !engine) return;
    void tick().then(() => {
      refit();
      if (!modalOpen && !searchOpen) engine?.term.focus();
    });
  });

  // Hand focus back to the terminal when a dialog goes away.
  let wasModal = false;
  $effect(() => {
    if (wasModal && !modalOpen && active) focusTerm();
    wasModal = modalOpen;
  });

  function closeSearch() {
    searchOpen = false;
    focusTerm();
  }

  function answerLink(open: boolean) {
    const url = link;
    link = null;
    if (open && url) void api.openExternal(url);
  }

  function answerPaste(confirm: boolean) {
    const p = paste;
    paste = null;
    if (confirm && p) engine?.term.paste(p.text);
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="pane"
  class:active
  role="region"
  aria-label={m.term_area({ name: title || tab.label })}
  style:--t-bg={theme.background}
  style:--t-fg={theme.foreground}
  onpastecapture={onPasteEvent}
  onkeydown={keepCtrlForShell}
>
  <div
    bind:this={xt}
    class="xt"
    class:hidden={conn.phase === 'connecting' || conn.phase === 'failed'}
    inert={modalOpen}
  ></div>
  {#if searchOpen && engine}
    <SearchBar search={engine.search} onclose={closeSearch} />
  {/if}
  <StatusLayer {conn} name={tab.label} onclosetab={closeTab} />
  <Prompts {conn} {link} {paste} onlink={answerLink} onpaste={answerPaste} onclosetab={closeTab} />
</div>

<style>
  .pane {
    position: absolute;
    inset: 0;
    display: none;
    overflow: hidden;
    background: var(--t-bg);
    color: var(--t-fg);
  }
  .pane.active {
    display: block;
  }
  /* The gap is the inset, not padding: FitAddon measures the parent's border-box height and
     would count padding as free room, adding a row that spills past the bottom edge. */
  .xt {
    position: absolute;
    inset: 8px;
  }
  .xt.hidden {
    visibility: hidden;
  }
  .xt :global(.xterm) {
    height: 100%;
  }
  .xt :global(.xterm-viewport) {
    background: transparent !important;
    scrollbar-width: thin;
    scrollbar-color: color-mix(in oklab, var(--t-fg) 25%, transparent) transparent;
  }
</style>
