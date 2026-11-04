import { describe, it, expect, vi, beforeEach } from 'vitest';
import { deskpilotApi, getSystemMetrics } from '../api';
import { IpcError } from '../errors';
import * as tauriCore from '@tauri-apps/api/core';
import type { SystemMetricsDto } from '../types';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('System Intelligence IPC Client', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const mockMetrics: SystemMetricsDto = {
    cpu: {
      brandName: 'Intel(R) Core(TM) i7-10750H CPU @ 2.60GHz',
      logicalCores: 12,
      architecture: 'x86_64',
      utilizationPercent: 18.5,
    },
    memory: {
      totalBytes: 17094770688,
      availableBytes: 8547385344,
      usedBytes: 8547385344,
      usedPercent: 50.0,
    },
    storage: {
      drives: [
        {
          rootPath: 'C:\\',
          volumeName: 'Windows',
          fileSystem: 'NTFS',
          totalBytes: 512000000000,
          availableBytes: 256000000000,
          usedBytes: 256000000000,
          usedPercent: 50.0,
        },
      ],
    },
    system: {
      uptimeSeconds: 86400,
      appUptimeSeconds: 120,
      osName: 'Windows',
      architecture: 'x86_64',
    },
    sampledAtMs: 1730680000000,
  };

  it('successfully invokes get_system_metrics and maps response correctly', async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockMetrics);

    const result = await deskpilotApi.getSystemMetrics();

    expect(tauriCore.invoke).toHaveBeenCalledTimes(1);
    expect(tauriCore.invoke).toHaveBeenCalledWith('get_system_metrics', undefined);
    expect(result).toEqual(mockMetrics);
    expect(result.cpu.logicalCores).toBe(12);
    expect(result.memory.usedPercent).toBe(50.0);
    expect(result.storage.drives).toHaveLength(1);
    expect(result.storage.drives[0].rootPath).toBe('C:\\');
  });

  it('standalone helper getSystemMetrics forwards to deskpilotApi', async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockMetrics);

    const result = await getSystemMetrics();
    expect(tauriCore.invoke).toHaveBeenCalledWith('get_system_metrics', undefined);
    expect(result.cpu.brandName).toBe(mockMetrics.cpu.brandName);
  });

  it('translates IPC errors from deskpilotApi into IpcError instances', async () => {
    vi.mocked(tauriCore.invoke).mockRejectedValueOnce({
      code: 'UNAVAILABLE',
      message: 'Failed to query system times via GetSystemTimes',
    });

    await expect(deskpilotApi.getSystemMetrics()).rejects.toThrow(IpcError);
  });

  it('translates IPC errors from standalone helper into IpcError instances', async () => {
    vi.mocked(tauriCore.invoke).mockRejectedValueOnce({
      code: 'UNAVAILABLE',
      message: 'Failed to query system times via GetSystemTimes',
    });

    await expect(getSystemMetrics()).rejects.toThrow(IpcError);
  });
});
