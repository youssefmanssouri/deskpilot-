import { describe, it, expect, vi, beforeEach } from 'vitest';
import ReactDOMServer from 'react-dom/server';
import {
  LaunchButton,
  executeLaunch,
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
      expect(html).toContain('Launch');
      expect(html).toContain('class="launch-button idle"');
      expect(html).toContain('aria-label="Launch application app_11112222333344445555666677778888"');
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

      expect(html).toContain('Launching...');
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

      expect(html).toContain('Launched');
      expect(html).toContain('class="launch-button launched"');
    });

    it('renders failed state when status="failed"', () => {
      const html = ReactDOMServer.renderToString(
        <LaunchButton
          applicationId="app_test"
          status="failed"
        />
      );

      expect(html).toContain('Launch Failed');
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
});
