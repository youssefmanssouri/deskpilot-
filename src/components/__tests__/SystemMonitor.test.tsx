// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react';
import {
  SystemMonitor,
  formatBytes,
  formatUptime,
  getLoadColorClass,
} from '../SystemMonitor';
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
});
