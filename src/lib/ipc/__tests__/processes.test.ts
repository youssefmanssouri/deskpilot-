import { describe, it, expect, vi, beforeEach } from 'vitest';
import { deskpilotApi, listProcesses } from '../api';
import { IpcError } from '../errors';
import * as tauriCore from '@tauri-apps/api/core';
import type { ProcessListResultDto, ProcessDto, ProcessQueryStatusDto } from '../types';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('Process Intelligence IPC Client', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const mockProcesses: ProcessDto[] = [
    {
      id: 'v1:1024:01db2d5843a0e120',
      pid: 1024,
      parentPid: 512,
      name: 'code.exe',
      executablePath: 'C:\\Users\\User\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe',
      createdAt: '2026-11-03T12:00:00.000Z',
      workingSetBytes: 157286400,
      cpuTimeMs: 4500,
      sessionId: 1,
      status: 'full',
    },
    {
      id: 'v1:2048:01db2d5850000000',
      pid: 2048,
      parentPid: 1024,
      name: 'conhost.exe',
      executablePath: 'C:\\Windows\\System32\\conhost.exe',
      createdAt: '2026-11-03T12:05:00.000Z',
      workingSetBytes: 10485760,
      cpuTimeMs: 120,
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
      parentPid: 512,
      name: 'transient.exe',
      executablePath: null,
      createdAt: null,
      workingSetBytes: null,
      cpuTimeMs: null,
      sessionId: null,
      status: 'exited',
    },
  ];

  it('handles a successful process list response with populated processes', async () => {
    const mockResult: ProcessListResultDto = {
      processes: mockProcesses,
      totalProcesses: 4,
      capturedAtMs: 1730678400000,
      isTruncated: false,
    };

    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockResult);

    const result = await deskpilotApi.listProcesses();

    expect(tauriCore.invoke).toHaveBeenCalledTimes(1);
    expect(tauriCore.invoke).toHaveBeenCalledWith('list_processes', undefined);
    expect(result.processes).toHaveLength(4);
    expect(result.totalProcesses).toBe(4);
    expect(result.capturedAtMs).toBe(1730678400000);
    expect(result.isTruncated).toBe(false);

    expect(result.processes[0].name).toBe('code.exe');
    expect(result.processes[0].pid).toBe(1024);
    expect(result.processes[0].status).toBe('full');
    expect(result.processes[0].workingSetBytes).toBe(157286400);
  });

  it('handles an empty process list response cleanly', async () => {
    const mockEmptyResult: ProcessListResultDto = {
      processes: [],
      totalProcesses: 0,
      capturedAtMs: 1730678400000,
      isTruncated: false,
    };

    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockEmptyResult);

    const result = await deskpilotApi.listProcesses();

    expect(result.processes).toHaveLength(0);
    expect(result.totalProcesses).toBe(0);
    expect(result.isTruncated).toBe(false);
  });

  it('handles truncated process snapshot result correctly', async () => {
    const mockTruncatedResult: ProcessListResultDto = {
      processes: mockProcesses.slice(0, 2),
      totalProcesses: 2450,
      capturedAtMs: 1730678400000,
      isTruncated: true,
    };

    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockTruncatedResult);

    const result = await deskpilotApi.listProcesses();

    expect(result.isTruncated).toBe(true);
    expect(result.totalProcesses).toBe(2450);
    expect(result.processes).toHaveLength(2);
  });

  it('correctly preserves null for all nullable fields across different statuses', async () => {
    const mockResult: ProcessListResultDto = {
      processes: [mockProcesses[2], mockProcesses[3]],
      totalProcesses: 2,
      capturedAtMs: 1730678400000,
      isTruncated: false,
    };

    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockResult);

    const result = await deskpilotApi.listProcesses();

    const pidZero = result.processes[0];
    expect(pidZero.executablePath).toBeNull();
    expect(pidZero.createdAt).toBeNull();
    expect(pidZero.workingSetBytes).toBeNull();
    expect(pidZero.cpuTimeMs).toBeNull();
    expect(pidZero.sessionId).toBeNull();
    expect(pidZero.status).toBe('handleUnavailable');

    const exited = result.processes[1];
    expect(exited.status).toBe('exited');
    expect(exited.executablePath).toBeNull();
  });

  it('verifies all 4 supported status enum values are valid', async () => {
    const statuses: ProcessQueryStatusDto[] = ['full', 'partial', 'handleUnavailable', 'exited'];
    const mockResult: ProcessListResultDto = {
      processes: statuses.map((status, idx) => ({
        id: `id_${idx}`,
        pid: 1000 + idx,
        parentPid: 500,
        name: `proc_${status}.exe`,
        executablePath: null,
        createdAt: null,
        workingSetBytes: null,
        cpuTimeMs: null,
        sessionId: null,
        status,
      })),
      totalProcesses: 4,
      capturedAtMs: 1730678400000,
      isTruncated: false,
    };

    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockResult);

    const result = await deskpilotApi.listProcesses();
    expect(result.processes.map((p) => p.status)).toEqual(statuses);
  });

  it('maps backend IPC error into typed IpcError instance', async () => {
    vi.mocked(tauriCore.invoke).mockRejectedValueOnce({
      code: 'OPERATION_FAILED',
      message: 'Failed to create Toolhelp32 snapshot',
    });

    try {
      await deskpilotApi.listProcesses();
      expect.unreachable('Should have thrown an IpcError');
    } catch (err) {
      expect(err).toBeInstanceOf(IpcError);
      const ipcErr = err as IpcError;
      expect(ipcErr.code).toBe('OPERATION_FAILED');
      expect(ipcErr.message).toBe('Failed to create Toolhelp32 snapshot');
      expect(ipcErr.isOperationFailed()).toBe(true);
    }
  });

  it('standalone helper invokes deskpilotApi.listProcesses directly', async () => {
    const mockResult: ProcessListResultDto = {
      processes: [],
      totalProcesses: 0,
      capturedAtMs: 1730678400000,
      isTruncated: false,
    };

    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockResult);

    const result = await listProcesses();
    expect(tauriCore.invoke).toHaveBeenCalledWith('list_processes', undefined);
    expect(result).toEqual(mockResult);
  });
});

