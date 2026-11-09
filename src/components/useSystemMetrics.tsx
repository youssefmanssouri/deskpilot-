import React, {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  useRef,
  type ReactNode,
} from 'react';
import { deskpilotApi, type SystemMetricsDto, IpcError } from '../lib/ipc';

export type RefreshMode = 'manual' | '3s' | '5s';

/** Maximum number of historical CPU utilization samples retained in memory. */
export const MAX_CPU_HISTORY_SAMPLES = 30;

export interface SystemMetricsState {
  metrics: SystemMetricsDto | null;
  cpuHistory: number[];
  isLoading: boolean;
  errorMsg: string | null;
  refreshMode: RefreshMode;
  setRefreshMode: (mode: RefreshMode) => void;
  isPaused: boolean;
  lastUpdated: Date | null;
  refresh: (isExplicitManual?: boolean) => Promise<void>;
}

export interface UseSystemMetricsOptions {
  initialData?: SystemMetricsDto;
  autoFetch?: boolean;
  defaultRefreshMode?: RefreshMode;
}

/**
 * Appends a new CPU utilization sample to a bounded historical buffer.
 *
 * Guarantees:
 * - Strictly retains at most `MAX_CPU_HISTORY_SAMPLES` (30).
 * - Ignores invalid, NaN, infinite, or negative values without fabricating zeros.
 * - Clamps values between 0.0 and 100.0%.
 */
export function appendCpuSample(
  prevHistory: number[],
  newSample: number | null | undefined
): number[] {
  if (
    newSample === null ||
    newSample === undefined ||
    typeof newSample !== 'number' ||
    isNaN(newSample) ||
    !isFinite(newSample) ||
    newSample < 0
  ) {
    return prevHistory;
  }
  const clamped = Math.max(0, Math.min(100, newSample));
  const updated = [...prevHistory, clamped];
  return updated.length > MAX_CPU_HISTORY_SAMPLES
    ? updated.slice(-MAX_CPU_HISTORY_SAMPLES)
    : updated;
}

/**
 * Hook managing system metrics polling, bounded CPU history, and lifecycle.
 */
export function useSystemMetrics({
  initialData,
  autoFetch = true,
  defaultRefreshMode = 'manual',
}: UseSystemMetricsOptions = {}): SystemMetricsState {
  const [metrics, setMetrics] = useState<SystemMetricsDto | null>(initialData ?? null);
  const [cpuHistory, setCpuHistory] = useState<number[]>(() => {
    if (initialData?.cpu?.utilizationPercent !== undefined) {
      return appendCpuSample([], initialData.cpu.utilizationPercent);
    }
    return [];
  });
  const [isLoading, setIsLoading] = useState<boolean>(!initialData && autoFetch);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [refreshMode, setRefreshMode] = useState<RefreshMode>(defaultRefreshMode);
  const [isPaused, setIsPaused] = useState<boolean>(false);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(
    initialData ? new Date(initialData.sampledAtMs) : null
  );

  const requestSeqRef = useRef<number>(0);

  const refresh = useCallback(async (isExplicitManual = false) => {
    const seq = ++requestSeqRef.current;
    if (isExplicitManual) {
      setIsLoading(true);
    }
    setErrorMsg(null);

    try {
      const data = await deskpilotApi.getSystemMetrics();
      if (seq === requestSeqRef.current) {
        setMetrics(data);
        setCpuHistory((prev) => appendCpuSample(prev, data.cpu?.utilizationPercent));
        setLastUpdated(new Date(data.sampledAtMs));
        setIsLoading(false);
      }
    } catch (err: unknown) {
      if (seq === requestSeqRef.current) {
        if (err instanceof IpcError) {
          setErrorMsg(err.message);
        } else if (err instanceof Error) {
          setErrorMsg(err.message);
        } else {
          setErrorMsg('An unexpected error occurred while querying system metrics.');
        }
        setIsLoading(false);
      }
    }
  }, []);

  // Initial fetch on mount
  useEffect(() => {
    if (autoFetch && !initialData) {
      void refresh(true);
    }
  }, [autoFetch, initialData, refresh]);

  // Pause polling when browser/desktop window is not visible to conserve battery and CPU
  useEffect(() => {
    const handleVisibilityChange = () => {
      setIsPaused(document.visibilityState === 'hidden');
    };
    document.addEventListener('visibilitychange', handleVisibilityChange);
    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
    };
  }, []);

  // Polling interval timer
  useEffect(() => {
    if (refreshMode === 'manual' || isPaused) {
      return;
    }
    const intervalMs = refreshMode === '3s' ? 3000 : 5000;
    const timer = setInterval(() => {
      void refresh(false);
    }, intervalMs);

    return () => {
      clearInterval(timer);
    };
  }, [refreshMode, isPaused, refresh]);

  return {
    metrics,
    cpuHistory,
    isLoading,
    errorMsg,
    refreshMode,
    setRefreshMode,
    isPaused,
    lastUpdated,
    refresh,
  };
}

const SystemMetricsContext = createContext<SystemMetricsState | null>(null);

export interface SystemMetricsProviderProps extends UseSystemMetricsOptions {
  children: ReactNode;
}

/**
 * SystemMetricsProvider establishes a single consolidated polling source
 * for DeskPilot system telemetry across the entire application shell.
 */
export const SystemMetricsProvider: React.FC<SystemMetricsProviderProps> = ({
  children,
  initialData,
  autoFetch = true,
  defaultRefreshMode = '5s',
}) => {
  const state = useSystemMetrics({
    initialData,
    autoFetch,
    defaultRefreshMode,
  });

  return (
    <SystemMetricsContext.Provider value={state}>
      {children}
    </SystemMetricsContext.Provider>
  );
};

/**
 * Consumes the shared system metrics from context if available, or null.
 */
export function useSharedSystemMetrics(): SystemMetricsState | null {
  return useContext(SystemMetricsContext);
}

