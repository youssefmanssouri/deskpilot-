import { invokeSafe } from './client';
import type {
  AppStatusDto,
  ApplicationDto,
  CommandIntentDto,
  DiscoveryResultDto,
  LaunchHistoryEntryDto,
  NavigableView,
  ShortcutStatusDto,
  LaunchResult,
  LaunchResultDto,
  ProcessListResultDto,
  SystemMetricsDto,
} from './types';

/**
 * Domain-level IPC API client for DeskPilot.
 *
 * Encapsulates all Tauri command invocations within strongly-typed functions.
 * Components must invoke these methods rather than calling `invoke()` directly.
 */
export const deskpilotApi = {
  /**
   * Retrieves the current application status from the Rust core.
   */
  getAppStatus(): Promise<AppStatusDto> {
    return invokeSafe<AppStatusDto>('get_app_status');
  },

  /**
   * Validates a high-level command intent against Rust security boundaries.
   */
  validateCommandIntent(intent: CommandIntentDto): Promise<void> {
    return invokeSafe<void>('validate_command_intent', { intent });
  },

  /**
   * Discovers installed Windows applications across Registry hives and Start Menu shortcuts.
   */
  discoverApplications(): Promise<DiscoveryResultDto> {
    return invokeSafe<DiscoveryResultDto>('discover_applications');
  },

  /**
   * Launches an installed application by its catalog identifier.
   *
   * Only the opaque application ID is passed across the boundary;
   * executable paths, arguments, working directories, or environment overrides
   * are strictly forbidden and cannot be provided.
   */
  launchApplication(applicationId: string): Promise<LaunchResultDto> {
    return invokeSafe<LaunchResultDto>('launch_application', { applicationId });
  },

  /**
   * Enumerate running Windows processes with bounded top-K selection and limited privilege.
   */
  listProcesses(): Promise<ProcessListResultDto> {
    return invokeSafe<ProcessListResultDto>('list_processes');
  },

  /**
   * Queries real-time System Intelligence metrics (CPU, memory, storage, uptime).
   */
  getSystemMetrics(): Promise<SystemMetricsDto> {
    return invokeSafe<SystemMetricsDto>('get_system_metrics');
  },

  /** Recent launches recorded by the backend after successful launches. */
  getLaunchHistory(): Promise<LaunchHistoryEntryDto[]> {
    return invokeSafe<LaunchHistoryEntryDto[]>('get_launch_history');
  },

  /** Deletes the local launch history (memory and file). */
  clearLaunchHistory(): Promise<void> {
    return invokeSafe<void>('clear_launch_history');
  },

  /** Current catalog without rescanning (empty until discovery has run once). */
  listApplications(): Promise<ApplicationDto[]> {
    return invokeSafe<ApplicationDto[]>('list_applications');
  },

  /** Hides the quick launcher window. */
  hideLauncher(): Promise<void> {
    return invokeSafe<void>('hide_launcher');
  },

  /** Shows the main window on a view, optionally with an app selected; hides the launcher. */
  showMainWindow(view: NavigableView, applicationId?: string): Promise<void> {
    return invokeSafe<void>('show_main_window', {
      view,
      applicationId: applicationId ?? null,
    });
  },

  /** Whether the quick launcher's global shortcut is registered. */
  getLauncherShortcut(): Promise<ShortcutStatusDto> {
    return invokeSafe<ShortcutStatusDto>('get_launcher_shortcut');
  },
};

/**
 * Standalone typed helper for launching an application by its catalog ID.
 */
export function launchApplication(applicationId: string): Promise<LaunchResult> {
  return deskpilotApi.launchApplication(applicationId);
}

/**
 * Standalone typed helper for enumerating running processes.
 */
export function listProcesses(): Promise<ProcessListResultDto> {
  return deskpilotApi.listProcesses();
}

/**
 * Standalone typed helper for querying system metrics.
 */
export function getSystemMetrics(): Promise<SystemMetricsDto> {
  return deskpilotApi.getSystemMetrics();
}
