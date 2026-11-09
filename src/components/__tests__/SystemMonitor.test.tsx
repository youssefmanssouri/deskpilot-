// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react';
import {
  SystemMonitor,
  formatBytes,
  formatUptime,
  getLoadColorClass,
} from '../SystemMonitor';
import { CpuSparkline } from '../CpuSparkline';
import {
  SystemMetricsProvider,
  useSharedSystemMetrics,
  appendCpuSample,
  MAX_CPU_HISTORY_SAMPLES,
} from '../useSystemMetrics';
import * as tauriCore from '@tauri-apps/api/core';
import type { SystemMetricsDto } from '../../lib/ipc';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const mockSystemData: SystemMetricsDto = {
  cpu: {
    brandName: 'AMD Ryzen 7 5800X 8-Core Processor',
    logicalCores: 16,
    architecture: 'x86_64',
    utilizationPercent: 24.5,
  },
  memory: {
    totalBytes: 34359738368, // 32 GiB
    availableBytes: 17179869184, // 16 GiB
    usedBytes: 17179869184,
    usedPercent: 50.0,
  },
  storage: {
    drives: [
      {
        rootPath: 'C:\\',
        volumeName: 'System',
        fileSystem: 'NTFS',
        totalBytes: 1073741824000, // 1 TB
        availableBytes: 536870912000, // 500 GB
        usedBytes: 536870912000,
        usedPercent: 50.0,
      },
      {
        rootPath: 'D:\\',
        volumeName: 'Games',
        fileSystem: 'NTFS',
        totalBytes: 2147483648000, // 2 TB
        availableBytes: 107374182400, // 100 GB (~4.6% free)
        usedBytes: 2040109465600,
        usedPercent: 95.0,
      },
    ],
  },
  system: {
    uptimeSeconds: 90061, // 1d 1h 1m 1s
    appUptimeSeconds: 125, // 2m 5s
    osName: 'Windows',
    architecture: 'x86_64',
  },
  sampledAtMs: 1730680000000,
};

describe('SystemMonitor Utility Functions', () => {
  describe('formatBytes', () => {
    it('handles null, undefined, and NaN gracefully', () => {
      expect(formatBytes(null)).toBe('—');
      expect(formatBytes(undefined)).toBe('—');
      expect(formatBytes(NaN)).toBe('—');
    });

    it('formats bytes, KB, MB, GB, and TB correctly', () => {
      expect(formatBytes(0)).toBe('0 B');
      expect(formatBytes(512)).toBe('512 B');
      expect(formatBytes(1024)).toBe('1.0 KB');
      expect(formatBytes(1536)).toBe('1.5 KB');
      expect(formatBytes(1048576)).toBe('1.0 MB');
      expect(formatBytes(1073741824)).toBe('1.0 GB');
      expect(formatBytes(1099511627776)).toBe('1.00 TB');
    });
  });

  describe('formatUptime', () => {
    it('handles invalid inputs', () => {
      expect(formatUptime(null)).toBe('—');
      expect(formatUptime(undefined)).toBe('—');
      expect(formatUptime(-1)).toBe('—');
    });

    it('formats seconds, minutes, hours, and days', () => {
      expect(formatUptime(45)).toBe('45s');
      expect(formatUptime(125)).toBe('2m 5s');
      expect(formatUptime(3665)).toBe('1h 1m 5s');
      expect(formatUptime(90061)).toBe('1d 1h 1m');
    });
  });

  describe('getLoadColorClass', () => {
    it('returns load-normal for values below 60', () => {
      expect(getLoadColorClass(0)).toBe('load-normal');
      expect(getLoadColorClass(59.9)).toBe('load-normal');
    });

    it('returns load-warning for values between 60 and 84.9', () => {
      expect(getLoadColorClass(60)).toBe('load-warning');
      expect(getLoadColorClass(84.9)).toBe('load-warning');
    });

    it('returns load-critical for values 85 and above', () => {
      expect(getLoadColorClass(85)).toBe('load-critical');
      expect(getLoadColorClass(99.9)).toBe('load-critical');
    });
  });
});

