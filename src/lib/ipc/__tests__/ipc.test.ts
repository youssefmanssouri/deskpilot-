import { describe, it, expect, vi, beforeEach } from 'vitest';
import { IpcError } from '../errors';
import { deskpilotApi } from '../api';
import * as tauriCore from '@tauri-apps/api/core';
import type { AppStatusDto } from '../types';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('IPC Error Architecture', () => {
  it('correctly categorizes all error codes', () => {
    const invalidInput = new IpcError({ code: 'INVALID_INPUT', message: 'Bad ID' });
    expect(invalidInput.isInvalidInput()).toBe(true);
    expect(invalidInput.isNotFound()).toBe(false);

    const notFound = new IpcError({ code: 'NOT_FOUND', message: 'Missing' });
    expect(notFound.isNotFound()).toBe(true);

    const permDenied = new IpcError({ code: 'PERMISSION_DENIED', message: 'Blocked' });
    expect(permDenied.isPermissionDenied()).toBe(true);

    const unavailable = new IpcError({ code: 'UNAVAILABLE', message: 'Busy' });
    expect(unavailable.isUnavailable()).toBe(true);

    const opFailed = new IpcError({ code: 'OPERATION_FAILED', message: 'Execution failed' });
    expect(opFailed.isOperationFailed()).toBe(true);

    const internal = new IpcError({ code: 'INTERNAL_ERROR', message: 'Crash' });
    expect(internal.isInternalError()).toBe(true);
  });

  it('safely converts arbitrary error responses into IpcError', () => {
    // 1. Structured DTO
    const errFromDto = IpcError.fromResponse({
      code: 'PERMISSION_DENIED',
      message: 'Access denied to sensitive system area',
    });
    expect(errFromDto).toBeInstanceOf(IpcError);
    expect(errFromDto.code).toBe('PERMISSION_DENIED');
    expect(errFromDto.message).toBe('Access denied to sensitive system area');

    // 2. String error
    const errFromString = IpcError.fromResponse('Raw backend failure');
    expect(errFromString.code).toBe('INTERNAL_ERROR');
    expect(errFromString.message).toBe('Raw backend failure');

    // 3. Fallback unknown error
    const errFromNull = IpcError.fromResponse(null);
    expect(errFromNull.code).toBe('INTERNAL_ERROR');
    expect(errFromNull.message).toBe('An unknown IPC communication error occurred.');
  });
});

describe('Typed Frontend IPC API Wrapper', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('getAppStatus invokes get_app_status and returns typed AppStatusDto', async () => {
    const mockStatus: AppStatusDto = {
      appName: 'DeskPilot',
      version: '0.1.0',
      status: 'ready',
      uptimeSeconds: 120,
      environment: 'desktop',
    };

    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockStatus);

    const result = await deskpilotApi.getAppStatus();

    expect(tauriCore.invoke).toHaveBeenCalledTimes(1);
    expect(tauriCore.invoke).toHaveBeenCalledWith('get_app_status', undefined);
    expect(result).toEqual(mockStatus);
    expect(result.appName).toBe('DeskPilot');
  });

  it('validateCommandIntent sends structured intent and handles rejection', async () => {
    vi.mocked(tauriCore.invoke).mockRejectedValueOnce({
      code: 'PERMISSION_DENIED',
      message: "Action 'rm_rf' is not permitted by current security boundary",
    });

    const intent = {
      intentId: 'cmd-1',
      actionType: 'rm_rf',
    };

    await expect(deskpilotApi.validateCommandIntent(intent)).rejects.toThrow(IpcError);

    try {
      await deskpilotApi.validateCommandIntent(intent);
    } catch (e) {
      const ipcErr = e as IpcError;
      expect(ipcErr.code).toBe('PERMISSION_DENIED');
      expect(ipcErr.isPermissionDenied()).toBe(true);
    }
  });
});

