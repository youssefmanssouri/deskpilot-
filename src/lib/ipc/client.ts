import { invoke } from '@tauri-apps/api/core';
import { IpcError } from './errors';

/**
 * Type-safe wrapper around Tauri's `invoke` API.
 *
 * Catches IPC rejections, deserializes them into typed `IpcError` instances,
 * and ensures components do not leak raw IPC promises or unhandled exceptions.
 */
export async function invokeSafe<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw IpcError.fromResponse(error);
  }
}

