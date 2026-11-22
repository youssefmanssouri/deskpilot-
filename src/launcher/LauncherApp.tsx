import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  deskpilotApi,
  IpcError,
  type ApplicationDto,
  type LaunchHistoryEntryDto,
  type NavigableView,
  type ShortcutStatusDto,
} from '../lib/ipc';
import {
  LAUNCHER_SHOWN_EVENT,
  SETTINGS_CHANGED_EVENT,
  onTauriEvent,
  onWindowFocusChange,
} from '../lib/tauriEnv';
import { applyAppearance } from '../lib/appearance';
import type { SettingsDto } from '../lib/ipc';
import { filterApplications } from '../components/ApplicationCatalog';
import { getAvailabilityLabel } from '../components/ApplicationCard';
import { AppIcon } from '../components/AppIcon';
import { canLaunchWithClick, canLaunchWithKeyboard } from '../components/launchPolicy';
import { describeLaunchError } from '../components/launchErrorDescription';
import './Launcher.css';

/** Maximum rows per group, so the list never needs much scrolling. */
export const MAX_APP_RESULTS = 7;
export const MAX_RECENT_RESULTS = 5;

export interface LauncherAction {
  id: string;
  label: string;
  /** Extra words that also match this action. */
  keywords: string;
  view: NavigableView;
}

export const LAUNCHER_ACTIONS: LauncherAction[] = [
  { id: 'home', label: 'Open DeskPilot', keywords: 'home deskpilot main window', view: 'home' },
  { id: 'apps', label: 'Browse all apps', keywords: 'apps applications catalog list', view: 'applications' },
  { id: 'processes', label: 'Show running processes', keywords: 'processes tasks running memory', view: 'processes' },
  { id: 'system', label: 'Show system resources', keywords: 'system cpu memory disk drives uptime', view: 'system' },
];

export type LauncherItem =
  | { kind: 'app'; app: ApplicationDto }
  | { kind: 'action'; action: LauncherAction };

export interface LauncherGroup {
  label: string;
  items: LauncherItem[];
}

/**
 * Builds the result groups shown in the launcher.
 * - Empty query: recent apps that are still installed, then the DeskPilot actions.
 * - With a query: matching apps (same ranking as the Apps view, missing apps excluded),
 *   then matching actions.
 */
export function buildLauncherGroups(
  query: string,
  apps: ApplicationDto[],
  history: LaunchHistoryEntryDto[]
): LauncherGroup[] {
  const q = query.trim().toLowerCase();
  const groups: LauncherGroup[] = [];

  if (!q) {
    const byId = new Map(apps.map((a) => [a.id, a]));
    const recent = history
      .map((h) => byId.get(h.applicationId))
      .filter((a): a is ApplicationDto => !!a && a.availability !== 'MissingTarget')
      .slice(0, MAX_RECENT_RESULTS);
    if (recent.length) groups.push({ label: 'Recent', items: recent.map((app) => ({ kind: 'app', app })) });
    groups.push({ label: 'DeskPilot', items: LAUNCHER_ACTIONS.map((action) => ({ kind: 'action', action })) });
    return groups;
  }

  const launchable = apps.filter((a) => a.availability !== 'MissingTarget');
  const matches = filterApplications(launchable, q).slice(0, MAX_APP_RESULTS);
  if (matches.length) groups.push({ label: 'Apps', items: matches.map((app) => ({ kind: 'app', app })) });

  const actions = LAUNCHER_ACTIONS.filter((a) =>
    `${a.label} ${a.keywords}`.toLowerCase().split(/\s+/).some((w) => w.startsWith(q))
  );
  if (actions.length) groups.push({ label: 'DeskPilot', items: actions.map((action) => ({ kind: 'action', action })) });
  return groups;
}

interface Notice {
  kind: 'error' | 'info';
  title: string;
  detail?: string;
}

function hideLauncher() {
  void deskpilotApi.hideLauncher().catch(() => {});
}

/**
 * The quick launcher: a small borderless window opened with the global shortcut.
 * Type to search, arrows to move, Enter to open, Ctrl+Enter to show in DeskPilot, Esc to close.
 */
