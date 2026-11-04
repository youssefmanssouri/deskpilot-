import { useState, useEffect, useCallback, useRef } from 'react';
import {
  deskpilotApi,
  type SystemMetricsDto,
  IpcError,
} from '../lib/ipc';
import './SystemMonitor.css';

export interface SystemMonitorProps {
  initialData?: SystemMetricsDto;
  autoFetch?: boolean;
  className?: string;
}

export type RefreshMode = 'manual' | '3s' | '5s';

/**
 * Formats a byte quantity into human-readable units (B, KB, MB, GB, TB).
 */
export function formatBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined || isNaN(bytes)) {
    return '—';
  }
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  const kb = bytes / 1024;
  if (kb < 1024) {
    return `${kb.toFixed(1)} KB`;
  }
  const mb = kb / 1024;
  if (mb < 1024) {
    return `${mb.toFixed(1)} MB`;
  }
  const gb = mb / 1024;
  if (gb < 1024) {
    return `${gb.toFixed(1)} GB`;
  }
  const tb = gb / 1024;
  return `${tb.toFixed(2)} TB`;
}

/**
 * Formats duration in seconds into a friendly elapsed string (e.g. "3d 4h 12m" or "2h 15m 30s").
 */
export function formatUptime(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined || isNaN(seconds) || seconds < 0) {
    return '—';
  }
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);

  if (days > 0) {
    return `${days}d ${hours}h ${minutes}m`;
  }
  if (hours > 0) {
    return `${hours}h ${minutes}m ${secs}s`;
  }
  if (minutes > 0) {
    return `${minutes}m ${secs}s`;
  }
  return `${secs}s`;
}

/**
 * Maps a utilization percentage (0-100) to a semantic progress bar color class.
 */
export function getLoadColorClass(percent: number): string {
  if (percent >= 85) {
    return 'load-critical';
  }
  if (percent >= 60) {
    return 'load-warning';
  }
  return 'load-normal';
}

