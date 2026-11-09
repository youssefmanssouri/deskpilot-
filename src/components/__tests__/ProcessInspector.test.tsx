// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/react';
import { ProcessInspector, getStatusExplanation } from '../ProcessInspector';
import type { ProcessDto } from '../../lib/ipc';

describe('ProcessInspector Component & Security Invariants', () => {
  const sampleProcess: ProcessDto = {
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
  };

  const restrictedProcess: ProcessDto = {
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
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  describe('Empty State Presentation', () => {
    it('renders empty selection state with clear guidance when process is null', () => {
      render(<ProcessInspector process={null} />);

      expect(screen.getByTestId('process-inspector-empty')).toBeDefined();
      expect(screen.getByText('Process Inspector')).toBeDefined();
      expect(screen.getByText('No Process Selected')).toBeDefined();
      expect(
        screen.getByText(/Select a process from the ledger to inspect detailed telemetry/i)
      ).toBeDefined();
      expect(screen.queryByTestId('process-inspector')).toBeNull();
    });
  });

  describe('Populated State Telemetry Presentation', () => {
    it('renders detailed process identification, telemetry, and status badges', () => {
      render(<ProcessInspector process={sampleProcess} />);

      expect(screen.getByTestId('process-inspector')).toBeDefined();
      expect(screen.getByText('code.exe')).toBeDefined();
      expect(screen.getByText('PID:')).toBeDefined();
      expect(screen.getByText('1000')).toBeDefined();
      expect(screen.getByText('PPID:')).toBeDefined();
      expect(screen.getByText('500')).toBeDefined();
      expect(screen.getByText('Session:')).toBeDefined();
      expect(screen.getByText('1')).toBeDefined();

      // Memory
      expect(screen.getByTestId('inspector-memory-formatted').textContent).toBe('200.0 MB');
      expect(screen.getByTestId('inspector-memory-bytes').textContent).toBe('209,715,200 bytes');

      // Cumulative CPU Time (Must not be CPU %)
      expect(screen.getByText('Cumulative CPU Time')).toBeDefined();
      expect(screen.getByTestId('inspector-cpu-formatted').textContent).toBe('15.0 s');
      expect(screen.getByTestId('inspector-cpu-ms').textContent).toBe('15,000 ms');

      // Timestamps & Path
      expect(screen.getByTestId('inspector-created-at').textContent).toBe('2026-11-03 10:00:00 UTC');
      expect(screen.getByTestId('inspector-executable-path').textContent).toBe(
        'C:\\Programs\\VSCode\\Code.exe'
      );
    });

    it('renders accurate status explanation for each query status', () => {
      expect(getStatusExplanation('full')).toContain('PROCESS_QUERY_LIMITED_INFORMATION');
      expect(getStatusExplanation('partial')).toContain('restricted rights');
      expect(getStatusExplanation('handleUnavailable')).toMatch(/operating system denied/i);
      expect(getStatusExplanation('exited')).toContain('terminated');
    });
  });

  describe('Null Value Fallbacks', () => {
    it('renders graceful fallbacks when optional process fields are null or restricted', () => {
      render(<ProcessInspector process={restrictedProcess} />);

      expect(screen.getByText('System Idle Process')).toBeDefined();
      expect(screen.getByText('PID:')).toBeDefined();
      expect(screen.getAllByText('0').length).toBeGreaterThanOrEqual(1);

      // Session pill should not render when null
      expect(screen.queryByText('Session:')).toBeNull();

      // Memory fallbacks
      expect(screen.getByTestId('inspector-memory-formatted').textContent).toBe('—');
      expect(screen.getByTestId('inspector-memory-bytes').textContent).toBe('Not reported by OS');

      // CPU duration fallbacks
      expect(screen.getByTestId('inspector-cpu-formatted').textContent).toBe('—');
      expect(screen.getByTestId('inspector-cpu-ms').textContent).toBe('Not reported by OS');

      // Created at fallback
      expect(screen.getByTestId('inspector-created-at').textContent).toBe('—');

      // Path fallback
      expect(screen.getByTestId('inspector-executable-path-empty')).toBeDefined();
      expect(screen.queryByTestId('copy-path-button')).toBeNull();
    });
  });

  describe('Clipboard Copy Path Interaction', () => {
    it('copies executable path when clipboard writeText is supported', async () => {
      const writeTextMock = vi.fn().mockResolvedValue(undefined);
      Object.assign(navigator, {
        clipboard: {
          writeText: writeTextMock,
        },
      });

      render(<ProcessInspector process={sampleProcess} />);

      const copyBtn = screen.getByTestId('copy-path-button');
      expect(copyBtn.textContent).toBe('Copy Path');

      fireEvent.click(copyBtn);

      expect(writeTextMock).toHaveBeenCalledWith('C:\\Programs\\VSCode\\Code.exe');

      await waitFor(() => {
        expect(copyBtn.textContent).toBe('✓ Copied');
      });
    });
  });

  describe('Strict Read-Only Verification', () => {
    it('renders read-only disclosure and contains zero process termination controls', () => {
      const { container } = render(<ProcessInspector process={sampleProcess} />);

      // Verify read-only security disclosure badge
      expect(screen.getByText(/Read-Only Telemetry • Diagnostic Mode/i)).toBeDefined();
      expect(screen.getByText(/termination, suspension, and injection are disabled/i)).toBeDefined();

      // Verify no destructive action buttons exist
      const buttons = container.querySelectorAll('button');
      for (const btn of buttons) {
        const text = btn.textContent?.toLowerCase() || '';
        const ariaLabel = btn.getAttribute('aria-label')?.toLowerCase() || '';
        expect(text).not.toMatch(/kill|terminate|end task|suspend|priority/i);
        expect(ariaLabel).not.toMatch(/kill|terminate|end task|suspend|priority/i);
      }
    });

    it('triggers onClose callback when close button is clicked', () => {
      const handleClose = vi.fn();
      render(<ProcessInspector process={sampleProcess} onClose={handleClose} />);

      const closeBtn = screen.getByRole('button', { name: /close process inspector/i });
      fireEvent.click(closeBtn);

      expect(handleClose).toHaveBeenCalledTimes(1);
    });
  });
});
