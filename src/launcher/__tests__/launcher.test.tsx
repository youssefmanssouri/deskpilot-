// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup, act } from '@testing-library/react';
import * as tauriCore from '@tauri-apps/api/core';
import * as tauriEvent from '@tauri-apps/api/event';
import { LauncherApp, buildLauncherGroups, MAX_APP_RESULTS, LAUNCHER_ACTIONS } from '../LauncherApp';
import { isTauriRuntime, onTauriEvent, NAVIGATE_EVENT } from '../../lib/tauriEnv';
import App from '../../App';
import type { ApplicationDto, LaunchHistoryEntryDto } from '../../lib/ipc';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Captures listeners so tests can fire Tauri events.
const listeners = new Map<string, (e: { payload: unknown }) => void>();
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (event: string, handler: (e: { payload: unknown }) => void) => {
    listeners.set(event, handler);
    return () => listeners.delete(event);
  }),
}));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ onFocusChanged: vi.fn(async () => () => {}) }),
}));

const id = (n: number) => `app_${n.toString(16).padStart(32, '0')}`;
const app = (n: number, name: string, availability: ApplicationDto['availability'] = 'Available'): ApplicationDto => ({
  id: id(n),
  displayName: name,
  publisher: 'Contoso',
  version: '1.0',
  sources: ['Start Menu (User)'],
  availability,
  isInstalled: true,
});

const apps = [
  app(1, 'Windows Terminal'),
  app(2, 'Windows Terminal Preview'),
  app(3, 'Slack', 'Unverified'),
  app(4, 'Legacy CAD', 'MissingTarget'),
  app(5, 'Visual Studio Code'),
];
const history: LaunchHistoryEntryDto[] = [
  { applicationId: id(5), displayName: 'Visual Studio Code', launchedAtMs: 2 },
  { applicationId: id(4), displayName: 'Legacy CAD', launchedAtMs: 1 },
];

const calls = (cmd: string) => vi.mocked(tauriCore.invoke).mock.calls.filter(([c]) => c === cmd);

function backend(overrides: Record<string, (args?: unknown) => unknown> = {}) {
  vi.mocked(tauriCore.invoke).mockImplementation(async (cmd: string, args?: unknown) => {
    if (overrides[cmd]) return overrides[cmd](args);
    switch (cmd) {
      case 'list_applications':
        return apps;
      case 'get_launch_history':
        return history;
      case 'get_launcher_shortcut':
        return { accelerator: 'Ctrl+Alt+Space', registered: true, message: null };
      case 'get_system_metrics':
        return {
          cpu: { brandName: 'Test CPU', logicalCores: 8, architecture: 'x86_64', utilizationPercent: 10 },
          memory: { totalBytes: 16e9, availableBytes: 8e9, usedBytes: 8e9, usedPercent: 50 },
          storage: { drives: [] },
          system: { uptimeSeconds: 60, appUptimeSeconds: 5, osName: 'Windows 11', architecture: 'x86_64' },
          sampledAtMs: 0,
        };
      default:
        return undefined;
    }
  });
}

const input = () => screen.getByRole('combobox', { name: /search apps/i });
const options = () => screen.getAllByRole('option');

describe('buildLauncherGroups', () => {
  it('with no query shows installed recent apps, then actions', () => {
    const groups = buildLauncherGroups('', apps, history);
    expect(groups.map((g) => g.label)).toEqual(['Recent', 'DeskPilot']);
    // Legacy CAD is missing, so it is left out of Recent.
    expect(groups[0].items).toHaveLength(1);
    expect(groups[1].items).toHaveLength(LAUNCHER_ACTIONS.length);
  });

  it('with a query ranks matching apps first and excludes missing ones', () => {
    const groups = buildLauncherGroups('term', apps, history);
    expect(groups[0].label).toBe('Apps');
    const names = groups[0].items.map((i) => (i.kind === 'app' ? i.app.displayName : ''));
    expect(names).toEqual(['Windows Terminal', 'Windows Terminal Preview']);
    expect(buildLauncherGroups('legacy', apps, history)).toEqual([]);
  });

  it('matches actions by their words and caps app results', () => {
    expect(buildLauncherGroups('proc', apps, []).flatMap((g) => g.items)).toEqual([
      { kind: 'action', action: LAUNCHER_ACTIONS.find((a) => a.id === 'processes') },
    ]);
    const many = Array.from({ length: 20 }, (_, i) => app(100 + i, `Tool ${i}`));
    expect(buildLauncherGroups('tool', many, [])[0].items).toHaveLength(MAX_APP_RESULTS);
  });
});

