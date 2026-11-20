// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import ReactDOMServer from 'react-dom/server';
import { render, screen, fireEvent, act, cleanup } from '@testing-library/react';
import {
  LaunchButton,
  executeLaunch,
  DEFAULT_LAUNCH_RESET_DELAY_MS,
} from '../LaunchButton';
import * as tauriCore from '@tauri-apps/api/core';
import { IpcError } from '../../lib/ipc';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('LaunchButton Component & Execution Layer', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  describe('executeLaunch Core Logic', () => {
    it('executes launch and passes strictly applicationId', async () => {
      const mockResult = {
        applicationId: 'app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d',
        processId: 5432,
      };
      vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockResult);

      const onSuccess = vi.fn();
      const onStart = vi.fn();

      const result = await executeLaunch('app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d', {
        onStart,
        onSuccess,
      });

      expect(onStart).toHaveBeenCalledTimes(1);
      expect(onSuccess).toHaveBeenCalledWith(5432);
      expect(result).toEqual(mockResult);

      expect(tauriCore.invoke).toHaveBeenCalledTimes(1);
      expect(tauriCore.invoke).toHaveBeenCalledWith('launch_application', {
        applicationId: 'app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d',
      });

      // Strict security assertion: verify payload contains only applicationId
      const callArgs = vi.mocked(tauriCore.invoke).mock.calls[0][1] as Record<string, unknown>;
      expect(Object.keys(callArgs)).toEqual(['applicationId']);
      expect(callArgs.path).toBeUndefined();
      expect(callArgs.executablePath).toBeUndefined();
      expect(callArgs.args).toBeUndefined();
      expect(callArgs.cwd).toBeUndefined();
      expect(callArgs.env).toBeUndefined();
      expect(callArgs.shell).toBeUndefined();
    });

    it('prevents duplicate launch when isBusy indicates an operation is in-flight', async () => {
      const onStart = vi.fn();
      const onSuccess = vi.fn();

      const result = await executeLaunch('app_test123', {
        isBusy: () => true, // simulates pending operation
        onStart,
        onSuccess,
      });

      expect(result).toBeNull();
      expect(onStart).not.toHaveBeenCalled();
      expect(onSuccess).not.toHaveBeenCalled();
      expect(tauriCore.invoke).not.toHaveBeenCalled();
    });

    it('propagates typed IpcError on backend launch failure and invokes onError', async () => {
      vi.mocked(tauriCore.invoke).mockRejectedValueOnce({
        code: 'PERMISSION_DENIED',
        message: 'Executable resides outside allowed directory boundaries',
      });

      const onError = vi.fn();
      const onSuccess = vi.fn();

      await expect(
        executeLaunch('app_test123', {
          onError,
          onSuccess,
        })
      ).rejects.toThrow(IpcError);

      expect(onSuccess).not.toHaveBeenCalled();
      expect(onError).toHaveBeenCalledTimes(1);
      const passedError = onError.mock.calls[0][0] as IpcError;
      expect(passedError.code).toBe('PERMISSION_DENIED');
      expect(passedError.message).toBe('Executable resides outside allowed directory boundaries');
    });
  });

  describe('LaunchButton Rendering & Accessibility', () => {
    it('renders idle state by default with accessible label', () => {
      const html = ReactDOMServer.renderToString(
        <LaunchButton applicationId="app_11112222333344445555666677778888" />
      );

      expect(html).toContain('type="button"');
      expect(html).toContain('>Open<');
      expect(html).toContain('class="launch-button idle"');
      expect(html).toContain('aria-label="Open application app_11112222333344445555666677778888"');
      expect(html).not.toContain('disabled=""');
      expect(html).not.toContain('disabled="true"');
      expect(html).not.toContain('aria-busy="true"');
    });

    it('renders launching state when status="launching" with aria-busy and disabled', () => {
      const html = ReactDOMServer.renderToString(
        <LaunchButton
          applicationId="app_test"
          status="launching"
        />
      );

      expect(html).toContain('Opening…');
      expect(html).toContain('disabled=""');
      expect(html).toContain('aria-busy="true"');
      expect(html).toContain('launching');
    });

    it('renders launched state when status="launched"', () => {
      const html = ReactDOMServer.renderToString(
        <LaunchButton
          applicationId="app_test"
          status="launched"
        />
      );

      expect(html).toContain('>Opened<');
      expect(html).toContain('class="launch-button launched"');
    });

    it('renders failed state when status="failed"', () => {
      const html = ReactDOMServer.renderToString(
        <LaunchButton
          applicationId="app_test"
          status="failed"
        />
      );

      expect(html).toContain('Couldn&#x27;t open');
      expect(html).toContain('class="launch-button failed"');
    });

    it('renders disabled attribute when disabled prop is true', () => {
      const html = ReactDOMServer.renderToString(
        <LaunchButton
          applicationId="app_test"
          disabled={true}
        />
      );

      expect(html).toContain('disabled=""');
    });
  });

  describe('LaunchButton Timer & State Recovery Lifecycle', () => {
    it('automatically resets from launched state back to idle after DEFAULT_LAUNCH_RESET_DELAY_MS', async () => {
      vi.useFakeTimers();
      vi.mocked(tauriCore.invoke).mockResolvedValueOnce({
        applicationId: 'app_timer_test',
        processId: 7890,
      });

      render(<LaunchButton applicationId="app_timer_test" />);

      const button = screen.getByRole('button', { name: /open application app_timer_test/i });
      expect(button.textContent).toBe('Open');

      // Click to launch
      await act(async () => {
        fireEvent.click(button);
      });

      // Status becomes launched
      expect(button.textContent).toBe('Opened (PID 7890)');
      expect(button.classList.contains('launched')).toBe(true);

      // Fast-forward 2999ms (not yet reset)
      act(() => {
        vi.advanceTimersByTime(2999);
      });
      expect(button.textContent).toBe('Opened (PID 7890)');

      // Fast-forward past DEFAULT_LAUNCH_RESET_DELAY_MS (3000ms)
      act(() => {
        vi.advanceTimersByTime(1);
      });

      // State is recovered back to idle, allowing repeated launches
      expect(button.textContent).toBe('Open');
      expect(button.classList.contains('idle')).toBe(true);
    });

    it('automatically resets from failed state back to idle after configured delay', async () => {
      vi.useFakeTimers();
      vi.mocked(tauriCore.invoke).mockRejectedValueOnce({
        code: 'UNAVAILABLE',
        message: 'Executable target not found on disk',
      });

      render(<LaunchButton applicationId="app_timer_fail" autoResetDelayMs={1500} />);

      const button = screen.getByRole('button', { name: /open application app_timer_fail/i });

      // Click to launch (fails)
      await act(async () => {
        fireEvent.click(button);
      });

      expect(button.textContent).toBe("Couldn't open");
      expect(button.classList.contains('failed')).toBe(true);

      // Advance by 1500ms
      act(() => {
        vi.advanceTimersByTime(1500);
      });

      // State recovered to idle
      expect(button.textContent).toBe('Open');
      expect(button.classList.contains('idle')).toBe(true);
    });

    it('cleans up pending timer on component unmount without error', async () => {
      vi.useFakeTimers();
      vi.mocked(tauriCore.invoke).mockResolvedValueOnce({
        applicationId: 'app_unmount',
        processId: 1111,
      });

      const { unmount } = render(<LaunchButton applicationId="app_unmount" />);
      const button = screen.getByRole('button', { name: /open application app_unmount/i });

      await act(async () => {
        fireEvent.click(button);
      });

      expect(button.textContent).toBe('Opened (PID 1111)');

      // Unmount before timer fires
      unmount();

      // Advancing timer after unmount should not trigger any exceptions or warnings
      expect(() => {
        act(() => {
          vi.advanceTimersByTime(DEFAULT_LAUNCH_RESET_DELAY_MS);
        });
      }).not.toThrow();
    });

    it('cancels pending reset timer when user initiates a new launch attempt', async () => {
      vi.useFakeTimers();
      vi.mocked(tauriCore.invoke).mockResolvedValue({
        applicationId: 'app_multilaunch',
        processId: 2222,
      });

      render(<LaunchButton applicationId="app_multilaunch" autoResetDelayMs={3000} />);
      const button = screen.getByRole('button', { name: /open application app_multilaunch/i });

      // First launch
      await act(async () => {
        fireEvent.click(button);
      });
      expect(button.textContent).toBe('Opened (PID 2222)');

      // 1000ms passes (timer is pending)
      act(() => {
        vi.advanceTimersByTime(1000);
      });

      // User launches again before the 3000ms timer completes
      vi.mocked(tauriCore.invoke).mockResolvedValueOnce({
        applicationId: 'app_multilaunch',
        processId: 3333,
      });
      await act(async () => {
        fireEvent.click(button);
      });

      expect(button.textContent).toBe('Opened (PID 3333)');

      // Another 2000ms passes (total 3000ms since first launch, but only 2000ms since second launch)
      act(() => {
        vi.advanceTimersByTime(2000);
      });
      // Stale timer from first launch MUST NOT reset the second launch early!
      expect(button.textContent).toBe('Opened (PID 3333)');

      // Another 1000ms passes (3000ms since second launch)
      act(() => {
        vi.advanceTimersByTime(1000);
      });
      // Now resets to idle
      expect(button.textContent).toBe('Open');
    });
  });
});
