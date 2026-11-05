import { useState, useRef, useEffect, useCallback } from 'react';
import { executeLaunch, DEFAULT_LAUNCH_RESET_DELAY_MS, type LaunchStatus } from './LaunchButton';
import { IpcError, type LaunchResultDto } from '../lib/ipc';

export interface ActiveLaunchState {
  applicationId: string;
  status: LaunchStatus;
  launchedPid?: number | null;
  errorMessage?: string | null;
}

export interface UseApplicationLauncherOptions {
  onSuccess?: (applicationId: string, processId: number) => void;
  onError?: (applicationId: string, error: IpcError) => void;
  autoResetDelayMs?: number;
}

/**
 * Consolidated React hook for managing application launch execution,
 * visual state transitions (idle -> launching -> launched / failed -> idle),
 * debounced duplicate-launch prevention, and auto-reset timers.
 */
export function useApplicationLauncher({
  onSuccess,
  onError,
  autoResetDelayMs = DEFAULT_LAUNCH_RESET_DELAY_MS,
}: UseApplicationLauncherOptions = {}) {
  const [activeLaunch, setActiveLaunch] = useState<ActiveLaunchState | null>(null);
  const isLaunchingRef = useRef<boolean>(false);
  const resetTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const launchAttemptRef = useRef<number>(0);

  useEffect(() => {
    return () => {
      if (resetTimerRef.current) {
        clearTimeout(resetTimerRef.current);
        resetTimerRef.current = null;
      }
    };
  }, []);

  const isBusy = useCallback(() => {
    return isLaunchingRef.current || activeLaunch?.status === 'launching';
  }, [activeLaunch?.status]);

  const scheduleReset = useCallback(
    (attempt: number) => {
      if (autoResetDelayMs > 0) {
        if (resetTimerRef.current) {
          clearTimeout(resetTimerRef.current);
          resetTimerRef.current = null;
        }
        resetTimerRef.current = setTimeout(() => {
          if (attempt === launchAttemptRef.current) {
            setActiveLaunch(null);
            resetTimerRef.current = null;
          }
        }, autoResetDelayMs);
      }
    },
    [autoResetDelayMs]
  );

  const launch = useCallback(
    async (applicationId: string): Promise<LaunchResultDto | null> => {
      if (isLaunchingRef.current || activeLaunch?.status === 'launching') {
        return null;
      }
      isLaunchingRef.current = true;

      if (resetTimerRef.current) {
        clearTimeout(resetTimerRef.current);
        resetTimerRef.current = null;
      }

      const currentAttempt = ++launchAttemptRef.current;
      setActiveLaunch({
        applicationId,
        status: 'launching',
      });

      try {
        const result = await executeLaunch(applicationId, {
          isBusy: () => false,
          onSuccess: (pid) => {
            if (currentAttempt === launchAttemptRef.current) {
              setActiveLaunch({
                applicationId,
                status: 'launched',
                launchedPid: pid,
              });
              onSuccess?.(applicationId, pid);
              scheduleReset(currentAttempt);
            }
          },
          onError: (err) => {
            if (currentAttempt === launchAttemptRef.current) {
              setActiveLaunch({
                applicationId,
                status: 'failed',
                errorMessage: err.message,
              });
              onError?.(applicationId, err);
              scheduleReset(currentAttempt);
            }
          },
        });
        return result;
      } catch {
        return null;
      } finally {
        isLaunchingRef.current = false;
      }
    },
    [activeLaunch?.status, onSuccess, onError, scheduleReset]
  );

  return {
    activeLaunch,
    launch,
    isBusy,
  };
}
