import { useEffect, useRef, useState } from 'react';
import {
  deskpilotApi,
  IpcError,
  type SettingsDto,
  type StartupStatusDto,
  type ThemePreference,
} from '../lib/ipc';
import { applyAppearance } from '../lib/appearance';
import './SettingsView.css';

/**
 * Builds a shortcut string ("Ctrl+Alt+K") from a key press, or returns null while only
 * modifiers are held or for keys DeskPilot does not accept. The backend validates again.
 */
export function shortcutFromKeyEvent(e: {
  key: string;
  code?: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}): string | null {
  let key: string | null = null;
  // Prefer the physical key code so AZERTY/QWERTY and Shift do not change the letter.
  const code = e.code ?? '';
  if (/^Key[A-Z]$/.test(code)) key = code.slice(3);
  else if (/^Digit[0-9]$/.test(code)) key = code.slice(5);
  else if (code === 'Space' || e.key === ' ') key = 'Space';
  else if (/^F([1-9]|1[0-9]|2[0-4])$/.test(e.key)) key = e.key;
  else if (/^[a-z0-9]$/i.test(e.key)) key = e.key.toUpperCase();
  if (!key) return null;

  const parts: string[] = [];
  if (e.ctrlKey) parts.push('Ctrl');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');
  if (e.metaKey) parts.push('Win');
  parts.push(key);
  return parts.join('+');
}

const THEMES: { id: ThemePreference; label: string }[] = [
  { id: 'system', label: 'Use Windows setting' },
  { id: 'light', label: 'Light' },
  { id: 'dark', label: 'Dark' },
];

type Section = 'appearance' | 'launcher' | 'startup' | 'privacy';

interface StartupNotice {
  kind: 'warning' | 'error';
  message: string;
}

function getStartupNotice(status: StartupStatusDto): StartupNotice | null {
  switch (status.state) {
    case 'disabled':
    case 'enabled':
      return null;
    case 'stale':
      return {
        kind: 'warning',
        message:
          'DeskPilot found a stale startup entry from a previous installation. Automatic changes are disabled to protect your startup configuration.',
      };
    case 'otherInstall':
      return {
        kind: 'warning',
        message:
          'Another DeskPilot installation is registered to start with Windows. Automatic changes are disabled here.',
      };
    case 'unrecognized':
      return {
        kind: 'warning',
        message:
          'The existing Windows startup entry is unrecognized. Automatic changes are disabled to avoid modifying it.',
      };
    case 'unavailable':
      switch (status.errorCode) {
        case 'PERMISSION_DENIED':
          return {
            kind: 'error',
            message: 'Permission denied while checking the Windows startup list.',
          };
        case 'INVALID_STARTUP_STATE':
          return {
            kind: 'error',
            message: "DeskPilot couldn't verify the Windows startup state.",
          };
        case 'OPERATION_FAILED':
        default:
          return {
            kind: 'error',
            message: "DeskPilot couldn't check the Windows startup list.",
          };
      }
  }
}

function startupMutationErrorMessage(err: unknown): string {
  const { code } = IpcError.fromResponse(err);
  switch (code) {
    case 'PERMISSION_DENIED':
      return 'Permission denied while updating the Windows startup list.';
    case 'STARTUP_CONFLICT':
      return 'Another or stale DeskPilot startup entry already exists and was not modified.';
    case 'INVALID_STARTUP_STATE':
      return 'The current Windows startup state does not allow this change.';
    case 'OPERATION_FAILED':
    default:
      return 'DeskPilot could not update the Windows startup list.';
  }
}

export interface SettingsViewProps {
  /** Lets the rest of the app drop its copy of the history after it is cleared. */
  onHistoryCleared?: () => void;
}

/**
 * Settings: theme, glass effect, launcher shortcut, start with Windows, launch history.
 * Every change is saved by the backend immediately; errors are shown next to the setting.
 */
