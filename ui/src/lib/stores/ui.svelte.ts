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
  paletteOpen = $state(false);
  settingsOpen = $state(false);
  editor = $state.raw<EditorTarget | null>(null);
  toasts = $state.raw<Toast[]>([]);
  /** Tree rows ask to start inline rename of this key. */
  renameKey = $state<string | null>(null);
  /** Nodes awaiting delete confirmation (`f:<id>` / `h:<id>` keys). */
  deleteKeys = $state.raw<string[] | null>(null);
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
