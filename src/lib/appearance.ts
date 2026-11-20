import type { SettingsDto, ThemePreference } from './ipc';

/** Per-window cache so the next start paints the right theme before settings load. */
const THEME_CACHE_KEY = 'deskpilot.theme';

/**
 * Applies the theme and glass state to the document root:
 * - data-theme="light" | "dark" forces a theme; no attribute follows Windows.
 * - data-glass="true" makes the window background transparent so Mica shows through.
 */
export function applyAppearance(
  settings: Pick<SettingsDto, 'theme' | 'glassActive'>,
  root: HTMLElement = document.documentElement
): void {
  applyTheme(settings.theme, root);
  if (settings.glassActive) root.dataset.glass = 'true';
  else delete root.dataset.glass;
}

export function applyTheme(theme: ThemePreference, root: HTMLElement = document.documentElement): void {
  if (theme === 'system') delete root.dataset.theme;
  else root.dataset.theme = theme;
  try {
    localStorage.setItem(THEME_CACHE_KEY, theme);
  } catch {
    // Storage can be unavailable; the theme still applies for this session.
  }
}

/** Applies the cached theme, if any. Called before the first render. */
export function applyCachedTheme(root: HTMLElement = document.documentElement): void {
  try {
    const cached = localStorage.getItem(THEME_CACHE_KEY);
    if (cached === 'light' || cached === 'dark') root.dataset.theme = cached;
  } catch {
    // Ignore: falls back to the Windows theme until settings load.
  }
}
