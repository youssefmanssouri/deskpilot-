import type { ApplicationDto } from '../lib/ipc';

/**
 * Single source of truth for when an application may be launched.
 *
 * Two levels of intent:
 * - Keyboard (Enter in search or on a focused card) is fast and easy to trigger by accident,
 *   so it only launches applications whose target was fully verified (`Available`).
 * - An explicit click on a Launch button is deliberate, so it also allows `Unverified` and
 *   `IncompleteMetadata` entries. Only `MissingTarget` can never be launched.
 */
export function canLaunchWithKeyboard(application: Pick<ApplicationDto, 'availability'>): boolean {
  return application.availability === 'Available';
}

export function canLaunchWithClick(application: Pick<ApplicationDto, 'availability'>): boolean {
  return application.availability !== 'MissingTarget';
}