export function LauncherApp() {
  const [query, setQuery] = useState('');
  const [apps, setApps] = useState<ApplicationDto[]>([]);
  const [history, setHistory] = useState<LaunchHistoryEntryDto[]>([]);
  const [activeIndex, setActiveIndex] = useState(0);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [shortcut, setShortcut] = useState<ShortcutStatusDto | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const shownAtRef = useRef<number>(0);
  const hasGainedFocusRef = useRef<boolean>(false);

  const refresh = useCallback(async () => {
    try {
      let list = await deskpilotApi.listApplications();
      // The main window normally discovers apps at startup; scan once if it has not yet.
      if (!Array.isArray(list) || list.length === 0) {
        list = (await deskpilotApi.discoverApplications()).applications;
      }
      setApps(list);
    } catch (err) {
      setNotice({
        kind: 'error',
        title: "Apps couldn't be loaded",
        detail: IpcError.fromResponse(err).message,
      });
    }
    deskpilotApi
      .getLaunchHistory()
      .then((h) => setHistory(Array.isArray(h) ? h : []))
      .catch(() => setHistory([]));
    deskpilotApi
      .getLauncherShortcut()
      .then(setShortcut)
      .catch(() => {});
    deskpilotApi
      .getSettings()
      .then((s) => s && applyAppearance({ theme: s.theme, glassActive: false }))
      .catch(() => {});
  }, []);

  const reset = useCallback(() => {
    setQuery('');
    setActiveIndex(0);
    setNotice(null);
    setBusyId(null);
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    void refresh();
    reset();
    const offShown = onTauriEvent(LAUNCHER_SHOWN_EVENT, () => {
      shownAtRef.current = Date.now();
      hasGainedFocusRef.current = false;
      reset();
      void refresh();
    });
    // Theme changes made in Settings apply to the launcher right away. (The launcher is
    // never transparent, so glass is always off here.)
    const offSettings = onTauriEvent<SettingsDto>(SETTINGS_CHANGED_EVENT, (s) =>
      applyAppearance({ theme: s.theme, glassActive: false })
    );
    // Like other launchers, clicking anywhere else closes it.
    // Ignores transient blur events that occur during initial window creation or within
    // 400ms of being shown before the window has acquired active focus.
    const offFocus = onWindowFocusChange((focused) => {
      if (focused) {
        hasGainedFocusRef.current = true;
        inputRef.current?.focus();
      } else {
        const elapsed = Date.now() - shownAtRef.current;
        if (elapsed < 400 && !hasGainedFocusRef.current) {
          return;
        }
        hideLauncher();
      }
    });
    return () => {
      offShown();
      offSettings();
      offFocus();
    };
  }, [refresh, reset]);

  const groups = useMemo(() => buildLauncherGroups(query, apps, history), [query, apps, history]);
  const flat = useMemo(() => groups.flatMap((g) => g.items), [groups]);
  const safeIndex = flat.length ? Math.min(activeIndex, flat.length - 1) : 0;

  const launch = async (app: ApplicationDto) => {
    setBusyId(app.id);
    setNotice(null);
    try {
      await deskpilotApi.launchApplication(app.id);
      hideLauncher();
      reset();
    } catch (err) {
      const e = IpcError.fromResponse(err);
      setNotice({
        kind: 'error',
        title: describeLaunchError(e.code).title,
        detail: `${e.message} Press Ctrl+Enter to see what to do in DeskPilot.`,
      });
    } finally {
      setBusyId(null);
    }
  };

  const showInDeskPilot = (view: NavigableView, applicationId?: string) => {
    void deskpilotApi.showMainWindow(view, applicationId).catch((err) =>
      setNotice({ kind: 'error', title: "DeskPilot couldn't be opened", detail: IpcError.fromResponse(err).message })
    );
  };

  const activate = (item: LauncherItem | undefined, how: 'keyboard' | 'click' | 'details') => {
    if (!item) return;
    if (item.kind === 'action') {
      showInDeskPilot(item.action.view);
      return;
    }
    const { app } = item;
    if (how === 'details') {
      showInDeskPilot('applications', app.id);
      return;
    }
    const allowed = how === 'keyboard' ? canLaunchWithKeyboard(app) : canLaunchWithClick(app);
    if (allowed) {
      void launch(app);
    } else {
      setNotice({
        kind: 'info',
        title: `${app.displayName} is ${getAvailabilityLabel(app.availability).toLowerCase()}`,
        detail: 'Enter only opens verified apps. Press Ctrl+Enter to check it in DeskPilot, or click it to open it anyway.',
      });
    }
  };

  const onKeyDown: React.KeyboardEventHandler<HTMLInputElement> = (e) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (flat.length) setActiveIndex((safeIndex + 1) % flat.length);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (flat.length) setActiveIndex((safeIndex - 1 + flat.length) % flat.length);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      activate(flat[safeIndex], e.ctrlKey ? 'details' : 'keyboard');
    } else if (e.key === 'Escape') {
      e.preventDefault();
      if (query) {
        setQuery('');
        setActiveIndex(0);
        setNotice(null);
      } else {
        hideLauncher();
      }
    }
  };

  // Keep the selected row visible when moving with the arrow keys.
  useEffect(() => {
    const el = document.getElementById(`launcher-item-${safeIndex}`);
    el?.scrollIntoView?.({ block: 'nearest' });
  }, [safeIndex]);

  let rowIndex = -1;
  const activeId = flat.length ? `launcher-item-${safeIndex}` : undefined;

  return (
    <div className="launcher" role="dialog" aria-label="DeskPilot quick launcher">
      <div className="launcher-search">
        <svg className="launcher-search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" aria-hidden="true">
          <circle cx="11" cy="11" r="7" />
          <line x1="20" y1="20" x2="16.5" y2="16.5" />
        </svg>
        <input
          ref={inputRef}
          className="launcher-input"
          type="text"
          role="combobox"
          aria-label="Search apps and DeskPilot actions"
          aria-expanded={flat.length > 0}
          aria-controls="launcher-results"
          aria-activedescendant={activeId}
          aria-autocomplete="list"
          placeholder="Open an app or a DeskPilot page"
          value={query}
          autoComplete="off"
          spellCheck={false}
          onChange={(e) => {
            setQuery(e.target.value);
            setActiveIndex(0);
            setNotice(null);
          }}
          onKeyDown={onKeyDown}
        />
      </div>

      {notice && (
        <div className={`launcher-notice launcher-notice--${notice.kind}`} role={notice.kind === 'error' ? 'alert' : 'status'}>
          <strong>{notice.title}</strong>
          {notice.detail && <span>{notice.detail}</span>}
        </div>
      )}

      <ul id="launcher-results" className="launcher-results" role="listbox" aria-label="Results">
        {groups.length === 0 && (
          <li className="launcher-empty" role="presentation">
            No apps match “{query}”.
          </li>
        )}
        {groups.map((group) => (
          <li key={group.label} role="presentation">
            <div className="launcher-group" role="presentation">
              {group.label}
            </div>
            <ul role="presentation" className="launcher-group-items">
              {group.items.map((item) => {
                rowIndex += 1;
                const index = rowIndex;
                const selected = index === safeIndex;
                const key = item.kind === 'app' ? item.app.id : item.action.id;
                return (
                  <li
                    key={key}
                    id={`launcher-item-${index}`}
                    role="option"
                    aria-selected={selected}
                    className={`launcher-item ${selected ? 'selected' : ''}`}
                    onMouseMove={() => index !== safeIndex && setActiveIndex(index)}
                    onClick={() => activate(item, 'click')}
                  >
                    {item.kind === 'app' ? (
                      <>
                        <AppIcon
                          applicationId={item.app.id}
                          displayName={item.app.displayName}
                          className="launcher-tile"
                        />
                        <span className="launcher-item-text">
                          <span className="launcher-item-name">{item.app.displayName}</span>
                          <span className="launcher-item-meta">
                            {item.app.publisher ?? 'Unknown publisher'}
                            {item.app.availability !== 'Available' && ` · ${getAvailabilityLabel(item.app.availability)}`}
                          </span>
                        </span>
                        <span className="launcher-item-hint">{busyId === item.app.id ? 'Opening…' : 'Open ⏎'}</span>
                      </>
                    ) : (
                      <>
                        <span className="launcher-tile launcher-tile--action" aria-hidden="true">
                          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                            <path d="m9 18 6-6-6-6" />
                          </svg>
                        </span>
                        <span className="launcher-item-text">
                          <span className="launcher-item-name">{item.action.label}</span>
                        </span>
                        <span className="launcher-item-hint">Go ⏎</span>
                      </>
                    )}
                  </li>
                );
              })}
            </ul>
          </li>
        ))}
      </ul>

      <div className="launcher-footer">
        {shortcut && !shortcut.registered ? (
          <span className="launcher-shortcut-warning" role="status">
            {shortcut.message}
          </span>
        ) : (
          <span className="launcher-footer-keys">
            <kbd>↑</kbd> <kbd>↓</kbd> Move
          </span>
        )}
        <span className="launcher-footer-keys">
          <kbd>Enter</kbd> Open
        </span>
        <span className="launcher-footer-keys">
          <kbd>Ctrl</kbd> <kbd>Enter</kbd> Show in DeskPilot
        </span>
        <span className="launcher-footer-keys">
          <kbd>Esc</kbd> Close
        </span>
      </div>
    </div>
  );
}