export const SystemMonitor: React.FC<SystemMonitorProps> = ({
  initialData,
  autoFetch = true,
  className = '',
}) => {
  const [metrics, setMetrics] = useState<SystemMetricsDto | null>(initialData ?? null);
  const [isLoading, setIsLoading] = useState<boolean>(!initialData && autoFetch);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [refreshMode, setRefreshMode] = useState<RefreshMode>('manual');
  const [isPaused, setIsPaused] = useState<boolean>(false);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(
    initialData ? new Date(initialData.sampledAtMs) : null
  );

  // Monotonic request counter to guard against race conditions and stale out-of-order updates
  const requestSeqRef = useRef<number>(0);

  const fetchMetrics = useCallback(async (isExplicitManual = false) => {
    const seq = ++requestSeqRef.current;
    if (isExplicitManual) {
      setIsLoading(true);
    }
    setErrorMsg(null);

    try {
      const data = await deskpilotApi.getSystemMetrics();
      // Drop response if a newer query was issued while this one was in flight
      if (seq === requestSeqRef.current) {
        setMetrics(data);
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
      fetchMetrics(true);
    }
  }, [autoFetch, initialData, fetchMetrics]);

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
      fetchMetrics(false);
    }, intervalMs);

    return () => {
      clearInterval(timer);
    };
  }, [refreshMode, isPaused, fetchMetrics]);

  return (
    <div
      className={`system-monitor-container ${className}`.trim()}
      role="region"
      aria-label="System Intelligence Dashboard"
    >
      {/* Top Toolbar */}
      <div className="system-toolbar">
        <div className="system-toolbar-info">
          <h2 className="system-title">System Intelligence</h2>
          <div className="system-subtitle-row">
            {lastUpdated && (
              <span className="system-timestamp">
                Updated {lastUpdated.toLocaleTimeString()}
              </span>
            )}
            {isPaused && refreshMode !== 'manual' && (
              <span className="system-badge paused-badge" role="status">
                Polling Paused (Window Inactive)
              </span>
            )}
          </div>
        </div>

        <div className="system-toolbar-actions">
          <div className="system-refresh-mode-wrapper">
            <label htmlFor="system-refresh-mode" className="system-mode-label">
              Refresh Mode
            </label>
            <select
              id="system-refresh-mode"
              className="system-mode-select"
              value={refreshMode}
              onChange={(e) => setRefreshMode(e.target.value as RefreshMode)}
              aria-label="Select auto-refresh interval"
            >
              <option value="manual">Manual</option>
              <option value="3s">Auto (3s)</option>
              <option value="5s">Auto (5s)</option>
            </select>
          </div>

          <button
            type="button"
            className="system-refresh-button"
            onClick={() => fetchMetrics(true)}
            disabled={isLoading}
            aria-label="Refresh system metrics"
          >
            {isLoading ? (
              <>
                <span className="spinner-icon" aria-hidden="true" />
                <span>Reading...</span>
              </>
            ) : (
              <>
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 16 16"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  aria-hidden="true"
                >
                  <path d="M14 2v4h-4" />
                  <path d="M2 14v-4h4" />
                  <path d="M3.51 9a9 9 0 0 1 14.85-3.36L20 7" />
                  <path d="M12.49 7a9 9 0 0 1-14.85 3.36L4 9" />
                </svg>
                <span>Refresh</span>
              </>
            )}
          </button>
        </div>
      </div>

      {/* Error Banner */}
      {errorMsg && (
        <div className="system-error-banner" role="alert">
          <div className="system-error-content">
            <p className="system-error-title">Unable to retrieve system metrics</p>
            <p className="system-error-detail">{errorMsg}</p>
          </div>
          <button
            type="button"
            className="system-retry-button"
            onClick={() => fetchMetrics(true)}
            aria-label="Retry loading metrics"
          >
            Retry
          </button>
        </div>
      )}

      {/* Main Metrics Cards */}
      {isLoading && !metrics ? (
        <div className="system-loading-state" role="status">
          <div className="system-loading-spinner" aria-hidden="true" />
          <p>Sampling system metrics from Windows...</p>
        </div>
      ) : metrics ? (
        <div className="system-content-wrapper">
          <div className="system-metrics-grid">
            {/* CPU Card */}
            <div className="system-card" role="region" aria-label="Processor Metrics">
              <div className="system-card-header">
                <span className="system-card-title">Processor (CPU)</span>
                <span className="system-badge arch-badge">{metrics.cpu.architecture}</span>
              </div>
              <div className="system-card-body">
                <div className="system-metric-value-row">
                  <span className="system-metric-hero">
                    {metrics.cpu.utilizationPercent.toFixed(1)}%
                  </span>
                  <span className="system-metric-sub">
                    {metrics.cpu.logicalCores} Logical Cores
                  </span>
                </div>
                <div
                  className="system-progress-track"
                  role="progressbar"
                  aria-valuenow={metrics.cpu.utilizationPercent}
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-label="CPU load percentage"
                >
                  <div
                    className={`system-progress-fill ${getLoadColorClass(
                      metrics.cpu.utilizationPercent
                    )}`}
                    style={{ width: `${Math.min(100, Math.max(0, metrics.cpu.utilizationPercent))}%` }}
                  />
                </div>
                <p className="system-hardware-desc" title={metrics.cpu.brandName}>
                  {metrics.cpu.brandName}
                </p>
              </div>
            </div>

            {/* Memory Card */}
            <div className="system-card" role="region" aria-label="Physical RAM Metrics">
              <div className="system-card-header">
                <span className="system-card-title">Memory (RAM)</span>
                <span className="system-badge mem-badge">
                  {metrics.memory.usedPercent.toFixed(1)}%
                </span>
              </div>
              <div className="system-card-body">
                <div className="system-metric-value-row">
                  <span className="system-metric-hero">
                    {formatBytes(metrics.memory.usedBytes)}
                  </span>
                  <span className="system-metric-sub">
                    / {formatBytes(metrics.memory.totalBytes)}
                  </span>
                </div>
                <div
                  className="system-progress-track"
                  role="progressbar"
                  aria-valuenow={metrics.memory.usedPercent}
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-label="Memory load percentage"
                >
                  <div
                    className={`system-progress-fill ${getLoadColorClass(
                      metrics.memory.usedPercent
                    )}`}
                    style={{ width: `${Math.min(100, Math.max(0, metrics.memory.usedPercent))}%` }}
                  />
                </div>
                <p className="system-hardware-desc">
                  {formatBytes(metrics.memory.availableBytes)} available for processes
                </p>
              </div>
            </div>

            {/* Storage Summary Card */}
            <div className="system-card" role="region" aria-label="Storage Summary">
              <div className="system-card-header">
                <span className="system-card-title">Local Storage</span>
                <span className="system-badge storage-badge">
                  {metrics.storage.drives.length} Volume{metrics.storage.drives.length === 1 ? '' : 's'}
                </span>
              </div>
              <div className="system-card-body">
                {(() => {
                  const totalCapacity = metrics.storage.drives.reduce(
                    (acc, d) => acc + d.totalBytes,
                    0
                  );
                  const totalUsed = metrics.storage.drives.reduce(
                    (acc, d) => acc + d.usedBytes,
                    0
                  );
                  const overallPercent =
                    totalCapacity > 0 ? (totalUsed / totalCapacity) * 100 : 0;
                  return (
                    <>
                      <div className="system-metric-value-row">
                        <span className="system-metric-hero">
                          {formatBytes(totalUsed)}
                        </span>
                        <span className="system-metric-sub">
                          / {formatBytes(totalCapacity)}
                        </span>
                      </div>
                      <div
                        className="system-progress-track"
                        role="progressbar"
                        aria-valuenow={overallPercent}
                        aria-valuemin={0}
                        aria-valuemax={100}
                        aria-label="Overall storage used percentage"
                      >
                        <div
                          className={`system-progress-fill ${getLoadColorClass(overallPercent)}`}
                          style={{ width: `${Math.min(100, Math.max(0, overallPercent))}%` }}
                        />
                      </div>
                      <p className="system-hardware-desc">
                        {formatBytes(totalCapacity - totalUsed)} free across all fixed drives
                      </p>
                    </>
                  );
                })()}
              </div>
            </div>

            {/* System & Uptime Card */}
            <div className="system-card" role="region" aria-label="System Runtime & Uptime">
              <div className="system-card-header">
                <span className="system-card-title">System Uptime</span>
                <span className="system-badge os-badge">{metrics.system.osName}</span>
              </div>
              <div className="system-card-body">
                <div className="system-metric-value-row">
                  <span className="system-metric-hero">
                    {formatUptime(metrics.system.uptimeSeconds)}
                  </span>
                </div>
                <div className="system-uptime-details">
                  <div className="system-uptime-line">
                    <span className="uptime-label">App Session:</span>
                    <strong className="uptime-val">
                      {formatUptime(metrics.system.appUptimeSeconds)}
                    </strong>
                  </div>
                  <div className="system-uptime-line">
                    <span className="uptime-label">Architecture:</span>
                    <strong className="uptime-val">{metrics.system.architecture}</strong>
                  </div>
                </div>
              </div>
            </div>
          </div>

          {/* Individual Fixed Storage Drives Section */}
          <div className="system-drives-section" role="region" aria-label="Drive Volumes Detail">
            <h3 className="system-section-heading">Fixed Volume Details</h3>
            <div className="system-drives-grid">
              {metrics.storage.drives.map((drive) => {
                const isLowSpace = drive.usedPercent >= 90;
                return (
                  <div
                    key={drive.rootPath}
                    className={`system-drive-card ${isLowSpace ? 'drive-warning' : ''}`}
                    tabIndex={0}
                    role="article"
                    aria-label={`Drive ${drive.rootPath} ${drive.volumeName}`}
                  >
                    <div className="system-drive-header">
                      <div className="drive-title-group">
                        <span className="drive-root">{drive.rootPath}</span>
                        <span className="drive-name">({drive.volumeName})</span>
                      </div>
                      <div className="drive-badges">
                        <span className="system-badge fs-badge">{drive.fileSystem}</span>
                        {isLowSpace && (
                          <span className="system-badge low-space-badge">Low Space</span>
                        )}
                      </div>
                    </div>

                    <div className="system-drive-body">
                      <div className="drive-capacity-row">
                        <span className="drive-used">{formatBytes(drive.usedBytes)} used</span>
                        <span className="drive-total">of {formatBytes(drive.totalBytes)}</span>
                      </div>

                      <div
                        className="system-progress-track"
                        role="progressbar"
                        aria-valuenow={drive.usedPercent}
                        aria-valuemin={0}
                        aria-valuemax={100}
                        aria-label={`Drive ${drive.rootPath} capacity load`}
                      >
                        <div
                          className={`system-progress-fill ${getLoadColorClass(drive.usedPercent)}`}
                          style={{ width: `${Math.min(100, Math.max(0, drive.usedPercent))}%` }}
                        />
                      </div>

                      <div className="drive-footer-row">
                        <span className="drive-free">
                          {formatBytes(drive.availableBytes)} free
                        </span>
                        <span className="drive-percent">{drive.usedPercent.toFixed(1)}%</span>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
};
