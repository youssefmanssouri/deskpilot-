import { useState, useRef, useEffect } from 'react';
import { launchApplication, IpcError, type LaunchResultDto } from '../lib/ipc';

export type LaunchStatus = 'idle' | 'launching' | 'launched' | 'failed';

/** Default duration in milliseconds to display launch feedback before resetting to idle. */
export const DEFAULT_LAUNCH_RESET_DELAY_MS = 3000;

export interface LaunchButtonProps {
  applicationId: string;
  disabled?: boolean;
  status?: LaunchStatus;
  launchedPid?: number | null;
  errorMessage?: string | null;
  onLaunch?: () => void;
  onSuccess?: (processId: number) => void;
  onError?: (error: IpcError) => void;
  className?: string;
  autoResetDelayMs?: number;
  ariaLabel?: string;
}

export interface ExecuteLaunchOptions {
  isBusy?: () => boolean;
  onStart?: () => void;
  onSuccess?: (processId: number) => void;
  onError?: (error: IpcError) => void;
}

/**
 * Executes a native application launch through the typed IPC boundary.
 *
 * Guarantees:
 * - Only `applicationId` is sent across IPC.
 * - Callers cannot supply paths, arguments, environment variables, or working directories.
 * - Prevents duplicate execution if `isBusy` returns true.
 * - Returns the typed `LaunchResultDto` or rethrows sanitized `IpcError`.
 */
export async function executeLaunch(
  applicationId: string,
  options?: ExecuteLaunchOptions
): Promise<LaunchResultDto | null> {
  if (options?.isBusy?.()) {
    return null;
  }

  options?.onStart?.();

  try {
    const result = await launchApplication(applicationId);
    options?.onSuccess?.(result.processId);
    return result;
  } catch (err) {
    const ipcErr = IpcError.fromResponse(err);
    options?.onError?.(ipcErr);
    throw ipcErr;
  }
}

/**
 * LaunchButton component encapsulates safe native application launching.
 *
 * Conforms strictly to the DeskPilot security boundary:
 * - Accepts only an opaque `applicationId`.
 * - Does not accept or forward filesystem paths, arguments, cwd, or environment parameters.
 * - Handles progression: idle -> launching -> launched or failed.
 * - Prevents double-click / duplicate launches while an operation is in flight.
 */
export function LaunchButton({
  applicationId,
  disabled = false,
  status: controlledStatus,
  launchedPid: controlledLaunchedPid,
  errorMessage: controlledErrorMessage,
  onLaunch,
  onSuccess,
  onError,
  className = '',
  autoResetDelayMs = DEFAULT_LAUNCH_RESET_DELAY_MS,
  ariaLabel,
}: LaunchButtonProps) {
  const [internalStatus, setInternalStatus] = useState<LaunchStatus>('idle');
  const [internalLaunchedPid, setInternalLaunchedPid] = useState<number | null>(null);
  const [internalErrorMessage, setInternalErrorMessage] = useState<string | null>(null);
  const resetTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const launchAttemptRef = useRef<number>(0);

  const status = controlledStatus ?? internalStatus;
  const isLaunching = status === 'launching';
  const displayPid = controlledLaunchedPid !== undefined ? controlledLaunchedPid : internalLaunchedPid;
  const displayError = controlledErrorMessage !== undefined ? controlledErrorMessage : internalErrorMessage;

  useEffect(() => {
    return () => {
      if (resetTimerRef.current) {
        clearTimeout(resetTimerRef.current);
        resetTimerRef.current = null;
      }
    };
  }, []);

  function scheduleReset(attempt: number) {
    if (autoResetDelayMs > 0) {
      if (resetTimerRef.current) {
        clearTimeout(resetTimerRef.current);
        resetTimerRef.current = null;
      }
      resetTimerRef.current = setTimeout(() => {
        // Prevent stale timers from resetting a newer launch attempt
        if (attempt === launchAttemptRef.current) {
          setInternalStatus('idle');
          setInternalLaunchedPid(null);
          setInternalErrorMessage(null);
          resetTimerRef.current = null;
        }
      }, autoResetDelayMs);
    }
  }

  async function handleLaunch() {
    if (isLaunching || disabled) {
      return;
    }

    if (resetTimerRef.current) {
      clearTimeout(resetTimerRef.current);
      resetTimerRef.current = null;
    }

    const currentAttempt = ++launchAttemptRef.current;
    setInternalStatus('launching');
    setInternalErrorMessage(null);

    try {
      await executeLaunch(applicationId, {
        isBusy: () => isLaunching || disabled,
        onSuccess: (pid) => {
          if (currentAttempt === launchAttemptRef.current) {
            setInternalStatus('launched');
            setInternalLaunchedPid(pid);
            onSuccess?.(pid);
            scheduleReset(currentAttempt);
          }
        },
        onError: (err) => {
          if (currentAttempt === launchAttemptRef.current) {
            setInternalStatus('failed');
            setInternalErrorMessage(err.message);
            onError?.(err);
            scheduleReset(currentAttempt);
          }
        },
      });
    } catch {
      // Error handling is dispatched through onError callback
    }
  }

  function handleClick() {
    if (onLaunch) {
      if (!disabled && !isLaunching) {
        onLaunch();
      }
    } else {
      void handleLaunch();
    }
  }

  const getButtonText = () => {
    switch (status) {
      case 'launching':
        return 'Opening…';
      case 'launched':
        return displayPid ? `Opened (PID ${displayPid})` : 'Opened';
      case 'failed':
        return "Couldn't open";
      case 'idle':
      default:
        return 'Open';
    }
  };

  const statusClass = `launch-button ${status} ${className}`.trim();

  return (
    <button
      type="button"
      className={statusClass}
      onClick={handleClick}
      disabled={disabled || isLaunching}
      aria-label={ariaLabel ?? `Open application ${applicationId}`}
      aria-busy={isLaunching}
      title={displayError ? `Error: ${displayError}` : undefined}
    >
      {getButtonText()}
    </button>
  );
}
