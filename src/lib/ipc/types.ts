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

/**
 * CPU metrics returned by `get_system_metrics`.
 */
export interface CpuMetricsDto {
  brandName: string;
  logicalCores: number;
  architecture: string;
  utilizationPercent: number;
}

/**
 * Physical RAM metrics returned by `get_system_metrics`.
 */
export interface MemoryMetricsDto {
  totalBytes: number;
  availableBytes: number;
  usedBytes: number;
  usedPercent: number;
}

/**
 * Fixed storage drive metrics returned by `get_system_metrics`.
 */
export interface DriveMetricsDto {
  rootPath: string;
  volumeName: string;
  fileSystem: string;
  totalBytes: number;
  availableBytes: number;
  usedBytes: number;
  usedPercent: number;
}

/**
 * Local storage collection returned by `get_system_metrics`.
 */
export interface StorageMetricsDto {
  drives: DriveMetricsDto[];
}

/**
 * System and application uptime information returned by `get_system_metrics`.
 */
export interface OsInfoMetricsDto {
  uptimeSeconds: number;
  appUptimeSeconds: number;
  osName: string;
  architecture: string;
}

/**
 * Top-level System Intelligence snapshot returned by `get_system_metrics`.
 */
export interface SystemMetricsDto {
  cpu: CpuMetricsDto;
  memory: MemoryMetricsDto;
  storage: StorageMetricsDto;
  system: OsInfoMetricsDto;
  sampledAtMs: number;
}

/**
 * One recent launch from the local history (most recent first).
 * Holds only the application id, its display name and the launch time; never paths.
 */
export interface LaunchHistoryEntryDto {
  applicationId: string;
  displayName: string;
  /** Milliseconds since the Unix epoch. */
  launchedAtMs: number;
}

/** Whether the quick launcher's global shortcut is active. */
export interface ShortcutStatusDto {
  /** Human-readable shortcut, e.g. "Ctrl+Alt+Space". */
  accelerator: string;
  registered: boolean;
  /** Plain-language explanation when registration failed. */
  message: string | null;
}

/** Views the main window can be asked to show. Mirrors CatalogView. */
export type NavigableView = 'home' | 'applications' | 'processes' | 'system';

/** Payload of the "deskpilot://navigate" event sent to the main window. */
export interface NavigateRequestDto {
  view: NavigableView;
  applicationId: string | null;
}

/** Theme preference: follow Windows, or force light/dark. */
export type ThemePreference = 'system' | 'light' | 'dark';

/** Sanitized startup registration state (see commands::settings::StartupStatusState). */
export type StartupStatusState =
  | 'disabled'
  | 'enabled'
  | 'stale'
  | 'otherInstall'
  | 'unrecognized'
  | 'unavailable';

/** Detailed startup registration status returned inside SettingsDto. */
export interface StartupStatusDto {
  state: StartupStatusState;
  errorCode?: string;
}

/** Everything the Settings page shows (see commands::settings::SettingsDto). */
export interface SettingsDto {
  theme: ThemePreference;
  historyEnabled: boolean;
  /** Canonical form, e.g. "Ctrl+Super+K". */
  launcherShortcut: string;
  /** Display form, e.g. "Ctrl+Win+K". */
  launcherShortcutLabel: string;
  shortcutRegistered: boolean;
  shortcutMessage: string | null;
  /**
   * Backward-compatible boolean view of startup status; null when stale,
   * owned by another install, unrecognized, or unavailable.
   */
  startWithWindows: boolean | null;
  /** Detailed sanitized startup status. */
  startupStatus: StartupStatusDto;
  /** Saved preference (applies after restart). */
  glassEffect: boolean;
  /** True on Windows 11. */
  glassSupported: boolean;
  /** True if glass is on in this session. */
  glassActive: boolean;
}
