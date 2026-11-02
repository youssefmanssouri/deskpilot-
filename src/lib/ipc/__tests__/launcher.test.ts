import { describe, it, expect, vi, beforeEach } from 'vitest';
import { deskpilotApi, launchApplication } from '../api';
import { IpcError } from '../errors';
import * as tauriCore from '@tauri-apps/api/core';
import type { LaunchResultDto } from '../types';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('Application Launcher IPC Client', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('launchApplication sends the correct command name and passes only applicationId', async () => {
    const mockResult: LaunchResultDto = {
      applicationId: 'app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d',
      processId: 4321,
    };

    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockResult);

    const result = await deskpilotApi.launchApplication('app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d');

    expect(tauriCore.invoke).toHaveBeenCalledTimes(1);
    expect(tauriCore.invoke).toHaveBeenCalledWith('launch_application', {
      applicationId: 'app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d',
    });

    // Statically and dynamically verify only applicationId was passed
    const callArgs = vi.mocked(tauriCore.invoke).mock.calls[0][1] as Record<string, unknown>;
    expect(callArgs).toBeDefined();
    expect(Object.keys(callArgs)).toEqual(['applicationId']);
    expect(callArgs.applicationId).toBe('app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d');

    // Verify forbidden fields are absent
    expect(callArgs).not.toHaveProperty('path');
    expect(callArgs).not.toHaveProperty('executablePath');
    expect(callArgs).not.toHaveProperty('executable');
    expect(callArgs).not.toHaveProperty('args');
    expect(callArgs).not.toHaveProperty('cwd');
    expect(callArgs).not.toHaveProperty('env');
    expect(callArgs).not.toHaveProperty('shell');
    expect(callArgs).not.toHaveProperty('command');

    // Verify result mapping
    expect(result.applicationId).toBe('app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d');
    expect(result.processId).toBe(4321);
  });

  it('standalone launchApplication function delegates correctly to deskpilotApi', async () => {
    const mockResult: LaunchResultDto = {
      applicationId: 'app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d',
      processId: 8888,
    };

    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockResult);

    const result = await launchApplication('app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d');
    expect(tauriCore.invoke).toHaveBeenCalledWith('launch_application', {
      applicationId: 'app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d',
    });
    expect(result.processId).toBe(8888);
  });

  it('launchApplication propagates structured IpcError on PERMISSION_DENIED', async () => {
    vi.mocked(tauriCore.invoke).mockRejectedValueOnce({
      code: 'PERMISSION_DENIED',
      message: 'Executable resides outside allowed directory boundaries',
    });

    await expect(
      deskpilotApi.launchApplication('app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d')
    ).rejects.toThrow(IpcError);

    try {
      await deskpilotApi.launchApplication('app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d');
    } catch (err) {
      const ipcErr = err as IpcError;
      expect(ipcErr.code).toBe('PERMISSION_DENIED');
      expect(ipcErr.isPermissionDenied()).toBe(true);
      expect(ipcErr.message).toBe('Executable resides outside allowed directory boundaries');
    }
  });

  it('launchApplication propagates NOT_FOUND when application is missing from catalog', async () => {
    vi.mocked(tauriCore.invoke).mockRejectedValueOnce({
      code: 'NOT_FOUND',
      message: 'Application not found in catalog',
    });

    try {
      await deskpilotApi.launchApplication('app_00000000000000000000000000000000');
      expect.fail('Should have thrown');
    } catch (err) {
      const ipcErr = err as IpcError;
      expect(ipcErr.code).toBe('NOT_FOUND');
      expect(ipcErr.isNotFound()).toBe(true);
    }
  });

  it('launchApplication propagates INVALID_INPUT when identifier is malformed', async () => {
    vi.mocked(tauriCore.invoke).mockRejectedValueOnce({
      code: 'INVALID_INPUT',
      message: 'Invalid application identifier format',
    });

    try {
      await deskpilotApi.launchApplication('bad_id');
      expect.fail('Should have thrown');
    } catch (err) {
      const ipcErr = err as IpcError;
      expect(ipcErr.code).toBe('INVALID_INPUT');
      expect(ipcErr.isInvalidInput()).toBe(true);
    }
  });

  it('verifies LaunchResult TypeScript shape contains only applicationId and processId', async () => {
    const mockResult: LaunchResultDto = {
      applicationId: 'app_test1234567890abcdef1234567890',
      processId: 9876,
    };
    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockResult);

    const result = await deskpilotApi.launchApplication('app_test1234567890abcdef1234567890');
    const keys = Object.keys(result);
    expect(keys).toContain('applicationId');
    expect(keys).toContain('processId');
    expect(keys).not.toContain('path');
    expect(keys).not.toContain('executable');
    expect(keys).not.toContain('cwd');
    expect(keys).not.toContain('args');
    expect(keys).not.toContain('env');
    expect(keys).not.toContain('shell');
  });
});