describe('tauriEnv outside Tauri', () => {
  it('is a no-op without the Tauri runtime', () => {
    expect(isTauriRuntime()).toBe(false);
    const off = onTauriEvent('x', () => {});
    expect(tauriEvent.listen).not.toHaveBeenCalled();
    expect(() => off()).not.toThrow();
  });
});

describe('LauncherApp', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listeners.clear();
  });
  afterEach(() => cleanup());

  it('loads the cached catalog, and scans once only when it is empty', async () => {
    backend({
      list_applications: () => [],
      discover_applications: () => ({ applications: apps, scannedSources: [], partialFailures: [], totalDiscovered: apps.length }),
    });
    render(<LauncherApp />);
    await screen.findByText('Visual Studio Code');
    expect(calls('list_applications')).toHaveLength(1);
    expect(calls('discover_applications')).toHaveLength(1);
  });

  it('does not rescan when the catalog is already loaded', async () => {
    backend();
    render(<LauncherApp />);
    await screen.findByText('Visual Studio Code');
    expect(calls('discover_applications')).toHaveLength(0);
    expect(document.activeElement).toBe(input());
  });

  it('Enter opens the selected verified app and hides the launcher', async () => {
    backend({ launch_application: () => ({ applicationId: id(1), processId: 7 }) });
    render(<LauncherApp />);
    await screen.findByText('Visual Studio Code');

    fireEvent.change(input(), { target: { value: 'term' } });
    expect(options()[0].getAttribute('aria-selected')).toBe('true');
    await act(async () => {
      fireEvent.keyDown(input(), { key: 'Enter' });
    });

    expect(calls('launch_application')[0][1]).toEqual({ applicationId: id(1) });
    expect(calls('hide_launcher')).toHaveLength(1);
  });

  it('arrow keys move the selection and update aria-activedescendant', async () => {
    backend();
    render(<LauncherApp />);
    await screen.findByText('Visual Studio Code');
    fireEvent.change(input(), { target: { value: 'term' } });

    fireEvent.keyDown(input(), { key: 'ArrowDown' });
    expect(options()[1].getAttribute('aria-selected')).toBe('true');
    expect(input().getAttribute('aria-activedescendant')).toBe('launcher-item-1');
    fireEvent.keyDown(input(), { key: 'ArrowDown' });
    expect(options()[0].getAttribute('aria-selected')).toBe('true'); // wraps around
  });

  it('Enter does not open an unverified app, but a click does', async () => {
    backend({ launch_application: () => ({ applicationId: id(3), processId: 8 }) });
    render(<LauncherApp />);
    await screen.findByText('Visual Studio Code');
    fireEvent.change(input(), { target: { value: 'slack' } });

    fireEvent.keyDown(input(), { key: 'Enter' });
    expect(calls('launch_application')).toHaveLength(0);
    expect(screen.getByRole('status').textContent).toContain('Enter only opens verified apps');

    await act(async () => {
      fireEvent.click(options()[0]);
    });
    expect(calls('launch_application')[0][1]).toEqual({ applicationId: id(3) });
  });

  it('Ctrl+Enter shows the app in DeskPilot instead of opening it', async () => {
    backend();
    render(<LauncherApp />);
    await screen.findByText('Visual Studio Code');
    fireEvent.change(input(), { target: { value: 'slack' } });

    await act(async () => {
      fireEvent.keyDown(input(), { key: 'Enter', ctrlKey: true });
    });
    expect(calls('show_main_window')[0][1]).toEqual({ view: 'applications', applicationId: id(3) });
    expect(calls('launch_application')).toHaveLength(0);
  });

  it('actions open the matching DeskPilot view', async () => {
    backend();
    render(<LauncherApp />);
    await screen.findByText('Visual Studio Code');
    fireEvent.change(input(), { target: { value: 'system' } });

    await act(async () => {
      fireEvent.keyDown(input(), { key: 'Enter' });
    });
    expect(calls('show_main_window')[0][1]).toEqual({ view: 'system', applicationId: null });
  });

  it('a failed launch keeps the launcher open with an explanation', async () => {
    backend({
      launch_application: () => {
        throw { code: 'PERMISSION_DENIED', message: 'Access to execute target binary was denied by the operating system.' };
      },
    });
    render(<LauncherApp />);
    await screen.findByText('Visual Studio Code');
    fireEvent.change(input(), { target: { value: 'visual' } });

    await act(async () => {
      fireEvent.keyDown(input(), { key: 'Enter' });
    });
    const alert = await screen.findByRole('alert');
    expect(alert.textContent).toContain('Windows or DeskPilot blocked this launch');
    expect(alert.textContent).toContain('Ctrl+Enter');
    expect(calls('hide_launcher')).toHaveLength(0);
  });

  it('Esc clears the query first, then hides the launcher', async () => {
    backend();
    render(<LauncherApp />);
    await screen.findByText('Visual Studio Code');
    fireEvent.change(input(), { target: { value: 'term' } });

    fireEvent.keyDown(input(), { key: 'Escape' });
    expect((input() as HTMLInputElement).value).toBe('');
    expect(calls('hide_launcher')).toHaveLength(0);

    fireEvent.keyDown(input(), { key: 'Escape' });
    expect(calls('hide_launcher')).toHaveLength(1);
  });

  it('explains a shortcut conflict in the footer', async () => {
    backend({
      get_launcher_shortcut: () => ({
        accelerator: 'Ctrl+Alt+Space',
        registered: false,
        message: 'Ctrl+Alt+Space is already used by another app.',
      }),
    });
    render(<LauncherApp />);
    expect(await screen.findByText(/already used by another app/)).toBeDefined();
  });

  it('shows an empty-results message', async () => {
    backend();
    render(<LauncherApp />);
    await screen.findByText('Visual Studio Code');
    fireEvent.change(input(), { target: { value: 'zzzz' } });
    expect(screen.getByText(/No apps match/)).toBeDefined();
  });
});

