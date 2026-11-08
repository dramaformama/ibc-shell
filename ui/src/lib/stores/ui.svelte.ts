/** Transient UI state: sidebar, overlays, editor target, toasts. */
import type { FolderId, HostId } from '$lib/api';

export type EditorTarget =
  | { kind: 'host'; id: HostId | null; folderId: FolderId | null }
  | { kind: 'folder'; id: FolderId | null; parentId: FolderId | null };

export interface Toast {
  id: number;
  text: string;
  tone: 'info' | 'error';
}

class UiState {
  sidebarOpen = $state(true);
  sidebarWidth = $state(264);
  inspectorOpen = $state(false);
  /** Side panel page next to the terminal. */
  panelTab = $state<'metrics' | 'snippets'>('metrics');
  /** What the main area shows when no server is selected. */
  page = $state<'overview' | 'docker' | 'domains' | 'snippets'>('overview');

  constructor() {
    try {
      this.inspectorOpen = localStorage.getItem('ibc.ui.inspectorOpen') === 'true';
      if (localStorage.getItem('ibc.ui.panelTab') === 'snippets') this.panelTab = 'snippets';
    } catch {
      /* storage unavailable */
    }
  }

  /** Opens the side panel on `tab` (closes it when that page is already shown). */
  showPanel(tab: 'metrics' | 'snippets'): void {
    if (this.inspectorOpen && this.panelTab === tab) return this.toggleInspector();
    this.panelTab = tab;
    if (!this.inspectorOpen) this.toggleInspector();
    try {
      localStorage.setItem('ibc.ui.panelTab', tab);
    } catch {
      /* storage unavailable */
    }
  }

  toggleInspector(): void {
    this.inspectorOpen = !this.inspectorOpen;
    try {
      localStorage.setItem('ibc.ui.inspectorOpen', String(this.inspectorOpen));
    } catch {
      /* storage unavailable */
    }
  }

  paletteOpen = $state(false);
  /** The "Keyboard shortcuts" window. */
  hotkeysOpen = $state(false);
  settingsOpen = $state(false);
  editor = $state.raw<EditorTarget | null>(null);
  toasts = $state.raw<Toast[]>([]);
  /** Tree rows ask to start inline rename of this key. */
  renameKey = $state<string | null>(null);
  /** Nodes awaiting delete confirmation (`f:<id>` / `h:<id>` keys). */
  deleteKeys = $state.raw<string[] | null>(null);
  /** Server (tab group key) whose close asks: keep in background or disconnect. */
  closeAsk = $state<string | null>(null);
  #seq = 0;

  openHostEditor(id: HostId | null, folderId: FolderId | null = null): void {
    this.editor = { kind: 'host', id, folderId };
  }
  openFolderEditor(id: FolderId | null, parentId: FolderId | null = null): void {
    this.editor = { kind: 'folder', id, parentId };
  }
  closeEditor(): void {
    this.editor = null;
  }

  toast(text: string, tone: Toast['tone'] = 'info'): void {
    const id = ++this.#seq;
    this.toasts = [...this.toasts, { id, text, tone }];
    setTimeout(() => this.dismiss(id), tone === 'error' ? 6000 : 3000);
  }
  dismiss(id: number): void {
    this.toasts = this.toasts.filter((t) => t.id !== id);
  }
}

export const ui = new UiState();
