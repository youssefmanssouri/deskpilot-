import { describe, it, expect, vi, beforeEach } from 'vitest';
import * as tauriCore from '@tauri-apps/api/core';
import { deskpilotApi } from '../api';
import { IpcError } from '../errors';
import type { LaunchHistoryEntryDto } from '../types';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('launch history IPC', () => {
  beforeEach(() => {
    vi.mocked(tauriCore.invoke).mockReset();
  });

  it('getLaunchHistory invokes get_launch_history with no arguments', async () => {
    const entries: LaunchHistoryEntryDto[] = [
      { applicationId: 'app_' + 'a'.repeat(32), displayName: 'Code', launchedAtMs: 1700000000000 },
    ];
    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(entries);

    await expect(deskpilotApi.getLaunchHistory()).resolves.toEqual(entries);
    expect(tauriCore.invoke).toHaveBeenCalledWith('get_launch_history', undefined);
  });

  it('clearLaunchHistory invokes clear_launch_history', async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(undefined);

    await expect(deskpilotApi.clearLaunchHistory()).resolves.toBeUndefined();
    expect(tauriCore.invoke).toHaveBeenCalledWith('clear_launch_history', undefined);
  });

  it('maps backend failures to IpcError', async () => {
    vi.mocked(tauriCore.invoke).mockRejectedValueOnce({
      code: 'OPERATION_FAILED',
      message: 'Could not save launch history.',
    });

    await expect(deskpilotApi.clearLaunchHistory()).rejects.toBeInstanceOf(IpcError);
  });
});
