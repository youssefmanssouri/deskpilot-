import { invokeSafe } from './client';

/** Edge length in pixels of icons returned by `getApplicationIcon` (matches the Rust side). */
export const APP_ICON_SIZE = 64;
import type {
  AppStatusDto,
  ApplicationDto,
  CommandIntentDto,
  DiscoveryResultDto,
  LaunchHistoryEntryDto,
  NavigableView,
  SettingsDto,
  ShortcutStatusDto,
  ThemePreference,
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
   * Real icon of a catalog application as raw RGBA bytes (64 x 64, top row first),
   * or `null` when it has none. Looked up by catalog ID only.
   */
  async getApplicationIcon(applicationId: string): Promise<Uint8ClampedArray<ArrayBuffer> | null> {
    const raw = await invokeSafe<ArrayBuffer | number[] | null>('get_application_icon', {
      applicationId,
    });
    if (!raw) return null;
    const bytes = raw instanceof ArrayBuffer ? new Uint8ClampedArray(raw) : Uint8ClampedArray.from(raw);
    return bytes.length === APP_ICON_SIZE * APP_ICON_SIZE * 4 ? bytes : null;
  },

  /** True when DeskPilot runs as administrator, so apps it launches do too. */
  isRunningElevated(): Promise<boolean> {
    return invokeSafe<boolean>('is_running_elevated');
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

  getSettings(): Promise<SettingsDto> {
    return invokeSafe<SettingsDto>('get_settings');
  },

  setTheme(theme: ThemePreference): Promise<SettingsDto> {
    return invokeSafe<SettingsDto>('set_theme', { theme });
  },

  /** Turning history off also clears it. */
  setHistoryEnabled(enabled: boolean): Promise<SettingsDto> {
    return invokeSafe<SettingsDto>('set_history_enabled', { enabled });
  },

  /** Rejected with an IpcError if the shortcut is invalid or taken; the old one stays active. */
  setLauncherShortcut(shortcut: string): Promise<SettingsDto> {
    return invokeSafe<SettingsDto>('set_launcher_shortcut', { shortcut });
  },

  setStartWithWindows(enabled: boolean): Promise<SettingsDto> {
    return invokeSafe<SettingsDto>('set_start_with_windows', { enabled });
  },

  /** Saved now; applied the next time DeskPilot starts (Windows 11 only). */
  setGlassEffect(enabled: boolean): Promise<SettingsDto> {
    return invokeSafe<SettingsDto>('set_glass_effect', { enabled });
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