describe('SystemMonitor Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it('renders loading state initially when autoFetch=true without initialData', () => {
    const pending = new Promise(() => {});
    vi.mocked(tauriCore.invoke).mockReturnValueOnce(pending as any);

    render(<SystemMonitor autoFetch={true} />);
    expect(screen.getByText('Sampling system metrics from Windows...')).toBeDefined();
  });

  it('renders populated metrics cards with initialData', () => {
    render(<SystemMonitor initialData={mockSystemData} autoFetch={false} />);

    // CPU Card
    expect(screen.getByText('Processor (CPU)')).toBeDefined();
    expect(screen.getByText('24.5%')).toBeDefined();
    expect(screen.getByText('16 Logical Cores')).toBeDefined();
    expect(screen.getByText('AMD Ryzen 7 5800X 8-Core Processor')).toBeDefined();

    // RAM Card
    expect(screen.getByText('Memory (RAM)')).toBeDefined();
    expect(screen.getByText('16.0 GB available for processes')).toBeDefined();

    // Storage Card
    expect(screen.getByText('Local Storage')).toBeDefined();
    expect(screen.getByText('2 Volumes')).toBeDefined();

    // Drive details
    expect(screen.getByText('C:\\')).toBeDefined();
    expect(screen.getByText('(System)')).toBeDefined();
    expect(screen.getByText('D:\\')).toBeDefined();
    expect(screen.getByText('(Games)')).toBeDefined();
    expect(screen.getByText('Low Space')).toBeDefined(); // D: is 95% used

    // Uptime Card
    expect(screen.getByText('System Uptime')).toBeDefined();
    expect(screen.getByText('1d 1h 1m')).toBeDefined();
  });

  it('fetches metrics automatically on mount and updates state', async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockSystemData);

    render(<SystemMonitor autoFetch={true} />);

    await waitFor(() => {
      expect(screen.getByText('24.5%')).toBeDefined();
    });

    expect(tauriCore.invoke).toHaveBeenCalledWith('get_system_metrics', undefined);
  });

  it('handles IPC errors and allows retry', async () => {
    vi.mocked(tauriCore.invoke).mockRejectedValueOnce({
      code: 'UNAVAILABLE',
      message: 'Win32 provider unavailable',
    });

    render(<SystemMonitor autoFetch={true} />);

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeDefined();
      expect(screen.getByText('Unable to retrieve system metrics')).toBeDefined();
      expect(screen.getByText('Win32 provider unavailable')).toBeDefined();
    });

    // Mock next invocation for retry
    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockSystemData);

    // Click retry
    const retryBtn = screen.getByRole('button', { name: /retry loading metrics/i });
    fireEvent.click(retryBtn);

    await waitFor(() => {
      expect(screen.getByText('24.5%')).toBeDefined();
    });
  });

  it('triggers manual refresh on button click', async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockSystemData);

    render(<SystemMonitor initialData={mockSystemData} autoFetch={false} />);

    const refreshBtn = screen.getByRole('button', { name: /refresh system metrics/i });
    fireEvent.click(refreshBtn);

    await waitFor(() => {
      expect(tauriCore.invoke).toHaveBeenCalledWith('get_system_metrics', undefined);
    });
  });

  it('allows changing refresh interval mode in dropdown', () => {
    render(<SystemMonitor initialData={mockSystemData} autoFetch={false} />);

    const select = screen.getByLabelText('Select auto-refresh interval') as HTMLSelectElement;
    expect(select.value).toBe('manual');

    fireEvent.change(select, { target: { value: '3s' } });
    expect(select.value).toBe('3s');

    fireEvent.change(select, { target: { value: '5s' } });
    expect(select.value).toBe('5s');
  });

  it('handles empty storage drives list gracefully with empty state notice', () => {
    const dataNoDrives: SystemMetricsDto = {
      ...mockSystemData,
      storage: { drives: [] },
    };

    render(<SystemMonitor initialData={dataNoDrives} autoFetch={false} />);

    expect(screen.getByText('No fixed storage volumes detected.')).toBeDefined();
    expect(screen.queryByRole('table')).toBeNull();
  });

  it('renders memory capacity scale markers', () => {
    render(<SystemMonitor initialData={mockSystemData} autoFetch={false} />);

    expect(screen.getByText('0%')).toBeDefined();
    expect(screen.getByText('25%')).toBeDefined();
    expect(screen.getByText('50%')).toBeDefined();
    expect(screen.getByText('75%')).toBeDefined();
    expect(screen.getByText('100%')).toBeDefined();
  });

  it('renders dual uptime distinguishing system uptime from application uptime', () => {
    render(<SystemMonitor initialData={mockSystemData} autoFetch={false} />);

    // System uptime: 90061s -> 1d 1h 1m
    expect(screen.getByText('System OS Uptime')).toBeDefined();
    expect(screen.getByText('1d 1h 1m')).toBeDefined();

    // App uptime: 125s -> 2m 5s
    expect(screen.getByText('DeskPilot Session Uptime')).toBeDefined();
    expect(screen.getByText('2m 5s')).toBeDefined();
  });
});

describe('appendCpuSample Helper & Bounded History (30 samples)', () => {
  it('appends valid CPU utilization samples and clamps between 0 and 100', () => {
    let history: number[] = [];
    history = appendCpuSample(history, 15.5);
    expect(history).toEqual([15.5]);

    history = appendCpuSample(history, 85.0);
    expect(history).toEqual([15.5, 85.0]);

    // Clamps below 0 to 0
    history = appendCpuSample(history, -5);
    // Negative values are discarded without fabricating 0
    expect(history).toEqual([15.5, 85.0]);

    // Clamps above 100 to 100
    history = appendCpuSample(history, 120);
    expect(history).toEqual([15.5, 85.0, 100]);
  });

  it('caps buffer at strictly MAX_CPU_HISTORY_SAMPLES (30) when exceeding limit', () => {
    let history: number[] = [];
    for (let i = 1; i <= 35; i++) {
      history = appendCpuSample(history, i);
    }

    expect(history).toHaveLength(MAX_CPU_HISTORY_SAMPLES);
    expect(history[0]).toBe(6); // 1..5 shifted out
    expect(history[history.length - 1]).toBe(35);
  });

  it('discards NaN, null, undefined, and non-numerical values without adding zero', () => {
    const initial = [10, 20, 30];
    expect(appendCpuSample(initial, NaN)).toEqual(initial);
    expect(appendCpuSample(initial, null)).toEqual(initial);
    expect(appendCpuSample(initial, undefined)).toEqual(initial);
    expect(appendCpuSample(initial, Infinity)).toEqual(initial);
  });
});

