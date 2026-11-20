/**
 * Turns a sanitized launch error from the Rust IPC boundary into plain-language guidance.
 *
 * The backend deliberately sends only an error category (`code`) and a sanitized message.
 * It does not forward raw Win32 error numbers, so this mapping describes the category
 * honestly instead of guessing a specific Windows cause.
 */
export interface LaunchErrorDescription {
  title: string;
  hint: string;
}

export function describeLaunchError(code: string | null | undefined): LaunchErrorDescription {
  switch (code) {
    case 'PERMISSION_DENIED':
      return {
        title: 'Windows or DeskPilot blocked this launch',
        hint:
          'Try opening the app from the Start menu. If it is blocked there too, a system or security policy is stopping it.',
      };
    case 'NOT_FOUND':
    case 'UNAVAILABLE':
      return {
        title: "The app's program file could not be found",
        hint: 'It may have been uninstalled or moved. Refresh the catalog, then try again.',
      };
    case 'INVALID_INPUT':
      return {
        title: 'This app entry cannot be launched safely',
        hint: 'Its registry or Start menu entry is incomplete or points somewhere DeskPilot does not allow.',
      };
    default:
      return {
        title: 'Windows could not start this app',
        hint:
          'Try again, or open it from the Start menu. Security software or an application control policy can cause this.',
      };
  }
}
