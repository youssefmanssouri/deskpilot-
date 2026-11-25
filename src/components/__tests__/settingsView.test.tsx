// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, cleanup, act } from '@testing-library/react';
import * as tauriCore from '@tauri-apps/api/core';
import { SettingsView, shortcutFromKeyEvent } from '../SettingsView';
import { applyAppearance, applyCachedTheme } from '../../lib/appearance';
import type { SettingsDto } from '../../lib/ipc';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const base: SettingsDto = {
  theme: 'system',
  historyEnabled: true,
  launcherShortcut: 'Ctrl+Alt+Space',
  launcherShortcutLabel: 'Ctrl+Alt+Space',
  shortcutRegistered: true,
  shortcutMessage: null,
  startWithWindows: false,
  startupStatus: { state: 'disabled' },
  glassEffect: false,
  glassSupported: true,
  glassActive: false,
};

const calls = (cmd: string) => vi.mocked(tauriCore.invoke).mock.calls.filter(([c]) => c === cmd);

/** Simple backend: applies each setter to a copy of `state`. */
function backend(
  initial: Partial<SettingsDto> = {},
  fail: Record<string, string | { code: string; message: string }> = {}
) {
  let state: SettingsDto = { ...base, ...initial };
  vi.mocked(tauriCore.invoke).mockImplementation(async (cmd: string, args?: unknown) => {
    const a = (args ?? {}) as Record<string, unknown>;
    if (fail[cmd]) {
      const f = fail[cmd];
      throw typeof f === 'string' ? { code: 'UNAVAILABLE', message: f } : f;
    }
    switch (cmd) {
      case 'get_settings':
        return state;
      case 'set_theme':
        state = { ...state, theme: a.theme as SettingsDto['theme'] };
        return state;
      case 'set_history_enabled':
        state = { ...state, historyEnabled: a.enabled as boolean };
        return state;
      case 'set_launcher_shortcut':
        state = { ...state, launcherShortcut: a.shortcut as string, launcherShortcutLabel: a.shortcut as string };
        return state;
      case 'set_start_with_windows': {
        const enabled = a.enabled as boolean;
        state = {
          ...state,
          startWithWindows: enabled,
          startupStatus: { state: enabled ? 'enabled' : 'disabled' },
        };
        return state;
      }
      case 'set_glass_effect':
        state = { ...state, glassEffect: a.enabled as boolean };
        return state;
      default:
        return undefined;
    }
  });
}

const key = (k: string, code: string, mods: Partial<Record<'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey', boolean>> = {}) => ({
  key: k,
  code,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...mods,
});

describe('shortcutFromKeyEvent', () => {
  it('builds a combination from modifiers and the physical key', () => {
    expect(shortcutFromKeyEvent(key('k', 'KeyK', { ctrlKey: true, altKey: true }))).toBe('Ctrl+Alt+K');
    expect(shortcutFromKeyEvent(key(' ', 'Space', { ctrlKey: true, altKey: true }))).toBe('Ctrl+Alt+Space');
    expect(shortcutFromKeyEvent(key('F13', 'F13', { metaKey: true }))).toBe('Win+F13');
    // On AZERTY the physical KeyQ key types "a": the shortcut follows the physical key.
    expect(shortcutFromKeyEvent(key('a', 'KeyQ', { ctrlKey: true }))).toBe('Ctrl+Q');
    expect(shortcutFromKeyEvent(key('&', 'Digit1', { ctrlKey: true }))).toBe('Ctrl+1');
  });

  it('returns null for modifier-only presses and unsupported keys', () => {
    expect(shortcutFromKeyEvent(key('Control', 'ControlLeft', { ctrlKey: true }))).toBeNull();
    expect(shortcutFromKeyEvent(key('Enter', 'Enter', { ctrlKey: true }))).toBeNull();
    expect(shortcutFromKeyEvent(key('F25', 'F25', { ctrlKey: true }))).toBeNull();
  });
});

