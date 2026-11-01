import { invokeSafe } from './client';
import type { AppStatusDto, CommandIntentDto } from './types';

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
};

