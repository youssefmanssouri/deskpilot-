/**
 * Machine-readable error codes returned across the Tauri IPC boundary.
 */
export type ErrorCode =
  | 'INVALID_INPUT'
  | 'NOT_FOUND'
  | 'PERMISSION_DENIED'
  | 'UNAVAILABLE'
  | 'OPERATION_FAILED'
  | 'INTERNAL_ERROR';

/**
 * Serialized error Data Transfer Object received from the Rust core.
 */
export interface IpcErrorDto {
  code: ErrorCode | string;
  message: string;
  details?: string;
}

/**
 * Application status DTO returned by `get_app_status`.
 */
export interface AppStatusDto {
  appName: string;
  version: string;
  status: string;
  uptimeSeconds: number;
  environment: string;
}

/**
 * High-level command intent DTO.
 * Expresses user/UI intent without exposing raw OS paths or commands.
 */
export interface CommandIntentDto {
  intentId: string;
  actionType: 'inspect' | 'status' | 'query' | 'preview' | string;
  targetId?: string;
}

