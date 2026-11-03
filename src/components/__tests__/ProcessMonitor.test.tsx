// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react';
import {
  ProcessMonitor,
  formatWorkingSet,
  formatCpuTime,
  formatCreatedAt,
  filterProcesses,
  sortProcesses,
  getStatusBadge,
} from '../ProcessMonitor';
import * as tauriCore from '@tauri-apps/api/core';
import type { ProcessDto, ProcessListResultDto } from '../../lib/ipc';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('ProcessMonitor Component & Pure Helpers', () => {
  const sampleProcesses: ProcessDto[] = [
    {
      id: 'v1:1000:01db2d5843a0e120',
      pid: 1000,
      parentPid: 500,
      name: 'code.exe',
      executablePath: 'C:\\Programs\\VSCode\\Code.exe',
      createdAt: '2026-11-03T10:00:00.000Z',
      workingSetBytes: 209715200, // 200 MB
      cpuTimeMs: 15000, // 15 s
      sessionId: 1,
      status: 'full',
    },
    {
      id: 'v1:2000:01db2d5850000000',
      pid: 2000,
      parentPid: 1000,
      name: 'browser.exe',
      executablePath: 'C:\\Programs\\Browser\\browser.exe',
      createdAt: '2026-11-03T11:00:00.000Z',
      workingSetBytes: 524288000, // 500 MB
      cpuTimeMs: 45000, // 45 s
      sessionId: 1,
      status: 'partial',
    },
    {
      id: 'eph:42:0',
      pid: 0,
      parentPid: 0,
      name: 'System Idle Process',
      executablePath: null,
      createdAt: null,
      workingSetBytes: null,
      cpuTimeMs: null,
      sessionId: null,
      status: 'handleUnavailable',
    },
    {
      id: 'eph:42:9999',
      pid: 9999,
      parentPid: 500,
      name: 'transient.exe',
      executablePath: null,
      createdAt: null,
      workingSetBytes: null,
      cpuTimeMs: null,
      sessionId: null,
      status: 'exited',
    },
  ];

  const sampleResult: ProcessListResultDto = {
    processes: sampleProcesses,
    totalProcesses: 4,
    capturedAtMs: 1730678400000,
    isTruncated: false,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  describe('Pure Formatting Helpers', () => {
    it('formatWorkingSet handles null, bytes, KB, MB, and GB boundaries', () => {
      expect(formatWorkingSet(null)).toBe('—');
      expect(formatWorkingSet(undefined)).toBe('—');
      expect(formatWorkingSet(500)).toBe('500 B');
      expect(formatWorkingSet(1024)).toBe('1.0 KB');
      expect(formatWorkingSet(1536)).toBe('1.5 KB');
      expect(formatWorkingSet(1048576)).toBe('1.0 MB');
      expect(formatWorkingSet(104857600)).toBe('100.0 MB');
      expect(formatWorkingSet(1073741824)).toBe('1.00 GB');
      expect(formatWorkingSet(2684354560)).toBe('2.50 GB');
    });

    it('formatCpuTime handles null, ms, seconds, and minutes boundaries', () => {
      expect(formatCpuTime(null)).toBe('—');
      expect(formatCpuTime(undefined)).toBe('—');
      expect(formatCpuTime(0)).toBe('0 ms');
      expect(formatCpuTime(450)).toBe('450 ms');
      expect(formatCpuTime(1500)).toBe('1.5 s');
      expect(formatCpuTime(59900)).toBe('59.9 s');
      expect(formatCpuTime(60000)).toBe('1m 00s');
      expect(formatCpuTime(75000)).toBe('1m 15s');
      expect(formatCpuTime(3665000)).toBe('61m 05s');
    });

    it('formatCreatedAt handles null, empty, invalid and ISO dates safely', () => {
      expect(formatCreatedAt(null)).toBe('—');
      expect(formatCreatedAt('')).toBe('—');
      expect(formatCreatedAt('   ')).toBe('—');
      expect(formatCreatedAt('invalid-date')).toBe('—');
      expect(formatCreatedAt('2026-11-03T12:00:00.000Z')).toBe('2026-11-03 12:00:00 UTC');
    });

    it('getStatusBadge maps all 4 query status variants with accessible labels and classes', () => {
      expect(getStatusBadge('full')).toEqual({
        label: 'Full',
        className: 'status-badge status-badge-full',
      });
      expect(getStatusBadge('partial')).toEqual({
        label: 'Partial',
        className: 'status-badge status-badge-partial',
      });
      expect(getStatusBadge('handleUnavailable')).toEqual({
        label: 'Handle Unavailable',
        className: 'status-badge status-badge-unavailable',
      });
      expect(getStatusBadge('exited')).toEqual({
        label: 'Exited',
        className: 'status-badge status-badge-exited',
      });
    });
  });

  describe('Pure Filter & Sort Functions', () => {
    it('filterProcesses matches by process name, PID, or executable path case-insensitively', () => {
      expect(filterProcesses(sampleProcesses, '')).toEqual(sampleProcesses);
      expect(filterProcesses(sampleProcesses, '   ')).toEqual(sampleProcesses);

      // By name
      const byName = filterProcesses(sampleProcesses, 'CODE');
      expect(byName).toHaveLength(1);
      expect(byName[0].name).toBe('code.exe');

      // By PID
      const byPid = filterProcesses(sampleProcesses, '2000');
      expect(byPid).toHaveLength(1);
      expect(byPid[0].pid).toBe(2000);

      // By path
      const byPath = filterProcesses(sampleProcesses, 'browser.exe');
      expect(byPath).toHaveLength(1);
      expect(byPath[0].name).toBe('browser.exe');

      // Non-matching
      expect(filterProcesses(sampleProcesses, 'nonexistent_proc_xyz')).toHaveLength(0);
    });

    it('sortProcesses sorts deterministically by name, pid, memory, cpu, and status in both directions', () => {
      // Name asc
      const nameAsc = sortProcesses(sampleProcesses, 'name', 'asc');
      expect(nameAsc.map((p) => p.name)).toEqual([
        'browser.exe',
        'code.exe',
        'System Idle Process',
        'transient.exe',
      ]);

      // Name desc
      const nameDesc = sortProcesses(sampleProcesses, 'name', 'desc');
      expect(nameDesc.map((p) => p.name)).toEqual([
        'transient.exe',
        'System Idle Process',
        'code.exe',
        'browser.exe',
      ]);

      // PID asc
      const pidAsc = sortProcesses(sampleProcesses, 'pid', 'asc');
      expect(pidAsc.map((p) => p.pid)).toEqual([0, 1000, 2000, 9999]);

      // PID desc
      const pidDesc = sortProcesses(sampleProcesses, 'pid', 'desc');
      expect(pidDesc.map((p) => p.pid)).toEqual([9999, 2000, 1000, 0]);

      // Memory desc (null values sort to bottom with -1)
      const memDesc = sortProcesses(sampleProcesses, 'memory', 'desc');
      expect(memDesc[0].pid).toBe(2000); // 500 MB
      expect(memDesc[1].pid).toBe(1000); // 200 MB

      // CPU desc
      const cpuDesc = sortProcesses(sampleProcesses, 'cpu', 'desc');
      expect(cpuDesc[0].pid).toBe(2000); // 45s
      expect(cpuDesc[1].pid).toBe(1000); // 15s

      // Status asc
      const statusAsc = sortProcesses(sampleProcesses, 'status', 'asc');
      expect(statusAsc.map((p) => p.status)).toEqual([
        'exited',
        'full',
        'handleUnavailable',
        'partial',
      ]);
    });
  });

  describe('Component Rendering & Interactive States', () => {
    it('renders loading state initially when autoFetch=true and no initialData', () => {
      vi.mocked(tauriCore.invoke).mockReturnValue(new Promise(() => {})); // Never resolves

      render(<ProcessMonitor autoFetch={true} />);

      expect(screen.getByText('Scanning active processes...')).toBeDefined();
    });

    it('renders process rows, headers, and metadata on successful load', async () => {
      vi.mocked(tauriCore.invoke).mockResolvedValueOnce(sampleResult);

      render(<ProcessMonitor autoFetch={true} />);

      await waitFor(() => {
        expect(screen.getByText('code.exe')).toBeDefined();
        expect(screen.getByText('browser.exe')).toBeDefined();
        expect(screen.getByText('System Idle Process')).toBeDefined();
        expect(screen.getByText('transient.exe')).toBeDefined();
      });

      // Verify table column headers
      expect(screen.getByRole('columnheader', { name: /process/i })).toBeDefined();
      expect(screen.getByRole('columnheader', { name: /pid/i })).toBeDefined();
      expect(screen.getByRole('columnheader', { name: /status/i })).toBeDefined();
      expect(screen.getByRole('columnheader', { name: /memory/i })).toBeDefined();
      expect(screen.getByRole('columnheader', { name: /cpu time/i })).toBeDefined();

      // Verify formatted metrics
      expect(screen.getByText('200.0 MB')).toBeDefined();
      expect(screen.getByText('500.0 MB')).toBeDefined();
      expect(screen.getByText('15.0 s')).toBeDefined();
      expect(screen.getByText('45.0 s')).toBeDefined();

      // Verify status badges
      expect(screen.getByText('Full')).toBeDefined();
      expect(screen.getByText('Partial')).toBeDefined();
      expect(screen.getByText('Handle Unavailable')).toBeDefined();
      expect(screen.getByText('Exited')).toBeDefined();

      // Verify summary bar metrics
      expect(screen.getByText('Retained Processes:')).toBeDefined();
      expect(screen.getByText('Total Observed:')).toBeDefined();
    });

    it('renders empty state when process list is empty', async () => {
      const emptyResult: ProcessListResultDto = {
        processes: [],
        totalProcesses: 0,
        capturedAtMs: 1730678400000,
        isTruncated: false,
      };

      vi.mocked(tauriCore.invoke).mockResolvedValueOnce(emptyResult);

      render(<ProcessMonitor autoFetch={true} />);

      await waitFor(() => {
        expect(screen.getByText('No processes discovered.')).toBeDefined();
      });
    });

    it('filters rows via search input and displays empty-filter state when no match', async () => {
      render(<ProcessMonitor initialData={sampleResult} autoFetch={false} />);

      expect(screen.getByText('code.exe')).toBeDefined();
      expect(screen.getByText('browser.exe')).toBeDefined();

      const searchInput = screen.getByRole('searchbox', { name: /filter processes/i });

      // Filter by name "code"
      fireEvent.change(searchInput, { target: { value: 'code' } });

      expect(screen.getByText('code.exe')).toBeDefined();
      expect(screen.queryByText('browser.exe')).toBeNull();

      // Filter by PID "2000"
      fireEvent.change(searchInput, { target: { value: '2000' } });
      expect(screen.getByText('browser.exe')).toBeDefined();
      expect(screen.queryByText('code.exe')).toBeNull();

      // Filter by non-existent query
      fireEvent.change(searchInput, { target: { value: 'unknown_xyz_123' } });
      expect(screen.getByText('No processes match your filter.')).toBeDefined();

      // Clear search with Escape
      fireEvent.keyDown(searchInput, { key: 'Escape' });
      expect(searchInput.getAttribute('value')).toBe('');
      expect(screen.getByText('code.exe')).toBeDefined();
      expect(screen.getByText('browser.exe')).toBeDefined();
    });

    it('sorts columns when clicking on headers', async () => {
      render(<ProcessMonitor initialData={sampleResult} autoFetch={false} />);

      const pidHeader = screen.getByRole('columnheader', { name: /pid/i });

      // Click PID header -> asc
      fireEvent.click(pidHeader);
      let rows = screen.getAllByTestId(/^process-row-/);
      expect(rows[0].getAttribute('data-testid')).toBe('process-row-eph:42:0'); // PID 0

      // Click PID header again -> desc
      fireEvent.click(pidHeader);
      rows = screen.getAllByTestId(/^process-row-/);
      expect(rows[0].getAttribute('data-testid')).toBe('process-row-eph:42:9999'); // PID 9999
    });

    it('displays truncation notice banner when isTruncated is true', () => {
      const truncatedResult: ProcessListResultDto = {
        processes: sampleProcesses,
        totalProcesses: 2450,
        capturedAtMs: 1730678400000,
        isTruncated: true,
      };

      render(<ProcessMonitor initialData={truncatedResult} autoFetch={false} />);

      expect(screen.getByRole('status')).toBeDefined();
      expect(screen.getByText(/Process list truncated at 2,000 entries/)).toBeDefined();
      expect(screen.getAllByText(/2,450/).length).toBeGreaterThanOrEqual(1);
    });

    it('handles manual refresh and disables refresh button while in-flight', async () => {
      let resolveRefresh!: (val: ProcessListResultDto) => void;
      const refreshPromise = new Promise<ProcessListResultDto>((resolve) => {
        resolveRefresh = resolve;
      });

      render(<ProcessMonitor initialData={sampleResult} autoFetch={false} />);

      const refreshBtn = screen.getByRole('button', { name: /refresh process list/i });
      expect(refreshBtn.textContent).toBe('Refresh Processes');
      expect((refreshBtn as HTMLButtonElement).disabled).toBe(false);

      vi.mocked(tauriCore.invoke).mockReturnValueOnce(refreshPromise);

      // Trigger refresh
      fireEvent.click(refreshBtn);

      expect(refreshBtn.textContent).toBe('Scanning...');
      expect((refreshBtn as HTMLButtonElement).disabled).toBe(true);

      resolveRefresh(sampleResult);

      await waitFor(() => {
        expect(refreshBtn.textContent).toBe('Refresh Processes');
        expect((refreshBtn as HTMLButtonElement).disabled).toBe(false);
      });
      expect(tauriCore.invoke).toHaveBeenCalledWith('list_processes', undefined);
    });

    it('renders error banner and handles Retry button click', async () => {
      vi.mocked(tauriCore.invoke).mockRejectedValueOnce({
        code: 'OPERATION_FAILED',
        message: 'Snapshot creation failed',
      });

      render(<ProcessMonitor autoFetch={true} />);

      await waitFor(() => {
        expect(screen.getByRole('alert')).toBeDefined();
        expect(screen.getByText(/Snapshot creation failed/)).toBeDefined();
      });

      const retryBtn = screen.getByRole('button', { name: /retry process enumeration/i });

      vi.mocked(tauriCore.invoke).mockResolvedValueOnce(sampleResult);

      fireEvent.click(retryBtn);

      await waitFor(() => {
        expect(screen.getByText('code.exe')).toBeDefined();
      });
      expect(screen.queryByRole('alert')).toBeNull();
    });

    it('stale response protection: older delayed response does not overwrite newer response', async () => {
      let resolveFirst!: (val: ProcessListResultDto) => void;
      const firstPromise = new Promise<ProcessListResultDto>((resolve) => {
        resolveFirst = resolve;
      });

      let resolveSecond!: (val: ProcessListResultDto) => void;
      const secondPromise = new Promise<ProcessListResultDto>((resolve) => {
        resolveSecond = resolve;
      });

      // First call (older request, reqId = 1 initiated by autoFetch on mount)
      vi.mocked(tauriCore.invoke).mockReturnValueOnce(firstPromise);
      const { rerender } = render(<ProcessMonitor autoFetch={true} />);

      // Second call (newer request, reqId = 2 initiated via lifecycle re-trigger)
      vi.mocked(tauriCore.invoke).mockReturnValueOnce(secondPromise);
      rerender(<ProcessMonitor autoFetch={false} />);
      rerender(<ProcessMonitor autoFetch={true} />);

      // Second call resolves first with single process
      resolveSecond({
        processes: [sampleProcesses[0]],
        totalProcesses: 1,
        capturedAtMs: 1730678401000,
        isTruncated: false,
      });

      await waitFor(() => {
        expect(screen.getByText('code.exe')).toBeDefined();
      });
      expect(screen.queryByText('browser.exe')).toBeNull();

      // Older first call resolves later with all processes
      resolveFirst(sampleResult);

      // Give event loop time to process the first resolution
      await new Promise((r) => setTimeout(r, 20));

      // State must NOT be overwritten by the older first request
      expect(screen.queryByText('browser.exe')).toBeNull();
      expect(screen.getByText('code.exe')).toBeDefined();
    });

    it('unmount during pending request does not cause state errors', async () => {
      const pendingPromise = new Promise<ProcessListResultDto>(() => {});
      vi.mocked(tauriCore.invoke).mockReturnValueOnce(pendingPromise);

      const { unmount } = render(<ProcessMonitor autoFetch={true} />);

      expect(screen.getByText('Scanning active processes...')).toBeDefined();

      // Unmounting while request is in-flight must cleanly unmount without throwing
      expect(() => unmount()).not.toThrow();
    });
  });
});
