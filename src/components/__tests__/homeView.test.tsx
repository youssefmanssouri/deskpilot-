// @vitest-environment jsdom
import { useState } from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup, act, within } from '@testing-library/react';
import * as tauriCore from '@tauri-apps/api/core';
import App from '../../App';
import { ApplicationCatalog, type CatalogView } from '../ApplicationCatalog';
import { formatRelativeTime, getGreeting, pushRecentEntry, MAX_RECENT_ENTRIES } from '../HomeView';
import type { ApplicationDto, LaunchHistoryEntryDto } from '../../lib/ipc';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const id = (n: number) => `app_${n.toString(16).padStart(32, '0')}`;

const apps: ApplicationDto[] = [
  { id: id(1), displayName: 'Visual Studio Code', publisher: 'Microsoft', version: '1.95.0', sources: ['Start Menu (User)'], availability: 'Available', isInstalled: true },
  { id: id(2), displayName: 'Slack', publisher: 'Slack Technologies', sources: ['Registry (HKCU User)'], availability: 'Unverified', isInstalled: true },
  { id: id(3), displayName: 'Legacy CAD', publisher: 'Contoso', sources: ['Registry (HKLM 32-bit)'], availability: 'MissingTarget', isInstalled: false },
];

const NOW = Date.UTC(2026, 10, 20, 12, 0, 0);
const history: LaunchHistoryEntryDto[] = [
  { applicationId: id(1), displayName: 'Visual Studio Code', launchedAtMs: NOW - 5 * 60_000 },
  { applicationId: id(9), displayName: 'Removed App', launchedAtMs: NOW - 2 * 86_400_000 },
];

type Handler = (cmd: string, args?: unknown) => unknown;
function mockBackend(handler: Handler) {
  vi.mocked(tauriCore.invoke).mockImplementation(async (cmd: string, args?: unknown) => handler(cmd, args));
}
const calls = (cmd: string) => vi.mocked(tauriCore.invoke).mock.calls.filter(([c]) => c === cmd);

/** Catalog with a real view state, like App provides. */
function Harness({ start = 'home' as CatalogView }) {
  const [view, setView] = useState<CatalogView>(start);
  return (
    <>
      <span data-testid="view">{view}</span>
      <ApplicationCatalog
        initialApplications={apps}
        autoDiscover={false}
        activeView={view}
        onViewChange={setView}
      />
    </>
  );
}

describe('Home helpers', () => {
  it('getGreeting follows the local hour', () => {
    expect(getGreeting(new Date(2026, 0, 1, 4, 59))).toBe('Good evening');
    expect(getGreeting(new Date(2026, 0, 1, 5, 0))).toBe('Good morning');
    expect(getGreeting(new Date(2026, 0, 1, 11, 59))).toBe('Good morning');
    expect(getGreeting(new Date(2026, 0, 1, 12, 0))).toBe('Good afternoon');
    expect(getGreeting(new Date(2026, 0, 1, 18, 0))).toBe('Good evening');
  });

  it('formatRelativeTime gives short relative labels', () => {
    expect(formatRelativeTime(NOW - 10_000, NOW)).toBe('Just now');
    expect(formatRelativeTime(NOW - 5 * 60_000, NOW)).toBe('5 min ago');
    expect(formatRelativeTime(NOW - 3 * 3_600_000, NOW)).toBe('3 h ago');
    expect(formatRelativeTime(NOW - 30 * 3_600_000, NOW)).toBe('Yesterday');
    expect(formatRelativeTime(NOW - 4 * 86_400_000, NOW)).toBe('4 days ago');
    expect(formatRelativeTime(NOW + 60_000, NOW)).toBe('Just now');
  });

  it('pushRecentEntry keeps newest first, one entry per app, capped', () => {
    const many = Array.from({ length: MAX_RECENT_ENTRIES }, (_, i) => ({
      applicationId: id(i + 1),
      displayName: `App ${i + 1}`,
      launchedAtMs: i,
    }));
    const out = pushRecentEntry(many, { applicationId: id(3), displayName: 'App 3', launchedAtMs: 99 });
    expect(out).toHaveLength(MAX_RECENT_ENTRIES);
    expect(out[0]).toMatchObject({ applicationId: id(3), launchedAtMs: 99 });
    expect(out.filter((e) => e.applicationId === id(3))).toHaveLength(1);

    const capped = pushRecentEntry(many, { applicationId: id(50), displayName: 'New', launchedAtMs: 1 });
    expect(capped).toHaveLength(MAX_RECENT_ENTRIES);
    expect(capped.some((e) => e.applicationId === id(MAX_RECENT_ENTRIES))).toBe(false);
  });
});

