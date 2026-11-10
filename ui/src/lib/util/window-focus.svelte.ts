/** Whether the app window has keyboard focus (the user is looking at it), shared app-wide. */
class WindowFocus {
  focused = $state(typeof document === 'undefined' || document.hasFocus());

  constructor() {
    if (typeof window === 'undefined') return;
    window.addEventListener('focus', () => (this.focused = true));
    window.addEventListener('blur', () => (this.focused = false));
  }
}

export const windowFocus = new WindowFocus();