export function SettingsView({ onHistoryCleared }: SettingsViewProps) {
  const [settings, setSettings] = useState<SettingsDto | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [errors, setErrors] = useState<Partial<Record<Section, string>>>({});
  const [notice, setNotice] = useState<Partial<Record<Section, string>>>({});
  const [busy, setBusy] = useState<Section | null>(null);
  const [startupBusy, setStartupBusy] = useState(false);
  const [startupUnconfirmed, setStartupUnconfirmed] = useState(false);
  const startupPendingRef = useRef(false);
  const [recording, setRecording] = useState(false);
  const [captured, setCaptured] = useState<string | null>(null);
  const captureRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    deskpilotApi
      .getSettings()
      .then((s) => {
        setSettings(s);
        setStartupUnconfirmed(false);
        applyAppearance(s);
      })
      .catch((err) => setLoadError(IpcError.fromResponse(err).message || 'Settings could not be loaded.'));
  }, []);

  useEffect(() => {
    if (recording) captureRef.current?.focus();
  }, [recording]);

  const run = async (
    section: Section,
    action: () => Promise<SettingsDto>,
    done?: string,
    formatError?: (err: unknown) => string
  ) => {
    setBusy(section);
    setErrors((e) => ({ ...e, [section]: undefined }));
    setNotice((n) => ({ ...n, [section]: undefined }));
    try {
      const next = await action();
      setSettings(next);
      setStartupUnconfirmed(false);
      applyAppearance(next);
      if (done) setNotice((n) => ({ ...n, [section]: done }));
      return true;
    } catch (err) {
      const message =
        formatError?.(err) ?? (IpcError.fromResponse(err).message || 'That change could not be saved.');
      setErrors((e) => ({ ...e, [section]: message }));
      return false;
    } finally {
      setBusy(null);
    }
  };

  const toggleStartup = async (nextEnabled: boolean) => {
    if (startupPendingRef.current) return;
    startupPendingRef.current = true;
    setStartupBusy(true);
    setErrors((e) => ({ ...e, startup: undefined }));
    setNotice((n) => ({ ...n, startup: undefined }));
    try {
      await deskpilotApi.setStartWithWindows(nextEnabled);
    } catch (err) {
      setErrors((e) => ({ ...e, startup: startupMutationErrorMessage(err) }));
      startupPendingRef.current = false;
      setStartupBusy(false);
      return;
    }

    try {
      const refreshed = await deskpilotApi.getSettings();
      setSettings(refreshed);
      setStartupUnconfirmed(false);
      applyAppearance(refreshed);
    } catch {
      setStartupUnconfirmed(true);
    } finally {
      startupPendingRef.current = false;
      setStartupBusy(false);
    }
  };

  if (loadError) {
    return (
      <div className="settings-view" role="region" aria-label="Settings">
        <p className="settings-error" role="alert">
          {loadError}
        </p>
      </div>
    );
  }
  if (!settings) {
    return (
      <div className="settings-view" role="region" aria-label="Settings">
        <p className="settings-muted">Loading settings…</p>
      </div>
    );
  }

  const glassPending = settings.glassSupported && settings.glassEffect !== settings.glassActive;
  const startupState = settings.startupStatus.state;
  const canToggleStartup =
    !startupUnconfirmed && (startupState === 'disabled' || startupState === 'enabled');
  const startupNotice: StartupNotice | null = startupUnconfirmed
    ? {
        kind: 'error',
        message:
          'The startup setting was updated, but its current status could not be confirmed. Automatic changes are paused until settings refresh.',
      }
    : getStartupNotice(settings.startupStatus);

  return (
    <div className="settings-view" role="region" aria-label="Settings">
      {/* Appearance */}
      <section className="settings-card" aria-labelledby="settings-appearance">
        <h2 id="settings-appearance" className="settings-card-title">
          Appearance
        </h2>

        <div className="settings-row">
          <div className="settings-row-text">
            <span className="settings-row-label" id="settings-theme-label">
              Theme
            </span>
            <span className="settings-row-help">Applies to DeskPilot and the quick launcher.</span>
          </div>
          <div className="settings-segmented" role="radiogroup" aria-labelledby="settings-theme-label">
            {THEMES.map((t) => (
              <button
                key={t.id}
                type="button"
                role="radio"
                aria-checked={settings.theme === t.id}
                className={settings.theme === t.id ? 'active' : ''}
                disabled={busy === 'appearance'}
                onClick={() => settings.theme !== t.id && run('appearance', () => deskpilotApi.setTheme(t.id))}
              >
                {t.label}
              </button>
            ))}
          </div>
        </div>

        <div className="settings-row">
          <div className="settings-row-text">
            <span className="settings-row-label" id="settings-glass-label">
              Glass effect
            </span>
            <span className="settings-row-help">
              {settings.glassSupported
                ? 'Frosted window background (Windows 11 Mica). Applies the next time DeskPilot starts.'
                : 'Needs Windows 11. DeskPilot uses a solid background on this PC.'}
            </span>
          </div>
          <button
            type="button"
            role="switch"
            className="settings-switch"
            aria-checked={settings.glassEffect}
            aria-labelledby="settings-glass-label"
            disabled={!settings.glassSupported || busy === 'appearance'}
            onClick={() => run('appearance', () => deskpilotApi.setGlassEffect(!settings.glassEffect))}
          />
        </div>
        {glassPending && (
          <p className="settings-note" role="status">
            Restart DeskPilot to {settings.glassEffect ? 'turn on' : 'turn off'} the glass effect.
          </p>
        )}
        {errors.appearance && (
          <p className="settings-error" role="alert">
            {errors.appearance}
          </p>
        )}
      </section>

      {/* Quick launcher */}
      <section className="settings-card" aria-labelledby="settings-launcher">
        <h2 id="settings-launcher" className="settings-card-title">
          Quick launcher
        </h2>
        <div className="settings-row">
          <div className="settings-row-text">
            <span className="settings-row-label">Shortcut</span>
            <span className="settings-row-help">
              {settings.shortcutRegistered
                ? 'Opens the quick launcher from any app.'
                : settings.shortcutMessage ?? 'The shortcut is not active.'}
            </span>
          </div>
          {!recording ? (
            <div className="settings-inline">
              <kbd className={`settings-kbd ${settings.shortcutRegistered ? '' : 'warning'}`.trim()}>
                {settings.launcherShortcutLabel}
              </kbd>
              <button
                type="button"
                className="settings-button"
                onClick={() => {
                  setCaptured(null);
                  setErrors((e) => ({ ...e, launcher: undefined }));
                  setNotice((n) => ({ ...n, launcher: undefined }));
                  setRecording(true);
                }}
              >
                Change
              </button>
            </div>
          ) : (
            <div className="settings-inline">
              <div
                ref={captureRef}
                className="settings-capture"
                tabIndex={0}
                role="textbox"
                aria-label="Press the new shortcut"
                aria-live="polite"
                onKeyDown={(e) => {
                  if (e.key === 'Escape' && !e.ctrlKey && !e.altKey && !e.metaKey) {
                    e.preventDefault();
                    setRecording(false);
                    return;
                  }
                  if (e.key === 'Tab') return;
                  e.preventDefault();
                  const combo = shortcutFromKeyEvent(e);
                  if (combo) setCaptured(combo);
                }}
              >
                {captured ?? 'Press the new shortcut…'}
              </div>
              <button
                type="button"
                className="settings-button primary"
                disabled={!captured || busy === 'launcher'}
                onClick={async () => {
                  if (!captured) return;
                  const ok = await run(
                    'launcher',
                    () => deskpilotApi.setLauncherShortcut(captured),
                    `The quick launcher now opens with ${captured}.`
                  );
                  if (ok) setRecording(false);
                }}
              >
                Save
              </button>
              <button type="button" className="settings-button" onClick={() => setRecording(false)}>
                Cancel
              </button>
            </div>
          )}
        </div>
        {recording && (
          <p className="settings-muted">
            Use Ctrl, Alt or Win together with a letter, a digit, Space or F1–F24. Esc cancels.
          </p>
        )}
        {notice.launcher && (
          <p className="settings-note" role="status">
            {notice.launcher}
          </p>
        )}
        {errors.launcher && (
          <p className="settings-error" role="alert">
            {errors.launcher}
          </p>
        )}
      </section>

      {/* Startup */}
      <section className="settings-card" aria-labelledby="settings-startup">
        <h2 id="settings-startup" className="settings-card-title">
          Startup
        </h2>
        <div className="settings-row">
          <div className="settings-row-text">
            <span className="settings-row-label" id="settings-autostart-label">
              Start with Windows
            </span>
            <span className="settings-row-help" id="settings-autostart-help">
              Starts in the tray when you sign in, so the shortcut always works.
            </span>
          </div>
          <button
            type="button"
            role="switch"
            className="settings-switch"
            aria-checked={!startupUnconfirmed && startupState === 'enabled'}
            aria-labelledby="settings-autostart-label"
            aria-describedby={startupNotice ? 'settings-autostart-status' : 'settings-autostart-help'}
            disabled={!canToggleStartup || startupBusy}
            onClick={() => {
              if (!canToggleStartup) return;
              void toggleStartup(startupState === 'disabled');
            }}
          />
        </div>
        {startupNotice && (
          <p
            id="settings-autostart-status"
            className={startupNotice.kind === 'error' ? 'settings-error' : 'settings-note'}
            role={startupNotice.kind === 'error' ? 'alert' : 'status'}
          >
            {startupNotice.message}
          </p>
        )}
        {errors.startup && (
          <p className="settings-error" role="alert">
            {errors.startup}
          </p>
        )}
      </section>

      {/* Privacy */}
      <section className="settings-card" aria-labelledby="settings-privacy">
        <h2 id="settings-privacy" className="settings-card-title">
          Privacy
        </h2>
        <div className="settings-row">
          <div className="settings-row-text">
            <span className="settings-row-label" id="settings-history-label">
              Remember recent apps
            </span>
            <span className="settings-row-help">
              Keeps the last 10 apps you opened, on this PC only. Turning this off also deletes the list.
            </span>
          </div>
          <button
            type="button"
            role="switch"
            className="settings-switch"
            aria-checked={settings.historyEnabled}
            aria-labelledby="settings-history-label"
            disabled={busy === 'privacy'}
            onClick={async () => {
              const turningOff = settings.historyEnabled;
              const ok = await run(
                'privacy',
                () => deskpilotApi.setHistoryEnabled(!settings.historyEnabled),
                turningOff ? 'Recent apps are off and the list was deleted.' : undefined
              );
              if (ok && turningOff) onHistoryCleared?.();
            }}
          />
        </div>
        {notice.privacy && (
          <p className="settings-note" role="status">
            {notice.privacy}
          </p>
        )}
        {errors.privacy && (
          <p className="settings-error" role="alert">
            {errors.privacy}
          </p>
        )}
      </section>
    </div>
  );
}