describe('Home view', () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it('does not load history while another view is shown', () => {
    mockBackend(() => []);
    render(<Harness start="applications" />);
    expect(calls('get_launch_history')).toHaveLength(0);
  });

  it('loads history once and shows recent apps with their state', async () => {
    mockBackend((cmd) => (cmd === 'get_launch_history' ? history : undefined));
    render(<Harness />);

    await screen.findByText('Removed App');
    expect(calls('get_launch_history')).toHaveLength(1);

    // An app that is no longer in the catalog cannot be opened.
    expect(screen.getByText('No longer found')).toBeDefined();
    expect((screen.getByRole('button', { name: 'Open Removed App' }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole('button', { name: 'Open Visual Studio Code' }) as HTMLButtonElement).disabled).toBe(false);

    // Switching away and back does not reload.
    fireEvent.click(screen.getByRole('button', { name: 'View Slack' }));
    expect(screen.getByTestId('view').textContent).toBe('applications');
    expect(calls('get_launch_history')).toHaveLength(1);
  });

  it('shows an empty state when there is no history', async () => {
    mockBackend((cmd) => (cmd === 'get_launch_history' ? [] : undefined));
    render(<Harness />);
    expect(await screen.findByText(/Apps you open from DeskPilot appear here/)).toBeDefined();
    expect(screen.queryByRole('button', { name: 'Clear recent apps' })).toBeNull();
  });

  it('opening from a Recent card records it locally and shows progress', async () => {
    mockBackend((cmd) => {
      if (cmd === 'get_launch_history') return history;
      if (cmd === 'launch_application') return { applicationId: id(1), processId: 42 };
      return undefined;
    });
    render(<Harness />);
    await screen.findByText('Removed App');

    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Open Visual Studio Code' }));
    });

    expect(calls('launch_application')[0][1]).toEqual({ applicationId: id(1) });
    expect(await screen.findByText('Opened just now')).toBeDefined();
    expect(calls('get_launch_history')).toHaveLength(1);
    const names = within(screen.getByRole('region', { name: 'Recent apps' }))
      .getAllByText(/Visual Studio Code|Removed App/)
      .map((n) => n.textContent);
    expect(names[0]).toBe('Visual Studio Code');
  });

  it('a failed launch links to the explanation in the Apps view', async () => {
    mockBackend((cmd) => {
      if (cmd === 'get_launch_history') return history;
      if (cmd === 'launch_application')
        throw { code: 'PERMISSION_DENIED', message: 'Access to execute target binary was denied by the operating system.' };
      return undefined;
    });
    render(<Harness />);
    await screen.findByText('Removed App');

    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Open Visual Studio Code' }));
    });
    fireEvent.click(await screen.findByRole('button', { name: /Couldn't open · Details/ }));

    expect(screen.getByTestId('view').textContent).toBe('applications');
    const panel = screen.getByTestId('launch-error-panel');
    expect(panel.textContent).toContain('Windows or DeskPilot blocked this launch');
  });

  it('clears history, and reports a failure to clear', async () => {
    let failClear = true;
    mockBackend((cmd) => {
      if (cmd === 'get_launch_history') return history;
      if (cmd === 'clear_launch_history') {
        if (failClear) throw { code: 'OPERATION_FAILED', message: 'Could not save launch history.' };
        return undefined;
      }
      return undefined;
    });
    render(<Harness />);
    await screen.findByText('Removed App');

    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Clear recent apps' }));
    });
    expect(screen.getByRole('alert').textContent).toContain('Could not save launch history.');
    expect(screen.getByText('Removed App')).toBeDefined();

    failClear = false;
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Clear recent apps' }));
    });
    expect(screen.queryByText('Removed App')).toBeNull();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(calls('clear_launch_history')).toHaveLength(2);
  });

  it('lists apps that need attention and opens them selected in Apps', async () => {
    mockBackend((cmd) => (cmd === 'get_launch_history' ? [] : undefined));
    render(<Harness />);

    const region = screen.getByRole('region', { name: 'Needs attention' });
    expect(within(region).getByText('Slack')).toBeDefined();
    expect(within(region).getByText('Legacy CAD')).toBeDefined();
    expect(within(region).queryByText('Visual Studio Code')).toBeNull();

    fireEvent.click(within(region).getByRole('button', { name: 'View Legacy CAD' }));
    expect(screen.getByTestId('view').textContent).toBe('applications');
    expect(screen.getByTestId('application-inspector').textContent).toContain('Legacy CAD');
  });

  it('the Ready and Need attention tiles open Apps with that filter', async () => {
    mockBackend((cmd) => (cmd === 'get_launch_history' ? [] : undefined));
    render(<Harness />);

    fireEvent.click(screen.getByRole('button', { name: /^2\s*Need attention/ }));
    expect(screen.getByTestId('view').textContent).toBe('applications');
    const group = screen.getByRole('group', { name: 'Filter by status' });
    expect(within(group).getByRole('button', { name: /^Needs attention/ }).getAttribute('aria-pressed')).toBe('true');
  });
});

describe('App on Home', () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => cleanup());

  it('Ctrl+K on Home switches to Apps and focuses its search', async () => {
    mockBackend((cmd) => {
      if (cmd === 'get_app_status')
        return { appName: 'DeskPilot', version: '0.1.1', status: 'ready', uptimeSeconds: 1, environment: 'development' };
      if (cmd === 'discover_applications')
        return { applications: apps, scannedSources: [], partialFailures: [], totalDiscovered: apps.length };
      if (cmd === 'get_launch_history') return [];
      if (cmd === 'get_system_metrics')
        return {
          cpu: { brandName: 'Test CPU', logicalCores: 8, architecture: 'x86_64', utilizationPercent: 12 },
          memory: { totalBytes: 16e9, availableBytes: 8e9, usedBytes: 8e9, usedPercent: 50 },
          storage: { drives: [] },
          system: { uptimeSeconds: 3600, appUptimeSeconds: 10, osName: 'Windows 11', architecture: 'x86_64' },
          sampledAtMs: 0,
        };
      return undefined;
    });
    render(<App />);
    expect(screen.getByRole('button', { name: 'Home' }).getAttribute('aria-current')).toBe('page');

    await act(async () => {
      fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
    });

    await waitFor(() => {
      expect(screen.getByRole('heading', { level: 1, name: /^apps$/i })).toBeDefined();
      expect(document.activeElement).toBe(screen.getByRole('searchbox', { name: /search applications/i }));
    });
  });
});