describe('CpuSparkline Component', () => {
  it('renders SVG vector graphic with accessible label and caption', () => {
    render(<CpuSparkline samples={[10, 20, 30, 40, 50]} />);

    const sparkline = screen.getByRole('img', {
      name: /CPU utilization trend across 5 samples, current 50.0%/i,
    });
    expect(sparkline).toBeDefined();
    expect(screen.getByText('5 samples (max 30)')).toBeDefined();
  });

  it('handles empty or single sample gracefully without broken SVG paths', () => {
    const { container: emptyContainer } = render(<CpuSparkline samples={[]} />);
    expect(emptyContainer.querySelector('path.cpu-sparkline-path')).toBeNull();
    expect(screen.getByText('0 samples (max 30)')).toBeDefined();

    cleanup();

    const { container: singleContainer } = render(<CpuSparkline samples={[45]} />);
    expect(singleContainer.querySelector('path.cpu-sparkline-path')).toBeNull();
    expect(screen.getByText('1 sample (max 30)')).toBeDefined();
  });

  it('renders path and area when multiple valid samples are provided', () => {
    const { container } = render(<CpuSparkline samples={[20, 40, 60, 80]} />);

    const path = container.querySelector('path.cpu-sparkline-path');
    const area = container.querySelector('path.cpu-sparkline-area');
    const dot = container.querySelector('circle.cpu-sparkline-dot');

    expect(path).not.toBeNull();
    expect(area).not.toBeNull();
    expect(dot).not.toBeNull();
  });
});

describe('Shared Polling Architecture & Lifecycle', () => {
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it('shares single polling state across multiple consumer components under SystemMetricsProvider', async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValue(mockSystemData);

    function ConsumerA() {
      const state = useSharedSystemMetrics();
      return <div data-testid="consumer-a">CPU A: {state?.metrics?.cpu.utilizationPercent ?? 'none'}</div>;
    }

    function ConsumerB() {
      const state = useSharedSystemMetrics();
      return <div data-testid="consumer-b">CPU B: {state?.metrics?.cpu.utilizationPercent ?? 'none'}</div>;
    }

    render(
      <SystemMetricsProvider initialData={mockSystemData} autoFetch={false}>
        <ConsumerA />
        <ConsumerB />
      </SystemMetricsProvider>
    );

    expect(screen.getByTestId('consumer-a').textContent).toBe('CPU A: 24.5');
    expect(screen.getByTestId('consumer-b').textContent).toBe('CPU B: 24.5');

    // Both consumed same state without issuing any invoke
    expect(tauriCore.invoke).not.toHaveBeenCalledWith('get_system_metrics', expect.anything());
  });

  it('cleans up interval timer and event listeners on unmount without throwing errors', () => {
    vi.useFakeTimers();
    vi.mocked(tauriCore.invoke).mockResolvedValue(mockSystemData);

    const { unmount } = render(
      <SystemMetricsProvider defaultRefreshMode="3s" autoFetch={false}>
        <div data-testid="child">Child Content</div>
      </SystemMetricsProvider>
    );

    // Unmount must clean up timer safely
    expect(() => unmount()).not.toThrow();
  });

  it('does not invoke list_processes during system metrics polling cycle', async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValue(mockSystemData);

    render(<SystemMonitor autoFetch={true} />);

    await waitFor(() => {
      expect(screen.getByText('24.5%')).toBeDefined();
    });

    // Confirms strict isolation: list_processes was never queried
    expect(tauriCore.invoke).not.toHaveBeenCalledWith('list_processes', expect.anything());
    expect(tauriCore.invoke).toHaveBeenCalledWith('get_system_metrics', undefined);
  });

  it('does not trigger redundant local initial IPC request when shared provider is active', async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValue(mockSystemData);

    render(
      <SystemMetricsProvider autoFetch={true} defaultRefreshMode="manual">
        <SystemMonitor />
      </SystemMetricsProvider>
    );

    await waitFor(() => {
      expect(screen.getByText('24.5%')).toBeDefined();
    });

    // Exactly one call issued by the shared provider; local fallback hook remained completely inert
    expect(tauriCore.invoke).toHaveBeenCalledTimes(1);
    expect(tauriCore.invoke).toHaveBeenCalledWith('get_system_metrics', undefined);
  });

  it('does not trigger automatic request when autoFetch is disabled in standalone mode', () => {
    render(<SystemMonitor autoFetch={false} />);
    expect(tauriCore.invoke).not.toHaveBeenCalled();
  });
});
