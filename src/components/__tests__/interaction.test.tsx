// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react';
import { ApplicationCatalog } from '../ApplicationCatalog';
import { LaunchButton } from '../LaunchButton';
import App from '../../App';
import * as tauriCore from '@tauri-apps/api/core';
import type { ApplicationDto, DiscoveryResultDto, LaunchResultDto, AppStatusDto } from '../../lib/ipc';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('Frontend Component Interaction & Root Integration', () => {
  const sampleApps: ApplicationDto[] = [
    {
      id: 'app_00000000000000000000000000000001',
      displayName: 'Visual Studio Code',
      publisher: 'Microsoft Corporation',
      version: '1.95.0',
      sources: ['Registry (HKCU User)'],
      availability: 'Available',
      isInstalled: true,
    },
    {
      id: 'app_00000000000000000000000000000002',
      displayName: 'Google Chrome',
      publisher: 'Google LLC',
      version: '130.0.0.0',
      sources: ['Registry (HKLM 64-bit)', 'Start Menu (Common)'],
      availability: 'Available',
      isInstalled: true,
    },
    {
      id: 'app_00000000000000000000000000000003',
      displayName: 'Broken Tool',
      publisher: 'Unknown Publisher',
      sources: ['Start Menu (User)'],
      availability: 'MissingTarget',
      isInstalled: false,
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  describe('Search Input Interactive Filtering', () => {
    it('filters rendered application cards in real-time as user types', () => {
      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      // Initially all 3 applications are in the DOM
      expect(screen.getByText('Visual Studio Code')).toBeDefined();
      expect(screen.getByText('Google Chrome')).toBeDefined();
      expect(screen.getByText('Broken Tool')).toBeDefined();

      const searchInput = screen.getByRole('searchbox', { name: /search applications/i });

      // User types "chrome"
      fireEvent.change(searchInput, { target: { value: 'chrome' } });

      // Only Google Chrome is rendered; other cards are filtered out
      expect(screen.getByText('Google Chrome')).toBeDefined();
      expect(screen.queryByText('Visual Studio Code')).toBeNull();
      expect(screen.queryByText('Broken Tool')).toBeNull();

      // User clears search input
      fireEvent.change(searchInput, { target: { value: '' } });

      // All cards return
      expect(screen.getByText('Visual Studio Code')).toBeDefined();
      expect(screen.getByText('Google Chrome')).toBeDefined();
      expect(screen.getByText('Broken Tool')).toBeDefined();
    });

    it('renders empty filter state when search matches no applications', () => {
      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      const searchInput = screen.getByRole('searchbox', { name: /search applications/i });
      fireEvent.change(searchInput, { target: { value: 'unmatched query 999' } });

      expect(screen.getByText('No applications match your filter.')).toBeDefined();
      expect(screen.queryByText('Visual Studio Code')).toBeNull();
    });
  });

  describe('Refresh Button & Discovery Concurrency', () => {
    it('transitions to scanning state and updates catalog on resolution', async () => {
      const refreshedResult: DiscoveryResultDto = {
        applications: [
          {
            id: 'app_00000000000000000000000000000099',
            displayName: 'Refreshed New App',
            publisher: 'New Org',
            sources: ['Registry (HKLM 64-bit)'],
            availability: 'Available',
            isInstalled: true,
          },
        ],
        scannedSources: ['Registry (HKLM 64-bit)'],
        partialFailures: [],
        totalDiscovered: 1,
      };

      vi.mocked(tauriCore.invoke).mockResolvedValueOnce(refreshedResult);

      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      const refreshBtn = screen.getByRole('button', { name: /refresh catalog/i });
      expect(refreshBtn.textContent).toBe('Refresh Catalog');

      fireEvent.click(refreshBtn);

      // Button enters scanning state immediately
      expect(refreshBtn.textContent).toBe('Scanning...');
      expect((refreshBtn as HTMLButtonElement).disabled).toBe(true);

      // Awaits resolution and updates the DOM
      await waitFor(() => {
        expect(screen.getByText('Refreshed New App')).toBeDefined();
      });

      expect(refreshBtn.textContent).toBe('Refresh Catalog');
      expect((refreshBtn as HTMLButtonElement).disabled).toBe(false);
      expect(screen.queryByText('Visual Studio Code')).toBeNull();
    });
  });

  describe('LaunchButton Interaction & Double-Click Guard', () => {
    it('prevents duplicate launch calls while request is pending', async () => {
      let resolveLaunch!: (val: LaunchResultDto) => void;
      const pendingPromise = new Promise<LaunchResultDto>((resolve) => {
        resolveLaunch = resolve;
      });

      vi.mocked(tauriCore.invoke).mockReturnValueOnce(pendingPromise);

      render(<LaunchButton applicationId="app_1111" />);

      const launchBtn = screen.getByRole('button', { name: /launch application app_1111/i });
      expect(launchBtn.textContent).toBe('Launch');

      // First click
      fireEvent.click(launchBtn);

      // In-flight state
      await waitFor(() => {
        expect(launchBtn.textContent).toBe('Launching...');
      });
      expect((launchBtn as HTMLButtonElement).disabled).toBe(true);
      expect(launchBtn.getAttribute('aria-busy')).toBe('true');
      expect(tauriCore.invoke).toHaveBeenCalledTimes(1);

      // Second click while in-flight
      fireEvent.click(launchBtn);
      // Double click must be ignored
      expect(tauriCore.invoke).toHaveBeenCalledTimes(1);

      // Resolve launch
      resolveLaunch({ applicationId: 'app_1111', processId: 6543 });

      await waitFor(() => {
        expect(launchBtn.textContent).toBe('Launched (PID 6543)');
      });
      expect((launchBtn as HTMLButtonElement).disabled).toBe(false);
    });

    it('handles launch failure and allows recovery on subsequent click', async () => {
      vi.mocked(tauriCore.invoke).mockRejectedValueOnce({
        code: 'OPERATION_FAILED',
        message: 'Process could not be spawned',
      });

      render(<LaunchButton applicationId="app_2222" />);

      const launchBtn = screen.getByRole('button', { name: /launch application app_2222/i });

      fireEvent.click(launchBtn);

      await waitFor(() => {
        expect(launchBtn.textContent).toBe('Launch Failed');
      });

      expect(launchBtn.getAttribute('title')).toContain('Process could not be spawned');
      expect((launchBtn as HTMLButtonElement).disabled).toBe(false);

      // Subsequent click recovers and retries
      vi.mocked(tauriCore.invoke).mockResolvedValueOnce({
        applicationId: 'app_2222',
        processId: 1122,
      });

      fireEvent.click(launchBtn);

      await waitFor(() => {
        expect(launchBtn.textContent).toBe('Launched (PID 1122)');
      });
      expect(tauriCore.invoke).toHaveBeenCalledTimes(2);
    });
  });

  describe('Card Isolation', () => {
    it('launching one card does not disable or alter state of adjacent cards', async () => {
      let resolveApp1!: (val: LaunchResultDto) => void;
      const pendingApp1 = new Promise<LaunchResultDto>((resolve) => {
        resolveApp1 = resolve;
      });

      vi.mocked(tauriCore.invoke).mockReturnValueOnce(pendingApp1);

      render(
        <ApplicationCatalog
          initialApplications={sampleApps.slice(0, 2)}
          autoDiscover={false}
        />
      );

      const btn1 = screen.getByRole('button', {
        name: `Launch application ${sampleApps[0].id}`,
      });
      const btn2 = screen.getByRole('button', {
        name: `Launch application ${sampleApps[1].id}`,
      });

      // Launch app 1
      fireEvent.click(btn1);

      // Button 1 is in-flight
      await waitFor(() => {
        expect(btn1.textContent).toBe('Launching...');
      });
      expect((btn1 as HTMLButtonElement).disabled).toBe(true);

      // Button 2 remains completely idle and clickable
      expect(btn2.textContent).toBe('Launch');
      expect((btn2 as HTMLButtonElement).disabled).toBe(false);

      resolveApp1({ applicationId: sampleApps[0].id, processId: 1001 });
      await waitFor(() => {
        expect(btn1.textContent).toBe('Launched (PID 1001)');
      });
    });
  });

  describe('Root UI Integration (App.tsx)', () => {
    it('renders the root App component with ApplicationCatalog and header', async () => {
      const mockStatus: AppStatusDto = {
        appName: 'DeskPilot',
        version: '0.1.0',
        status: 'ready',
        uptimeSeconds: 60,
        environment: 'development',
      };

      const mockDiscovery: DiscoveryResultDto = {
        applications: sampleApps,
        scannedSources: ['Registry (HKLM 64-bit)'],
        partialFailures: [],
        totalDiscovered: 3,
      };

      vi.mocked(tauriCore.invoke).mockImplementation(async (cmd: string) => {
        if (cmd === 'get_app_status') return mockStatus;
        if (cmd === 'discover_applications') return mockDiscovery;
        throw new Error(`Unexpected command ${cmd}`);
      });

      render(<App />);

      // Verifies root header is present
      expect(screen.getByRole('heading', { level: 1, name: /deskpilot application discovery/i })).toBeDefined();

      // Verifies AppStatus header subtitle loads
      await waitFor(() => {
        expect(screen.getByText(/Core v0.1.0/)).toBeDefined();
      });

      // Verifies ApplicationCatalog is rendered under App
      await waitFor(() => {
        expect(screen.getByText('Visual Studio Code')).toBeDefined();
        expect(screen.getByText('Google Chrome')).toBeDefined();
      });

      // Verifies launch buttons are rendered
      expect(screen.getAllByRole('button', { name: /launch application/i })).toHaveLength(3);
    });
  });

  describe('Keyboard Navigation & Catalog Hotkeys', () => {
    it('auto-focuses search input on initial catalog ready state', async () => {
      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      const searchInput = screen.getByRole('searchbox', { name: /search applications/i });
      expect(document.activeElement).toBe(searchInput);
    });

    it('clears active search query when user presses Escape in search input', () => {
      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      const searchInput = screen.getByRole('searchbox', { name: /search applications/i });
      fireEvent.change(searchInput, { target: { value: 'chrome' } });
      expect((searchInput as HTMLInputElement).value).toBe('chrome');

      // Press Escape
      fireEvent.keyDown(searchInput, { key: 'Escape' });
      expect((searchInput as HTMLInputElement).value).toBe('');
      // All apps restored
      expect(screen.getByText('Visual Studio Code')).toBeDefined();
    });

    it('launches first available application when user presses Enter in search input', async () => {
      vi.mocked(tauriCore.invoke).mockResolvedValueOnce({
        applicationId: sampleApps[1].id,
        processId: 4444,
      });

      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      const searchInput = screen.getByRole('searchbox', { name: /search applications/i });
      fireEvent.change(searchInput, { target: { value: 'chrome' } });

      // User presses Enter in search input
      fireEvent.keyDown(searchInput, { key: 'Enter' });

      await waitFor(() => {
        expect(tauriCore.invoke).toHaveBeenCalledWith('launch_application', {
          applicationId: sampleApps[1].id,
        });
      });
    });

    it('does not launch any application on Enter when no available application matches', async () => {
      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      const searchInput = screen.getByRole('searchbox', { name: /search applications/i });
      // "broken" matches sampleApps[2], which has availability='MissingTarget'
      fireEvent.change(searchInput, { target: { value: 'broken' } });

      fireEvent.keyDown(searchInput, { key: 'Enter' });

      // No launch invoke should be dispatched
      expect(tauriCore.invoke).not.toHaveBeenCalledWith('launch_application', expect.anything());
    });

    it('navigates focus between search input and cards using arrow keys', () => {
      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      const searchInput = screen.getByRole('searchbox', { name: /search applications/i });
      const cards = screen.getAllByRole('article');

      expect(document.activeElement).toBe(searchInput);

      // ArrowDown from search input moves focus to first card
      fireEvent.keyDown(searchInput, { key: 'ArrowDown' });
      expect(document.activeElement).toBe(cards[0]);

      // ArrowRight moves focus to second card
      fireEvent.keyDown(cards[0], { key: 'ArrowRight' });
      expect(document.activeElement).toBe(cards[1]);

      // ArrowDown moves focus to third card
      fireEvent.keyDown(cards[1], { key: 'ArrowDown' });
      expect(document.activeElement).toBe(cards[2]);

      // ArrowUp moves focus back to second card
      fireEvent.keyDown(cards[2], { key: 'ArrowUp' });
      expect(document.activeElement).toBe(cards[1]);

      // ArrowLeft moves focus to first card
      fireEvent.keyDown(cards[1], { key: 'ArrowLeft' });
      expect(document.activeElement).toBe(cards[0]);

      // ArrowUp on first card returns focus to search input
      fireEvent.keyDown(cards[0], { key: 'ArrowUp' });
      expect(document.activeElement).toBe(searchInput);
    });

    it('launches application when pressing Enter on a focused card', async () => {
      vi.mocked(tauriCore.invoke).mockResolvedValueOnce({
        applicationId: sampleApps[0].id,
        processId: 5555,
      });

      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      const cards = screen.getAllByRole('article');
      cards[0].focus();
      expect(document.activeElement).toBe(cards[0]);

      // Press Enter on the card
      fireEvent.keyDown(cards[0], { key: 'Enter' });

      await waitFor(() => {
        expect(tauriCore.invoke).toHaveBeenCalledWith('launch_application', {
          applicationId: sampleApps[0].id,
        });
      });
    });
  });

  describe('Catalog & Process Monitor View Switching', () => {
    it('switches between Applications and Active Processes views cleanly', async () => {
      vi.mocked(tauriCore.invoke).mockImplementation(async (cmd: string) => {
        if (cmd === 'list_processes') {
          return {
            processes: [
              {
                id: 'v1:1234:01db2d5843a0e120',
                pid: 1234,
                parentPid: 500,
                name: 'deskpilot.exe',
                executablePath: 'C:\\DeskPilot\\deskpilot.exe',
                createdAt: '2026-11-03T12:00:00.000Z',
                workingSetBytes: 52428800,
                cpuTimeMs: 1200,
                sessionId: 1,
                status: 'full',
              },
            ],
            totalProcesses: 1,
            capturedAtMs: 1730678400000,
            isTruncated: false,
          };
        }
        if (cmd === 'discover_applications') {
          return {
            applications: sampleApps,
            scannedSources: ['Registry (HKLM 64-bit)'],
            partialFailures: [],
            totalDiscovered: 3,
          };
        }
        throw new Error(`Unexpected command: ${cmd}`);
      });

      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      // Initially on Applications view
      const appTab = screen.getByRole('tab', { name: 'Applications' });
      const procTab = screen.getByRole('tab', { name: 'Active Processes' });

      expect(appTab.getAttribute('aria-selected')).toBe('true');
      expect(procTab.getAttribute('aria-selected')).toBe('false');
      expect(screen.getByText('Visual Studio Code')).toBeDefined();
      expect(screen.queryByText('deskpilot.exe')).toBeNull();

      // Click "Active Processes" tab
      fireEvent.click(procTab);

      expect(appTab.getAttribute('aria-selected')).toBe('false');
      expect(procTab.getAttribute('aria-selected')).toBe('true');

      // Process Monitor is rendered and fetches processes
      await waitFor(() => {
        expect(screen.getByText('deskpilot.exe')).toBeDefined();
      });
      expect(screen.getByRole('columnheader', { name: /process/i })).toBeDefined();

      // Application cards are not in DOM while on Processes tab
      expect(screen.queryByText('Visual Studio Code')).toBeNull();

      // Click "Applications" tab to return
      fireEvent.click(appTab);

      expect(appTab.getAttribute('aria-selected')).toBe('true');
      expect(procTab.getAttribute('aria-selected')).toBe('false');
      expect(screen.getByText('Visual Studio Code')).toBeDefined();
      expect(screen.queryByText('deskpilot.exe')).toBeNull();
    });
  });
});