describe('appearance', () => {
  afterEach(() => {
    delete document.documentElement.dataset.theme;
    delete document.documentElement.dataset.glass;
    localStorage.clear();
  });

  it('sets or clears data-theme and data-glass, and caches the theme', () => {
    applyAppearance({ theme: 'dark', glassActive: true });
    expect(document.documentElement.dataset.theme).toBe('dark');
    expect(document.documentElement.dataset.glass).toBe('true');

    applyAppearance({ theme: 'system', glassActive: false });
    expect(document.documentElement.dataset.theme).toBeUndefined();
    expect(document.documentElement.dataset.glass).toBeUndefined();

    applyAppearance({ theme: 'light', glassActive: false });
    delete document.documentElement.dataset.theme;
    applyCachedTheme();
    expect(document.documentElement.dataset.theme).toBe('light');
  });
});

describe('SettingsView', () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => {
    cleanup();
    delete document.documentElement.dataset.theme;
    localStorage.clear();
  });

  it('changes the theme and applies it immediately', async () => {
    backend();
    render(<SettingsView />);
    const dark = await screen.findByRole('radio', { name: 'Dark' });
    expect(screen.getByRole('radio', { name: 'Use Windows setting' }).getAttribute('aria-checked')).toBe('true');

    await act(async () => {
      fireEvent.click(dark);
    });
    expect(calls('set_theme')[0][1]).toEqual({ theme: 'dark' });
    expect(dark.getAttribute('aria-checked')).toBe('true');
    expect(document.documentElement.dataset.theme).toBe('dark');
  });

  it('records and saves a new launcher shortcut', async () => {
    backend();
    render(<SettingsView />);
    fireEvent.click(await screen.findByRole('button', { name: 'Change' }));

    const capture = screen.getByRole('textbox', { name: 'Press the new shortcut' });
    expect(document.activeElement).toBe(capture);
    fireEvent.keyDown(capture, { key: 'Control', code: 'ControlLeft', ctrlKey: true });
    expect(capture.textContent).toBe('Press the new shortcut…');
    fireEvent.keyDown(capture, { key: 'k', code: 'KeyK', ctrlKey: true, metaKey: true });
    expect(capture.textContent).toBe('Ctrl+Win+K');

    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    });
    expect(calls('set_launcher_shortcut')[0][1]).toEqual({ shortcut: 'Ctrl+Win+K' });
    expect(screen.getByRole('status').textContent).toContain('now opens with Ctrl+Win+K');
    expect(screen.getByText('Ctrl+Win+K')).toBeDefined();
  });

  it('keeps the old shortcut and explains when the new one is refused', async () => {
    backend({}, { set_launcher_shortcut: 'Ctrl+Alt+K is already used by another app or reserved by Windows. Your shortcut is still Ctrl+Alt+Space.' });
    render(<SettingsView />);
    fireEvent.click(await screen.findByRole('button', { name: 'Change' }));
    const capture = screen.getByRole('textbox', { name: 'Press the new shortcut' });
    fireEvent.keyDown(capture, { key: 'k', code: 'KeyK', ctrlKey: true, altKey: true });

    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    });
    expect(screen.getByRole('alert').textContent).toContain('Your shortcut is still Ctrl+Alt+Space');
    // Still recording, so the user can try another combination.
    expect(screen.getByRole('textbox', { name: 'Press the new shortcut' })).toBeDefined();
  });

  it('Esc cancels shortcut recording without saving', async () => {
    backend();
    render(<SettingsView />);
    fireEvent.click(await screen.findByRole('button', { name: 'Change' }));
    fireEvent.keyDown(screen.getByRole('textbox', { name: 'Press the new shortcut' }), { key: 'Escape', code: 'Escape' });
    expect(screen.queryByRole('textbox', { name: 'Press the new shortcut' })).toBeNull();
    expect(calls('set_launcher_shortcut')).toHaveLength(0);
  });

  it('turning history off clears it and tells the rest of the app', async () => {
    backend();
    const onHistoryCleared = vi.fn();
    render(<SettingsView onHistoryCleared={onHistoryCleared} />);
    const sw = await screen.findByRole('switch', { name: 'Remember recent apps' });

    await act(async () => {
      fireEvent.click(sw);
    });
    expect(calls('set_history_enabled')[0][1]).toEqual({ enabled: false });
    expect(sw.getAttribute('aria-checked')).toBe('false');
    expect(onHistoryCleared).toHaveBeenCalledTimes(1);
    expect(screen.getByRole('status').textContent).toContain('the list was deleted');
  });

  it('toggles start with Windows, refreshes from backend, and disables it when Windows cannot be checked', async () => {
    backend();
    const { unmount } = render(<SettingsView />);
    const sw = await screen.findByRole('switch', { name: 'Start with Windows' });
    expect((sw as HTMLButtonElement).disabled).toBe(false);
    expect(sw.getAttribute('aria-checked')).toBe('false');

    await act(async () => {
      fireEvent.click(sw);
    });
    expect(calls('set_start_with_windows')[0][1]).toEqual({ enabled: true });
    expect(calls('get_settings')).toHaveLength(2);
    expect(sw.getAttribute('aria-checked')).toBe('true');

    await act(async () => {
      fireEvent.click(sw);
    });
    expect(calls('set_start_with_windows')[1][1]).toEqual({ enabled: false });
    expect(calls('get_settings')).toHaveLength(3);
    expect(sw.getAttribute('aria-checked')).toBe('false');
    unmount();

    backend({ startWithWindows: null, startupStatus: { state: 'unavailable', errorCode: 'OPERATION_FAILED' } });
    render(<SettingsView />);
    const unknown = await screen.findByRole('switch', { name: 'Start with Windows' });
    expect((unknown as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByRole('alert').textContent).toContain("couldn't check the Windows startup list");
  });

  it('shows warnings and disables startup toggle for stale, otherInstall, unrecognized, and unavailable states', async () => {
    const cases: Array<{
      startupStatus: SettingsDto['startupStatus'];
      role: 'status' | 'alert';
      expectedText: RegExp;
    }> = [
      {
        startupStatus: { state: 'stale' },
        role: 'status',
        expectedText: /stale startup entry/i,
      },
      {
        startupStatus: { state: 'otherInstall' },
        role: 'status',
        expectedText: /Another DeskPilot installation/i,
      },
      {
        startupStatus: { state: 'unrecognized' },
        role: 'status',
        expectedText: /unrecognized/i,
      },
      {
        startupStatus: { state: 'unavailable', errorCode: 'PERMISSION_DENIED' },
        role: 'alert',
        expectedText: /Permission denied/i,
      },
      {
        startupStatus: { state: 'unavailable', errorCode: 'INVALID_STARTUP_STATE' },
        role: 'alert',
        expectedText: /couldn't verify the Windows startup state/i,
      },
    ];

    for (const tc of cases) {
      vi.clearAllMocks();
      backend({ startWithWindows: null, startupStatus: tc.startupStatus });
      const { unmount } = render(<SettingsView />);
      const sw = await screen.findByRole('switch', { name: 'Start with Windows' });
      expect((sw as HTMLButtonElement).disabled).toBe(true);
      expect(sw.getAttribute('aria-checked')).toBe('false');
      expect(screen.getByRole(tc.role).textContent).toMatch(tc.expectedText);

      await act(async () => {
        fireEvent.click(sw);
      });
      expect(calls('set_start_with_windows')).toHaveLength(0);
      unmount();
    }
  });

  it('maps startup mutation error codes to sanitized messages without leaking raw details', async () => {
    const errorCases: Array<{ code: string; expectedText: RegExp }> = [
      { code: 'PERMISSION_DENIED', expectedText: /Permission denied while updating/i },
      { code: 'STARTUP_CONFLICT', expectedText: /Another or stale DeskPilot startup entry/i },
      { code: 'INVALID_STARTUP_STATE', expectedText: /does not allow this change/i },
      { code: 'OPERATION_FAILED', expectedText: /could not update the Windows startup list/i },
    ];

    for (const tc of errorCases) {
      vi.clearAllMocks();
      backend(
        { startWithWindows: false, startupStatus: { state: 'disabled' } },
        { set_start_with_windows: { code: tc.code, message: String.raw`C:\Users\secret\deskpilot.exe HKCU\Run` } }
      );
      const { unmount } = render(<SettingsView />);
      const sw = await screen.findByRole('switch', { name: 'Start with Windows' });
      await act(async () => {
        fireEvent.click(sw);
      });
      const alertText = screen.getByRole('alert').textContent ?? '';
      expect(alertText).toMatch(tc.expectedText);
      expect(alertText).not.toContain('secret');
      expect(alertText).not.toContain('HKCU');
      expect(sw.getAttribute('aria-checked')).toBe('false');
      unmount();
    }
  });

  it('keeps startup switch disabled while pending even when another settings section runs, and prevents duplicate mutations', async () => {
    let state: SettingsDto = { ...base };
    let resolveStartup!: (val: SettingsDto) => void;
    const pendingStartup = new Promise<SettingsDto>((resolve) => {
      resolveStartup = resolve;
    });

    vi.mocked(tauriCore.invoke).mockImplementation(async (cmd: string, args?: unknown) => {
      const a = (args ?? {}) as Record<string, unknown>;
      switch (cmd) {
        case 'get_settings':
          return state;
        case 'set_theme':
          state = { ...state, theme: a.theme as SettingsDto['theme'] };
          return state;
        case 'set_start_with_windows':
          return pendingStartup;
        default:
          return state;
      }
    });

    render(<SettingsView />);
    const sw = (await screen.findByRole('switch', { name: 'Start with Windows' })) as HTMLButtonElement;
    const dark = screen.getByRole('radio', { name: 'Dark' });

    await act(async () => {
      fireEvent.click(sw);
    });
    expect(sw.disabled).toBe(true);
    expect(calls('set_start_with_windows')).toHaveLength(1);

    // Repeated activation while pending must not issue a second mutation.
    await act(async () => {
      fireEvent.click(sw);
    });
    expect(calls('set_start_with_windows')).toHaveLength(1);

    // Completing an unrelated section operation must not clear startupBusy.
    await act(async () => {
      fireEvent.click(dark);
    });
    expect(calls('set_theme')).toHaveLength(1);
    expect(sw.disabled).toBe(true);

    // Settle the pending startup mutation and verify refresh restores actionable state.
    state = { ...state, startWithWindows: true, startupStatus: { state: 'enabled' } };
    await act(async () => {
      resolveStartup(state);
    });
    expect(sw.disabled).toBe(false);
    expect(sw.getAttribute('aria-checked')).toBe('true');
  });

  it('shows unconfirmed status and disables switch if settings refresh fails after successful startup mutation, then recovers on later refresh', async () => {
    let state: SettingsDto = {
      ...base,
      startWithWindows: true,
      startupStatus: { state: 'enabled' },
    };
    let failNextGetSettings = false;

    vi.mocked(tauriCore.invoke).mockImplementation(async (cmd: string, args?: unknown) => {
      const a = (args ?? {}) as Record<string, unknown>;
      switch (cmd) {
        case 'get_settings':
          if (failNextGetSettings) {
            failNextGetSettings = false;
            throw {
              code: 'OPERATION_FAILED',
              message: String.raw`C:\Users\alice\deskpilot.exe HKCU\Run`,
            };
          }
          return state;
        case 'set_start_with_windows': {
          const enabled = a.enabled as boolean;
          state = {
            ...state,
            startWithWindows: enabled,
            startupStatus: { state: enabled ? 'enabled' : 'disabled' },
          };
          failNextGetSettings = true;
          return state;
        }
        case 'set_theme':
          state = { ...state, theme: a.theme as SettingsDto['theme'] };
          return state;
        default:
          return state;
      }
    });

    render(<SettingsView />);
    const sw = (await screen.findByRole('switch', { name: 'Start with Windows' })) as HTMLButtonElement;
    expect(sw.getAttribute('aria-checked')).toBe('true');
    expect(sw.disabled).toBe(false);

    await act(async () => {
      fireEvent.click(sw);
    });

    expect(calls('set_start_with_windows')).toHaveLength(1);
    expect(calls('get_settings')).toHaveLength(2);
    const alertText = screen.getByRole('alert').textContent ?? '';
    expect(alertText).toMatch(/startup setting was updated, but its current status could not be confirmed/i);
    expect(alertText).not.toMatch(/could not update the Windows startup list/i);
    expect(alertText).not.toContain('alice');
    expect(alertText).not.toContain('HKCU');
    expect(sw.disabled).toBe(true);
    expect(sw.getAttribute('aria-checked')).toBe('false');

    // Subsequent successful backend settings refresh restores confirmed state.
    const dark = screen.getByRole('radio', { name: 'Dark' });
    await act(async () => {
      fireEvent.click(dark);
    });
    expect(sw.disabled).toBe(false);
    expect(sw.getAttribute('aria-checked')).toBe('false');
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('handles unknown, missing, malformed, and non-IpcError startup rejections with sanitized fallback without leaking details', async () => {
    const rejections: unknown[] = [
      {
        code: 'UNKNOWN_FUTURE_CODE',
        message: String.raw`C:\Users\secret\deskpilot.exe HKCU\Software\Microsoft\Windows\CurrentVersion\Run`,
      },
      {
        message: String.raw`Missing code: C:\Windows\System32\cmd.exe (0x80070005)`,
      },
      {
        code: 42,
        message: String.raw`Malformed code at C:\secret\path`,
      },
      new Error(String.raw`Access denied at C:\Program Files\DeskPilot\deskpilot.exe (0x80070005)`),
      String.raw`HKCU\Run failure at C:\secret\deskpilot.exe`,
      null,
      undefined,
    ];

    for (const rejection of rejections) {
      vi.clearAllMocks();
      vi.mocked(tauriCore.invoke).mockImplementation(async (cmd: string) => {
        if (cmd === 'get_settings') return base;
        if (cmd === 'set_start_with_windows') throw rejection;
        return base;
      });

      const { container, unmount } = render(<SettingsView />);
      const sw = (await screen.findByRole('switch', { name: 'Start with Windows' })) as HTMLButtonElement;
      await act(async () => {
        fireEvent.click(sw);
      });

      const alertText = screen.getByRole('alert').textContent ?? '';
      expect(alertText).toBe('DeskPilot could not update the Windows startup list.');
      const domText = container.textContent ?? '';
      for (const sensitive of ['secret', 'HKCU', 'CurrentVersion', 'deskpilot.exe', '0x80070005', 'System32']) {
        expect(domText).not.toContain(sensitive);
      }
      expect(sw.disabled).toBe(false);
      expect(sw.getAttribute('aria-checked')).toBe('false');
      unmount();
    }
  });

  it('glass: disabled without Windows 11, restart note when the choice differs from the session', async () => {
    backend({ glassSupported: false });
    const { unmount } = render(<SettingsView />);
    expect(((await screen.findByRole('switch', { name: 'Glass effect' })) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText(/Needs Windows 11/)).toBeDefined();
    unmount();

    backend();
    render(<SettingsView />);
    const glass = await screen.findByRole('switch', { name: 'Glass effect' });
    await act(async () => {
      fireEvent.click(glass);
    });
    expect(calls('set_glass_effect')[0][1]).toEqual({ enabled: true });
    expect(screen.getByRole('status').textContent).toContain('Restart DeskPilot to turn on the glass effect');
  });

  it('shows a load error instead of an empty page', async () => {
    backend({}, { get_settings: 'Settings lock poisoned' });
    render(<SettingsView />);
    expect((await screen.findByRole('alert')).textContent).toContain('Settings lock poisoned');
  });

  it('shows the shortcut conflict message from the backend', async () => {
    backend({
      shortcutRegistered: false,
      shortcutMessage: 'Ctrl+Alt+Space is already used by another app or reserved by Windows.',
    });
    render(<SettingsView />);
    expect(await screen.findByText(/already used by another app/)).toBeDefined();
  });
});
