import { useState } from 'react';
import { launchApplication, IpcError } from '../lib/ipc';

export interface LaunchButtonProps {
  applicationId: string;
  disabled?: boolean;
  onSuccess?: (processId: number) => void;
  onError?: (error: IpcError) => void;
}

/**
 * LaunchButton component encapsulates safe native application launching.
 *
 * Conforms strictly to the DeskPilot security boundary:
 * - Accepts only an opaque `applicationId`.
 * - Does not accept or forward filesystem paths, arguments, cwd, or environment parameters.
 * - Handles loading and error states locally while dispatching typed notifications.
 */
export function LaunchButton({
  applicationId,
  disabled = false,
  onSuccess,
  onError,
}: LaunchButtonProps) {
  const [isLaunching, setIsLaunching] = useState(false);

  async function handleLaunch() {
    if (isLaunching || disabled) return;
    setIsLaunching(true);
    try {
      const result = await launchApplication(applicationId);
      onSuccess?.(result.processId);
    } catch (err) {
      const ipcErr = IpcError.fromResponse(err);
      onError?.(ipcErr);
    } finally {
      setIsLaunching(false);
    }
  }

  return (
    <button
      type="button"
      className="launch-button"
      onClick={handleLaunch}
      disabled={disabled || isLaunching}
      aria-label={`Launch application ${applicationId}`}
    >
      {isLaunching ? 'Launching...' : 'Launch'}
    </button>
  );
}
