// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup, act } from '@testing-library/react';
import { ApplicationCatalog } from '../ApplicationCatalog';
import { ApplicationCard } from '../ApplicationCard';
import { LaunchButton } from '../LaunchButton';
import App from '../../App';
import * as tauriCore from '@tauri-apps/api/core';
import {
  IpcError,
  type ApplicationDto,
  type DiscoveryResultDto,
  type LaunchResultDto,
  type AppStatusDto,
} from '../../lib/ipc';

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
    vi.useRealTimers();
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
      expect(screen.getAllByText('Visual Studio Code').length).toBeGreaterThanOrEqual(1);
      expect(screen.getByText('Google Chrome')).toBeDefined();
      expect(screen.getByText('Broken Tool')).toBeDefined();

      const searchInput = screen.getByRole('searchbox', { name: /search applications/i });

      // User types "chrome"
      fireEvent.change(searchInput, { target: { value: 'chrome' } });

      // Only Google Chrome is rendered; other cards are filtered out
      expect(screen.getAllByText('Google Chrome').length).toBeGreaterThanOrEqual(1);
      expect(screen.queryByText('Visual Studio Code')).toBeNull();
      expect(screen.queryByText('Broken Tool')).toBeNull();

      // User clears search input
      fireEvent.change(searchInput, { target: { value: '' } });

      // All cards return
      expect(screen.getAllByText('Visual Studio Code').length).toBeGreaterThanOrEqual(1);
      expect(screen.getAllByText('Google Chrome').length).toBeGreaterThanOrEqual(1);
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
        <div>
          <ApplicationCard application={sampleApps[0]} showLaunchButton={true} />
          <ApplicationCard application={sampleApps[1]} showLaunchButton={true} />
        </div>
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
        version: '0.1.1',
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

      // Verifies root header is present with dynamic view title
      expect(screen.getByRole('heading', { level: 1, name: /application discovery/i })).toBeDefined();

      // Verifies AppStatus header subtitle loads with synchronized v0.1.1
      await waitFor(() => {
        expect(screen.getByText(/Core v0.1.1/)).toBeDefined();
      });

      // Verifies ApplicationCatalog is rendered under App
      await waitFor(() => {
        expect(screen.getByText('Visual Studio Code')).toBeDefined();
        expect(screen.getByText('Google Chrome')).toBeDefined();
      });

      // Verifies primary launch button is rendered in inspector for selected application
      expect(screen.getByRole('button', { name: `Launch ${sampleApps[0].displayName}` })).toBeDefined();

      // Verifies persistent sidebar navigation is present
      const nav = screen.getByRole('navigation', { name: /views/i });
      expect(nav).toBeDefined();

      const appNavBtn = screen.getByRole('button', { name: 'Applications' });
      const procNavBtn = screen.getByRole('button', { name: 'Active Processes' });
      const sysNavBtn = screen.getByRole('button', { name: 'System Intelligence' });

      expect(appNavBtn.getAttribute('aria-current')).toBe('page');
      expect(procNavBtn.getAttribute('aria-current')).toBeNull();
      expect(sysNavBtn.getAttribute('aria-current')).toBeNull();

      // Verifies brand and version in sidebar
      expect(screen.getByText('DeskPilot')).toBeDefined();
      expect(screen.getByText('v0.1.1')).toBeDefined();
    });

    it('switches views and updates aria-current when sidebar buttons are clicked', async () => {
      vi.mocked(tauriCore.invoke).mockImplementation(async (cmd: string) => {
        if (cmd === 'get_app_status') {
          return {
            appName: 'DeskPilot',
            version: '0.1.1',
            status: 'ready',
            uptimeSeconds: 120,
            environment: 'development',
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
        if (cmd === 'list_processes') {
          return {
            processes: [],
            totalProcesses: 0,
            capturedAtMs: Date.now(),
            isTruncated: false,
          };
        }
        if (cmd === 'get_system_metrics') {
          return {
            cpu: { brandName: 'Intel', logicalCores: 8, architecture: 'x86_64', utilizationPercent: 10 },
            memory: { totalBytes: 16000000000, availableBytes: 8000000000, usedBytes: 8000000000, usedPercent: 50 },
            storage: { drives: [] },
            system: { uptimeSeconds: 1000, appUptimeSeconds: 100, osName: 'Windows', architecture: 'x86_64' },
            sampledAtMs: Date.now(),
          };
        }
        throw new Error(`Unexpected command ${cmd}`);
      });

      render(<App />);

      const procNavBtn = screen.getByRole('button', { name: 'Active Processes' });
      fireEvent.click(procNavBtn);

      await waitFor(() => {
        expect(procNavBtn.getAttribute('aria-current')).toBe('page');
        expect(screen.getByRole('region', { name: /process monitor/i })).toBeDefined();
        expect(screen.getByRole('heading', { level: 1, name: /process monitor/i })).toBeDefined();
      });

      const sysNavBtn = screen.getByRole('button', { name: 'System Intelligence' });
      fireEvent.click(sysNavBtn);

      await waitFor(() => {
        expect(sysNavBtn.getAttribute('aria-current')).toBe('page');
        expect(screen.getAllByRole('region', { name: /system intelligence/i }).length).toBeGreaterThanOrEqual(1);
        expect(screen.getByRole('heading', { level: 1, name: /system intelligence/i })).toBeDefined();
      });
    });

    it('focuses the search input when global Ctrl+K shortcut is triggered', async () => {
      vi.mocked(tauriCore.invoke).mockImplementation(async (cmd: string) => {
        if (cmd === 'get_app_status') {
          return {
            appName: 'DeskPilot',
            version: '0.1.1',
            status: 'ready',
            uptimeSeconds: 120,
            environment: 'development',
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
        throw new Error(`Unexpected command ${cmd}`);
      });

      render(<App />);

      await waitFor(() => {
        expect(screen.getByText('Visual Studio Code')).toBeDefined();
      });

      const searchInput = screen.getByRole('searchbox', { name: /search applications/i }) as HTMLInputElement;

      // Blur input first
      searchInput.blur();
      expect(document.activeElement).not.toBe(searchInput);

      // Trigger Ctrl+K
      act(() => {
        fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
      });

      expect(document.activeElement).toBe(searchInput);
    });

    it('focuses search input when global search trigger button in header is clicked', async () => {
      vi.mocked(tauriCore.invoke).mockImplementation(async (cmd: string) => {
        if (cmd === 'get_app_status') {
          return {
            appName: 'DeskPilot',
            version: '0.1.1',
            status: 'ready',
            uptimeSeconds: 120,
            environment: 'development',
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
        throw new Error(`Unexpected command ${cmd}`);
      });

      render(<App />);

      await waitFor(() => {
        expect(screen.getByText('Visual Studio Code')).toBeDefined();
      });

      const searchInput = screen.getByRole('searchbox', { name: /search applications/i }) as HTMLInputElement;
      searchInput.blur();
      expect(document.activeElement).not.toBe(searchInput);

      const searchTrigger = screen.getByRole('button', { name: /search applications \(ctrl\+k\)/i });
      fireEvent.click(searchTrigger);

      expect(document.activeElement).toBe(searchInput);
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

    it('launches first available application on Enter when search filter is empty', async () => {
      vi.mocked(tauriCore.invoke).mockResolvedValueOnce({
        applicationId: sampleApps[0].id,
        processId: 7777,
      });

      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      const searchInput = screen.getByRole('searchbox', { name: /search applications/i });
      fireEvent.keyDown(searchInput, { key: 'Enter' });

      await waitFor(() => {
        expect(tauriCore.invoke).toHaveBeenCalledWith('launch_application', {
          applicationId: sampleApps[0].id,
        });
      });
    });

    it('delivers correct application ID to backend IPC and onLaunchSuccess handler without DOM querying', async () => {
      vi.mocked(tauriCore.invoke).mockResolvedValueOnce({
        applicationId: sampleApps[1].id,
        processId: 8888,
      });

      const onLaunchSuccess = vi.fn();

      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
          onLaunchSuccess={onLaunchSuccess}
        />
      );

      const searchInput = screen.getByRole('searchbox', { name: /search applications/i });
      fireEvent.change(searchInput, { target: { value: 'chrome' } });
      fireEvent.keyDown(searchInput, { key: 'Enter' });

      await waitFor(() => {
        expect(tauriCore.invoke).toHaveBeenCalledWith('launch_application', {
          applicationId: sampleApps[1].id,
        });
        expect(onLaunchSuccess).toHaveBeenCalledWith(sampleApps[1].id, 8888);
      });
    });

    it('displays launching state on corresponding card LaunchButton while Enter-key launch is in-flight', async () => {
      let resolveLaunch!: (val: LaunchResultDto) => void;
      const pendingPromise = new Promise<LaunchResultDto>((resolve) => {
        resolveLaunch = resolve;
      });

      vi.mocked(tauriCore.invoke).mockReturnValueOnce(pendingPromise);

      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      const searchInput = screen.getByRole('searchbox', { name: /search applications/i });
      fireEvent.change(searchInput, { target: { value: 'chrome' } });
      fireEvent.keyDown(searchInput, { key: 'Enter' });

      const chromeBtn = screen.getByRole('button', {
        name: `Launch ${sampleApps[1].displayName}`,
      });

      await waitFor(() => {
        expect(chromeBtn.textContent).toBe('Launching...');
      });
      expect(chromeBtn.classList.contains('launching')).toBe(true);
      expect((chromeBtn as HTMLButtonElement).disabled).toBe(true);
      expect(chromeBtn.getAttribute('aria-busy')).toBe('true');

      resolveLaunch({ applicationId: sampleApps[1].id, processId: 4444 });
      await waitFor(() => {
        expect(chromeBtn.textContent).toBe('Launched (PID 4444)');
      });
    });

    it('displays launched feedback with process ID and auto-resets on successful Enter launch', async () => {
      vi.useFakeTimers();
      vi.mocked(tauriCore.invoke).mockResolvedValueOnce({
        applicationId: sampleApps[1].id,
        processId: 5555,
      });

      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      const searchInput = screen.getByRole('searchbox', { name: /search applications/i });
      fireEvent.change(searchInput, { target: { value: 'chrome' } });

      await act(async () => {
        fireEvent.keyDown(searchInput, { key: 'Enter' });
      });

      const chromeBtn = screen.getByRole('button', {
        name: `Launch ${sampleApps[1].displayName}`,
      });

      expect(chromeBtn.textContent).toBe('Launched (PID 5555)');
      expect(chromeBtn.classList.contains('launched')).toBe(true);

      // Fast-forward past 3000ms auto-reset
      act(() => {
        vi.advanceTimersByTime(3000);
      });

      expect(chromeBtn.textContent).toBe('Launch');
      expect(chromeBtn.classList.contains('idle')).toBe(true);
    });

    it('displays failed feedback and error tooltip on failed Enter launch', async () => {
      vi.mocked(tauriCore.invoke).mockRejectedValueOnce({
        code: 'OPERATION_FAILED',
        message: 'Executable spawn failed in test environment',
      });

      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      const searchInput = screen.getByRole('searchbox', { name: /search applications/i });
      fireEvent.change(searchInput, { target: { value: 'chrome' } });
      fireEvent.keyDown(searchInput, { key: 'Enter' });

      const chromeBtn = screen.getByRole('button', {
        name: `Launch ${sampleApps[1].displayName}`,
      });

      await waitFor(() => {
        expect(chromeBtn.textContent).toBe('Launch Failed');
      });
      expect(chromeBtn.classList.contains('failed')).toBe(true);
      expect(chromeBtn.getAttribute('title')).toContain('Executable spawn failed in test environment');
    });

    it('clicking the inspector LaunchButton shares the consolidated launcher state', async () => {
      vi.mocked(tauriCore.invoke).mockResolvedValueOnce({
        applicationId: sampleApps[0].id,
        processId: 3333,
      });

      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      const vscodeBtn = screen.getByRole('button', {
        name: `Launch ${sampleApps[0].displayName}`,
      });

      fireEvent.click(vscodeBtn);

      await waitFor(() => {
        expect(vscodeBtn.textContent).toBe('Launched (PID 3333)');
      });
      expect(tauriCore.invoke).toHaveBeenCalledWith('launch_application', {
        applicationId: sampleApps[0].id,
      });
    });

    it('propagates launch errors to onLaunchError handler on Enter', async () => {
      vi.mocked(tauriCore.invoke).mockRejectedValueOnce({
        code: 'PERMISSION_DENIED',
        message: 'Launch forbidden by security policy',
      });

      const onLaunchError = vi.fn();

      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
          onLaunchError={onLaunchError}
        />
      );

      const searchInput = screen.getByRole('searchbox', { name: /search applications/i });
      fireEvent.change(searchInput, { target: { value: 'chrome' } });
      fireEvent.keyDown(searchInput, { key: 'Enter' });

      await waitFor(() => {
        expect(onLaunchError).toHaveBeenCalledTimes(1);
        const [appId, err] = onLaunchError.mock.calls[0];
        expect(appId).toBe(sampleApps[1].id);
        expect(err).toBeInstanceOf(IpcError);
        expect(err.code).toBe('PERMISSION_DENIED');
        expect(err.message).toBe('Launch forbidden by security policy');
      });
    });

    it('prevents duplicate concurrent launches when Enter is pressed repeatedly while in-flight', async () => {
      let resolveLaunch!: (val: LaunchResultDto) => void;
      const pendingPromise = new Promise<LaunchResultDto>((resolve) => {
        resolveLaunch = resolve;
      });

      vi.mocked(tauriCore.invoke).mockReturnValueOnce(pendingPromise);

      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      const searchInput = screen.getByRole('searchbox', { name: /search applications/i });
      fireEvent.change(searchInput, { target: { value: 'chrome' } });

      // First Enter
      fireEvent.keyDown(searchInput, { key: 'Enter' });
      expect(tauriCore.invoke).toHaveBeenCalledTimes(1);

      // Rapid second and third Enter presses while in-flight
      fireEvent.keyDown(searchInput, { key: 'Enter' });
      fireEvent.keyDown(searchInput, { key: 'Enter' });
      expect(tauriCore.invoke).toHaveBeenCalledTimes(1);

      // Resolve pending launch
      resolveLaunch({
        applicationId: sampleApps[1].id,
        processId: 9999,
      });

      await waitFor(() => {
        expect(tauriCore.invoke).toHaveBeenCalledTimes(1);
      });
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

  describe('Master-Detail Interaction & Source Filtering', () => {
    it('clicking an application card selects it and updates ApplicationInspector', async () => {
      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      // Initially first app (Visual Studio Code) is selected in Inspector
      const inspector = screen.getByTestId('application-inspector');
      expect(inspector).toBeDefined();
      expect(inspector.textContent).toContain('Visual Studio Code');

      // Click on Google Chrome card
      const cards = screen.getAllByRole('article');
      fireEvent.click(cards[1]);

      // Google Chrome is now selected
      expect(cards[1].classList.contains('selected')).toBe(true);
      expect(inspector.textContent).toContain('Google Chrome');
      expect(inspector.textContent).toContain('Google LLC');
      expect(inspector.textContent).toContain('v130.0.0.0');
    });

    it('clicking a source filter pill filters the application list and updates counts', async () => {
      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      // Initially 3 applications
      expect(screen.getByText('Showing 3 applications')).toBeDefined();

      // Click "HKLM 64-bit" pill
      const hklmPill = screen.getByRole('button', { name: 'HKLM 64-bit' });
      fireEvent.click(hklmPill);

      // Only Google Chrome has HKLM 64-bit in sampleApps
      expect(screen.getByText(/filtered from 3 total/i)).toBeDefined();
      const cards = screen.getAllByRole('article');
      expect(cards).toHaveLength(1);
      expect(cards[0].textContent).toContain('Google Chrome');

      // Return to All
      const allPill = screen.getByRole('button', { name: 'All' });
      fireEvent.click(allPill);
      expect(screen.getAllByRole('article')).toHaveLength(3);
    });

    it('arrow key navigation updates selection and ApplicationInspector synchronization', () => {
      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      const cards = screen.getAllByRole('article');
      const inspector = screen.getByTestId('application-inspector');

      // Card 0 is selected initially
      expect(inspector.textContent).toContain('Visual Studio Code');

      // Press ArrowDown on Card 0
      fireEvent.keyDown(cards[0], { key: 'ArrowDown' });

      // Selection moves to Card 1 (Google Chrome)
      expect(inspector.textContent).toContain('Google Chrome');

      // Press ArrowUp on Card 1
      fireEvent.keyDown(cards[1], { key: 'ArrowUp' });

      // Selection moves back to Card 0
      expect(inspector.textContent).toContain('Visual Studio Code');
    });

    it('reconciles selection when active filter excludes currently selected application', () => {
      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      const inspector = screen.getByTestId('application-inspector');
      // Visual Studio Code is initially selected
      expect(inspector.textContent).toContain('Visual Studio Code');

      // Click "HKLM 64-bit" which excludes Visual Studio Code (HKCU only)
      const hklmPill = screen.getByRole('button', { name: 'HKLM 64-bit' });
      fireEvent.click(hklmPill);

      // Reconciles to Google Chrome (the only matching app)
      expect(inspector.textContent).toContain('Google Chrome');
    });

    it('launches selected application from the ApplicationInspector primary launch button', async () => {
      vi.mocked(tauriCore.invoke).mockResolvedValueOnce({
        applicationId: sampleApps[1].id,
        processId: 8899,
      });

      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      // Select Google Chrome
      const cards = screen.getAllByRole('article');
      fireEvent.click(cards[1]);

      // Launch from inspector button
      const inspectorLaunchBtn = screen.getByRole('button', { name: 'Launch Google Chrome' });
      fireEvent.click(inspectorLaunchBtn);

      await waitFor(() => {
        expect(tauriCore.invoke).toHaveBeenCalledWith('launch_application', {
          applicationId: sampleApps[1].id,
        });
      });
    });
  });

  describe('Workspace View Routing & Context-Aware Search (F-01, F-02, F-03)', () => {
    it('renders clean catalog interface without redundant internal view switcher tabs', () => {
      render(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      // Verify no obsolete internal tab switcher exists in ApplicationCatalog (F-01)
      expect(screen.queryByRole('tablist')).toBeNull();
      expect(screen.queryByRole('tab')).toBeNull();
    });

    it('manages context-aware search input focus across views and protects against form input hijacking', async () => {
      vi.mocked(tauriCore.invoke).mockImplementation(async (cmd: string) => {
        if (cmd === 'get_app_status') {
          return {
            appName: 'DeskPilot',
            version: '0.1.1',
            status: 'ready',
            uptimeSeconds: 120,
            environment: 'development',
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
        if (cmd === 'get_system_metrics') {
          return {
            cpu: { brandName: 'Intel', logicalCores: 8, architecture: 'x86_64', utilizationPercent: 10 },
            memory: { totalBytes: 16000000000, availableBytes: 8000000000, usedBytes: 8000000000, usedPercent: 50 },
            storage: { drives: [] },
            system: { uptimeSeconds: 1000, appUptimeSeconds: 100, osName: 'Windows', architecture: 'x86_64' },
            sampledAtMs: Date.now(),
          };
        }
        throw new Error(`Unexpected command: ${cmd}`);
      });

      render(<App />);

      // 1. In Applications view: Ctrl+K focuses #catalog-search-input
      await waitFor(() => {
        expect(screen.getByText('Visual Studio Code')).toBeDefined();
      });
      const catalogSearch = screen.getByRole('searchbox', { name: /search applications/i });
      catalogSearch.blur();
      expect(document.activeElement).not.toBe(catalogSearch);

      act(() => {
        fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
      });
      expect(document.activeElement).toBe(catalogSearch);

      // 2. In Processes view: Ctrl+K focuses #process-search-input
      const procNavBtn = screen.getByRole('button', { name: 'Active Processes' });
      fireEvent.click(procNavBtn);

      await waitFor(() => {
        expect(screen.getByRole('searchbox', { name: /filter processes/i })).toBeDefined();
      });
      const procSearch = screen.getByRole('searchbox', { name: /filter processes/i });
      procSearch.blur();
      expect(document.activeElement).not.toBe(procSearch);

      act(() => {
        fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
      });
      expect(document.activeElement).toBe(procSearch);

      // 3. In System view: search trigger button is disabled and Ctrl+K does nothing
      const sysNavBtn = screen.getByRole('button', { name: 'System Intelligence' });
      fireEvent.click(sysNavBtn);

      await waitFor(() => {
        expect(screen.getByRole('heading', { level: 1, name: /system intelligence/i })).toBeDefined();
      });
      const headerSearchBtn = document.querySelector('button.global-search-trigger') as HTMLButtonElement;
      expect(headerSearchBtn).toBeDefined();
      expect(headerSearchBtn.disabled).toBe(true);

      // 4. Input-focus guard: typing Ctrl+K inside an arbitrary input element does not hijack focus or prevent default
      const arbitraryInput = document.createElement('input');
      arbitraryInput.id = 'arbitrary-field';
      document.body.appendChild(arbitraryInput);
      arbitraryInput.focus();
      expect(document.activeElement).toBe(arbitraryInput);

      // Keystroke while focused in an arbitrary form field
      const defaultPrevented = fireEvent.keyDown(arbitraryInput, { key: 'k', ctrlKey: true });
      // Guard returns early: default is NOT prevented (fireEvent returns true) and focus is preserved
      expect(defaultPrevented).toBe(true);
      expect(document.activeElement).toBe(arbitraryInput);
      document.body.removeChild(arbitraryInput);
    });
  });
});
