import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

/** Event names shared with the Rust side (core::launcher). */
export const NAVIGATE_EVENT = 'deskpilot://navigate';
export const LAUNCHER_SHOWN_EVENT = 'deskpilot://launcher-shown';
export const SETTINGS_CHANGED_EVENT = 'deskpilot://settings-changed';

/**
 * True only inside the Tauri webview. Event and window APIs are skipped elsewhere
 * (unit tests, a plain browser preview) instead of throwing.
 */
export function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/**
 * Subscribes to a Tauri event. Returns an unsubscribe function that is safe to call
 * immediately, even before the subscription has finished setting up.
 */
export function onTauriEvent<T>(event: string, handler: (payload: T) => void): () => void {
  if (!isTauriRuntime()) return () => {};
  let unlisten: (() => void) | null = null;
  let cancelled = false;
  listen<T>(event, (e) => handler(e.payload))
    .then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    })
    .catch(() => {});
  return () => {
    cancelled = true;
    unlisten?.();
  };
}

/** Calls `handler(false)` when this window loses focus and `handler(true)` when it gains it. */
export function onWindowFocusChange(handler: (focused: boolean) => void): () => void {
  if (!isTauriRuntime()) return () => {};
  let unlisten: (() => void) | null = null;
  let cancelled = false;
  getCurrentWindow()
    .onFocusChanged(({ payload }) => handler(payload))
    .then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    })
    .catch(() => {});
  return () => {
    cancelled = true;
    unlisten?.();
  };
}
