import { useState, useRef, useEffect } from 'react';
import { launchApplication, IpcError, type LaunchResultDto } from '../lib/ipc';

export type LaunchStatus = 'idle' | 'launching' | 'launched' | 'failed';

export interface LaunchButtonProps {
  applicationId: string;
  disabled?: boolean;
  status?: LaunchStatus;
  onSuccess?: (processId: number) => void;
  onError?: (error: IpcError) => void;
  className?: string;
  autoResetDelayMs?: number;
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
  onSuccess,
  onError,
  className = '',
  autoResetDelayMs = 0,
}: LaunchButtonProps) {
  const [internalStatus, setInternalStatus] = useState<LaunchStatus>('idle');
  const [launchedPid, setLaunchedPid] = useState<number | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const resetTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const status = controlledStatus ?? internalStatus;
  const isLaunching = status === 'launching';

  useEffect(() => {
    return () => {
      if (resetTimerRef.current) {
        clearTimeout(resetTimerRef.current);
      }
    };
  }, []);

  function scheduleReset() {
    if (autoResetDelayMs > 0) {
      if (resetTimerRef.current) clearTimeout(resetTimerRef.current);
      resetTimerRef.current = setTimeout(() => {
        setInternalStatus('idle');
        setLaunchedPid(null);
        setErrorMessage(null);
      }, autoResetDelayMs);
    }
  }

  async function handleLaunch() {
    if (isLaunching || disabled) {
      return;
    }

    setInternalStatus('launching');
    setErrorMessage(null);

    try {
      await executeLaunch(applicationId, {
        isBusy: () => isLaunching || disabled,
        onSuccess: (pid) => {
          setInternalStatus('launched');
          setLaunchedPid(pid);
          onSuccess?.(pid);
          scheduleReset();
        },
        onError: (err) => {
          setInternalStatus('failed');
          setErrorMessage(err.message);
          onError?.(err);
          scheduleReset();
        },
      });
    } catch {
      // Error handling is dispatched through onError callback
    }
  }

  const getButtonText = () => {
    switch (status) {
      case 'launching':
        return 'Launching...';
      case 'launched':
        return launchedPid ? `Launched (PID ${launchedPid})` : 'Launched';
      case 'failed':
        return 'Launch Failed';
      case 'idle':
      default:
        return 'Launch';
    }
  };

  const statusClass = `launch-button ${status} ${className}`.trim();

  return (
    <button
      type="button"
      className={statusClass}
      onClick={handleLaunch}
      disabled={disabled || isLaunching}
      aria-label={`Launch application ${applicationId}`}
      aria-busy={isLaunching}
      title={errorMessage ? `Error: ${errorMessage}` : undefined}
    >
      {getButtonText()}
    </button>
  );
}