describe('Main window navigation from the launcher', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listeners.clear();
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  });
  afterEach(() => {
    cleanup();
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it('switches to Apps with the requested app selected, and shows the shortcut', async () => {
    backend({
      get_app_status: () => ({ appName: 'DeskPilot', version: '0.1.1', status: 'ready', uptimeSeconds: 1, environment: 'development' }),
      discover_applications: () => ({ applications: apps, scannedSources: [], partialFailures: [], totalDiscovered: apps.length }),
      get_launch_history: () => [],
    });
    render(<App />);
    expect(await screen.findByText('opens the quick launcher from anywhere')).toBeDefined();
    await waitFor(() => expect(listeners.has(NAVIGATE_EVENT)).toBe(true));

    await act(async () => {
      listeners.get(NAVIGATE_EVENT)!({ payload: { view: 'applications', applicationId: id(3) } });
    });

    await waitFor(() => {
      expect(screen.getByRole('heading', { level: 1, name: /^apps$/i })).toBeDefined();
      expect(screen.getByTestId('application-inspector').textContent).toContain('Slack');
    });
  });

  it('ignores an unknown view', async () => {
    backend({
      get_app_status: () => ({ appName: 'DeskPilot', version: '0.1.1', status: 'ready', uptimeSeconds: 1, environment: 'development' }),
      get_launch_history: () => [],
    });
    render(<App />);
    await waitFor(() => expect(listeners.has(NAVIGATE_EVENT)).toBe(true));
    await act(async () => {
      listeners.get(NAVIGATE_EVENT)!({ payload: { view: 'settings', applicationId: null } });
    });
    expect(screen.getByRole('button', { name: 'Home' }).getAttribute('aria-current')).toBe('page');
  });
});
