type Side = 'left' | 'right';

export async function listenOsDrop(
  hover: (side: Side | null) => void,
  accepts: (side: Side) => boolean,
  upload: (side: Side, paths: string[]) => void
): Promise<() => void> {
  try {
    const { getCurrentWebview } = await import('@tauri-apps/api/webview');
    return await getCurrentWebview().onDragDropEvent((ev) => {
      const p = ev.payload;
      if (p.type === 'leave') {
        hover(null);
        return;
      }
      if (p.type === 'enter' || p.type === 'over' || p.type === 'drop') {
        const dpr = window.devicePixelRatio || 1;
        const el = document.elementFromPoint(p.position.x / dpr, p.position.y / dpr);
        const side = el?.closest<HTMLElement>('[data-side]')?.dataset.side as Side | undefined;
        const target = side && accepts(side) ? side : null;
        hover(p.type === 'drop' ? null : target);
        if (p.type === 'drop' && target && p.paths.length) {
          upload(target, p.paths);
        }
      }
    });
  } catch {
    return () => {};
  }
}
