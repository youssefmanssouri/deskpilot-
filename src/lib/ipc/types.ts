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

/**
 * Operational availability status of a discovered application.
 */
export type AvailabilityStateDto =
  | 'Available'
  | 'MissingTarget'
  | 'IncompleteMetadata'
  | 'Unverified';

/**
 * Safe frontend representation of a discovered Windows application.
 *
 * Excludes machine-specific file paths, usernames, or registry subkeys.
 */
export interface ApplicationDto {
  id: string;
  displayName: string;
  publisher?: string;
  version?: string;
  sources: string[];
  availability: AvailabilityStateDto;
  isInstalled: boolean;
}

/**
 * Payload returned by `discover_applications` IPC command.
 */
export interface DiscoveryResultDto {
  applications: ApplicationDto[];
  scannedSources: string[];
  partialFailures: string[];
  totalDiscovered: number;
}

/**
 * Safe frontend representation of a process launch outcome.
 *
 * Contains strictly the opaque application identifier and the assigned OS process ID.
 * Machine filesystem paths, arguments, working directories, and environment variables are omitted.
 */
export interface LaunchResultDto {
  applicationId: string;
  processId: number;
}

export type LaunchResult = LaunchResultDto;

/**
 * Factual query status returned by the operating system for a process.
 * Matches Rust `ProcessQueryStatus` camelCase serde serialization.
 */
export type ProcessQueryStatusDto = 'full' | 'partial' | 'handleUnavailable' | 'exited';

/**
 * Safe frontend Data Transfer Object representing an enumerated Windows process.
 * Encapsulates process metadata without granting executable capabilities.
 */
export interface ProcessDto {
  /** Deterministic process identity token (`v1:{pid}:{ticksHex}` or `eph:{seq}:{pid}`). */
  id: string;
  /** Operating system Process Identifier (PID). */
  pid: number;
  /** Parent process identifier reported at creation time. */
  parentPid: number;
  /** Base executable image name (e.g. "code.exe", "explorer.exe"). */
  name: string;
  /** Full canonical executable path if accessible, otherwise null. */
  executablePath: string | null;
  /** Creation timestamp formatted as an ISO-8601 UTC string if accessible, otherwise null. */
  createdAt: string | null;
  /** Physical resident memory (working set) in bytes if accessible, otherwise null. */
  workingSetBytes: number | null;
  /** Cumulative CPU execution time in milliseconds (kernel + user) if accessible, otherwise null. */
  cpuTimeMs: number | null;
  /** Terminal Services session identifier if accessible, otherwise null. */
  sessionId: number | null;
  /** Factual query status returned by the operating system. */
  status: ProcessQueryStatusDto;
}

/**
 * Payload returned by `list_processes` IPC command.
 */
export interface ProcessListResultDto {
  /** Deterministically ordered list of at most K retained processes. */
  processes: ProcessDto[];
  /** Exact count of all processes enumerated in the snapshot walk before truncation. */
  totalProcesses: number;
  /** Point in time (Unix epoch milliseconds) when snapshot acquisition succeeded. */
  capturedAtMs: number;
  /** True if total observed processes exceeded the maximum retention ceiling (2,000). */
  isTruncated: boolean;
}

