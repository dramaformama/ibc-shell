/**
 * Scroll edge: when a page scrolls, the toolbar shows its hairline and the content fades softly
 * under it (no hard cut). Pages with their own sticky glass header pass `fade = false`.
 */
export const edge = $state({ scrolled: false, fade: true });

export function scrollEdge(node: HTMLElement, fade = true) {
  const sync = () => (edge.scrolled = node.scrollTop > 2);
  edge.fade = fade;
  sync();
  node.addEventListener('scroll', sync, { passive: true });
  return {
    update(next: boolean) {
      edge.fade = next;
    },
    destroy() {
      node.removeEventListener('scroll', sync);
      edge.scrolled = false;
      edge.fade = true;
    }
  };
}
