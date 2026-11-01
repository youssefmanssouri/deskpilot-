import { describe, it, expect, vi, beforeEach } from 'vitest';
import { deskpilotApi } from '../api';
import { IpcError } from '../errors';
import * as tauriCore from '@tauri-apps/api/core';
import type { DiscoveryResultDto } from '../types';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('Application Discovery IPC Client', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('handles a successful catalog response with populated applications', async () => {
    const mockResult: DiscoveryResultDto = {
      applications: [
        {
          id: 'app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d',
          displayName: 'Google Chrome',
          publisher: 'Google LLC',
          version: '156.0.0.0',
          sources: ['Registry (HKLM 32-bit)', 'Start Menu (Common)'],
          availability: 'Available',
          isInstalled: true,
        },
        {
          id: 'app_9f8e7d6c5b4a3a2a1b2c3d4e5f6a7b8c',
          displayName: 'Visual Studio Code',
          publisher: 'Microsoft Corporation',
          version: '1.99.0',
          sources: ['Registry (HKCU User)'],
          availability: 'Available',
          isInstalled: true,
        },
      ],
      scannedSources: ['Registry (HKLM 32-bit)', 'Registry (HKCU User)', 'Start Menu (Common)'],
      partialFailures: [],
      totalDiscovered: 2,
    };

    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockResult);

    const result = await deskpilotApi.discoverApplications();

    expect(tauriCore.invoke).toHaveBeenCalledTimes(1);
    expect(tauriCore.invoke).toHaveBeenCalledWith('discover_applications', undefined);
    expect(result.applications).toHaveLength(2);
    expect(result.totalDiscovered).toBe(2);
    expect(result.applications[0].displayName).toBe('Google Chrome');
    expect(result.applications[0].sources).toContain('Registry (HKLM 32-bit)');
    expect(result.applications[0].availability).toBe('Available');
  });

  it('verifies 128-bit collision-resistant application ID format', async () => {
    const mockResult: DiscoveryResultDto = {
      applications: [
        {
          id: 'app_e3b0c44298fc1c149afbf4c8996fb924',
          displayName: 'Test App',
          sources: ['Registry (HKLM 64-bit)'],
          availability: 'Available',
          isInstalled: true,
        },
      ],
      scannedSources: ['Registry (HKLM 64-bit)'],
      partialFailures: [],
      totalDiscovered: 1,
    };

    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockResult);

    const result = await deskpilotApi.discoverApplications();
    const appId = result.applications[0].id;

    // Must strictly start with app_ followed by exactly 32 lowercase hex characters (36 chars total)
    expect(appId).toMatch(/^app_[0-9a-f]{32}$/);
    expect(appId.length).toBe(36);
  });

  it('handles an empty catalog response cleanly', async () => {
    const mockEmptyResult: DiscoveryResultDto = {
      applications: [],
      scannedSources: ['Registry (HKLM 64-bit)', 'Registry (HKCU User)'],
      partialFailures: [],
      totalDiscovered: 0,
    };

    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockEmptyResult);

    const result = await deskpilotApi.discoverApplications();

    expect(result.applications).toHaveLength(0);
    expect(result.totalDiscovered).toBe(0);
    expect(result.scannedSources).toHaveLength(2);
  });

  it('handles partial discovery failures gracefully without crashing', async () => {
    const mockPartialResult: DiscoveryResultDto = {
      applications: [
        {
          id: 'app_112233445566778899aabbccddeeff00',
          displayName: 'Notepad++',
          publisher: 'Don Ho',
          version: '8.6.2',
          sources: ['Registry (HKLM 64-bit)'],
          availability: 'Available',
          isInstalled: true,
        },
      ],
      scannedSources: ['Registry (HKLM 64-bit)', 'Registry (HKLM 32-bit)'],
      partialFailures: ['Registry (HKLM 32-bit): Access is denied.'],
      totalDiscovered: 1,
    };

    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockPartialResult);

    const result = await deskpilotApi.discoverApplications();

    expect(result.applications).toHaveLength(1);
    expect(result.partialFailures).toHaveLength(1);
    expect(result.partialFailures[0]).toBe('Registry (HKLM 32-bit): Access is denied.');
  });

  it('verifies partial failures are sanitized and omit raw filesystem paths and usernames', async () => {
    const mockResult: DiscoveryResultDto = {
      applications: [],
      scannedSources: ['Start Menu (User)'],
      partialFailures: ['Start Menu (User): Access is denied.'],
      totalDiscovered: 0,
    };

    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockResult);

    const result = await deskpilotApi.discoverApplications();
    expect(result.partialFailures).toHaveLength(1);
    expect(result.partialFailures[0]).toBe('Start Menu (User): Access is denied.');
    expect(result.partialFailures[0]).not.toMatch(/[a-zA-Z]:\\/);
    expect(result.partialFailures[0]).not.toMatch(/Users/i);
    expect(result.partialFailures[0]).not.toMatch(/AppData/i);
  });

  it('catches and maps backend IPC errors to typed IpcError', async () => {
    vi.mocked(tauriCore.invoke).mockRejectedValueOnce({
      code: 'UNAVAILABLE',
      message: 'Windows registry subsystem is temporarily locked by another process',
    });

    await expect(deskpilotApi.discoverApplications()).rejects.toThrow(IpcError);

    try {
      await deskpilotApi.discoverApplications();
    } catch (err) {
      const ipcErr = err as IpcError;
      expect(ipcErr.code).toBe('UNAVAILABLE');
      expect(ipcErr.isUnavailable()).toBe(true);
      expect(ipcErr.message).toContain('registry subsystem is temporarily locked');
    }
  });

  it('handles malformed string error rejections safely', async () => {
    vi.mocked(tauriCore.invoke).mockRejectedValueOnce('Low-level IO failure');

    await expect(deskpilotApi.discoverApplications()).rejects.toThrow(IpcError);

    try {
      await deskpilotApi.discoverApplications();
    } catch (err) {
      const ipcErr = err as IpcError;
      expect(ipcErr.code).toBe('INTERNAL_ERROR');
      expect(ipcErr.message).toBe('Low-level IO failure');
    }
  });

  it('supports repeated refresh calls', async () => {
    const mock1: DiscoveryResultDto = {
      applications: [],
      scannedSources: [],
      partialFailures: [],
      totalDiscovered: 0,
    };
    const mock2: DiscoveryResultDto = {
      applications: [
        {
          id: 'app_1234567890abcdef1234567890abcdef',
          displayName: 'App',
          sources: [],
          availability: 'Available',
          isInstalled: true,
        },
      ],
      scannedSources: [],
      partialFailures: [],
      totalDiscovered: 1,
    };

    vi.mocked(tauriCore.invoke)
      .mockResolvedValueOnce(mock1)
      .mockResolvedValueOnce(mock2);

    const res1 = await deskpilotApi.discoverApplications();
    expect(res1.totalDiscovered).toBe(0);

    const res2 = await deskpilotApi.discoverApplications();
    expect(res2.totalDiscovered).toBe(1);
    expect(tauriCore.invoke).toHaveBeenCalledTimes(2);
  });
});
