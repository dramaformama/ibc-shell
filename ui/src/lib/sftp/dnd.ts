/** In-app drag state (dataTransfer payloads are unreadable during dragover, so it lives here). */
export interface DragSource {
  pane: 'left' | 'right';
  names: string[];
}

export const dnd: { source: DragSource | null } = { source: null };
