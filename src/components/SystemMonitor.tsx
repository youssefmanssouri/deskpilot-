import React from 'react';
import {
  type SystemMetricsDto,
} from '../lib/ipc';
import { CpuSparkline } from './CpuSparkline';
import {
  useSystemMetrics,
  useSharedSystemMetrics,
  type RefreshMode,
} from './useSystemMetrics';
import './SystemMonitor.css';

export interface SystemMonitorProps {
  initialData?: SystemMetricsDto;
  autoFetch?: boolean;
  className?: string;
  defaultRefreshMode?: RefreshMode;
}

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

/**
 * SystemMonitor renders the System resources view.
 *
 * Guarantees:
 * - Precision Command design: high information density, factual data, no fabricated metrics.
 * - Single shared polling source across application shell with unmount and hidden-window guards.
 * - Bounded 30-sample historical CPU sparkline visualization.
 * - Clean fixed-disk storage tabular ledger and dual-uptime accounting.
 */
export const SystemMonitor: React.FC<SystemMonitorProps> = ({
  initialData,
  autoFetch = true,
  defaultRefreshMode = 'manual',
  className = '',
}) => {
  const shared = useSharedSystemMetrics();
  const local = useSystemMetrics({
    initialData,
    autoFetch: autoFetch && !shared && initialData === undefined,
    defaultRefreshMode,
  });

  // Prefer shared polling instance when available in app tree, unless local initialData is explicitly passed
  const {
    metrics,
    cpuHistory,
    isLoading,
    errorMsg,
    refreshMode,
    setRefreshMode,
    isPaused,
    lastUpdated,
    refresh,
  } = initialData !== undefined || !shared ? local : shared;

  return (
    <div
      className={`system-monitor-container ${className}`.trim()}
      role="region"
      aria-label="System resources"
    >
      {/* Top Toolbar */}
      <div className="system-toolbar">
        <div className="system-toolbar-info">
          <div className="system-subtitle-row">
            {lastUpdated && (
              <span className="system-timestamp">
                Updated {lastUpdated.toLocaleTimeString()}
              </span>
            )}
            {isPaused && refreshMode !== 'manual' && (
              <span className="system-badge paused-badge" role="status">
                Updates paused while DeskPilot is in the background
              </span>
            )}
          </div>
        </div>

        <div className="system-toolbar-actions">
          <div className="system-refresh-mode-wrapper">
            <label htmlFor="system-refresh-mode" className="system-mode-label">
              Update every
            </label>
            <select
              id="system-refresh-mode"
              className="system-mode-select"
              value={refreshMode}
              onChange={(e) => setRefreshMode(e.target.value as RefreshMode)}
              aria-label="Update interval"
            >
              <option value="manual">Only when I refresh</option>
              <option value="3s">3 seconds</option>
              <option value="5s">5 seconds</option>
            </select>
          </div>

          <button
            type="button"
            className="system-refresh-button"
            onClick={() => refresh(true)}
            disabled={isLoading}
            aria-label="Refresh system metrics"
          >
            {isLoading ? (
              <>
                <span className="spinner-icon" aria-hidden="true" />
                <span>Updating…</span>
              </>
            ) : (
              <>
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  aria-hidden="true"
                >
                  <path d="M21 12a9 9 0 1 1-3-6.7L21 8" />
                  <path d="M21 3v5h-5" />
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
            onClick={() => refresh(true)}
            aria-label="Retry loading metrics"
          >
            Retry
          </button>
        </div>
      )}

      {/* Main Metrics Content */}
      {isLoading && !metrics ? (
        <div className="system-loading-state" role="status">
          <div className="system-loading-spinner" aria-hidden="true" />
          <p>Sampling system metrics from Windows...</p>
        </div>
      ) : metrics ? (
        <div className="system-content-wrapper">
          {/* Top Row: CPU Utilization + Physical Memory (RAM) */}
          <div className="system-ledger-primary-grid">
            {/* CPU Card */}
            <section className="system-card" role="region" aria-label="Processor (CPU) Metrics">
              <div className="system-card-header">
                <span className="system-card-title">Processor</span>
                <span className="system-badge arch-badge">{metrics.cpu.architecture}</span>
              </div>
              <div className="system-card-body">
                <div className="system-card-hero-row">
                  <div className="system-metric-hero-block">
                    <span className="system-metric-hero">
                      {metrics.cpu.utilizationPercent.toFixed(1)}%
                    </span>
                    <span className="system-metric-sub">
                      {metrics.cpu.logicalCores} logical cores
                    </span>
                  </div>
                  <CpuSparkline samples={cpuHistory} />
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
                    style={{
                      width: `${Math.min(100, Math.max(0, metrics.cpu.utilizationPercent))}%`,
                    }}
                  />
                </div>

                <p className="system-hardware-desc" title={metrics.cpu.brandName}>
                  {metrics.cpu.brandName}
                </p>
              </div>
            </section>

            {/* RAM Card */}
            <section className="system-card" role="region" aria-label="Physical RAM Metrics">
              <div className="system-card-header">
                <span className="system-card-title">Memory</span>
                <span className="system-badge mem-badge">
                  {metrics.memory.usedPercent.toFixed(1)}%
                </span>
              </div>
              <div className="system-card-body">
                <div className="system-metric-hero-block">
                  <span className="system-metric-hero">
                    {formatBytes(metrics.memory.usedBytes)} in use
                  </span>
                  <span className="system-metric-sub">
                    of {formatBytes(metrics.memory.totalBytes)} ·{' '}
                    {formatBytes(metrics.memory.availableBytes)} free
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
                    style={{
                      width: `${Math.min(100, Math.max(0, metrics.memory.usedPercent))}%`,
                    }}
                  />
                </div>

                <div className="system-capacity-scale" aria-hidden="true">
                  <span>0%</span>
                  <span>25%</span>
                  <span>50%</span>
                  <span>75%</span>
                  <span>100%</span>
                </div>

                <p className="system-hardware-desc">
                  {formatBytes(metrics.memory.availableBytes)} available for processes
                </p>
              </div>
            </section>
          </div>

          {/* Fixed Disk Storage Ledger Table */}
          <section
            className="system-storage-ledger-section"
            role="region"
            aria-label="Drives"
          >
            <div className="system-section-header">
              <div className="system-section-title-group">
                <h3 className="system-section-heading">Drives</h3>
                <span className="system-badge storage-badge">
                  {metrics.storage.drives.length} {metrics.storage.drives.length === 1 ? 'drive' : 'drives'}
                </span>
              </div>
              <span className="system-section-sub">
                {(() => {
                  const totalCapacity = metrics.storage.drives.reduce(
                    (acc, d) => acc + d.totalBytes,
                    0
                  );
                  const totalUsed = metrics.storage.drives.reduce(
                    (acc, d) => acc + d.usedBytes,
                    0
                  );
                  return `${formatBytes(totalUsed)} used across all volumes (${formatBytes(
                    totalCapacity
                  )} total)`;
                })()}
              </span>
            </div>

            {metrics.storage.drives.length === 0 ? (
              <div className="system-storage-empty" role="status">
                <p>No drives found.</p>
              </div>
            ) : (
              <div className="system-table-responsive-wrapper">
                <table className="system-storage-table" aria-label="Drive Volumes Detail">
                  <thead>
                    <tr>
                      <th scope="col">Drive</th>
                      <th scope="col">Format</th>
                      <th scope="col">Used</th>
                      <th scope="col">Capacity</th>
                      <th scope="col">Available</th>
                      <th scope="col">% Used</th>
                    </tr>
                  </thead>
                  <tbody>
                    {metrics.storage.drives.map((drive) => {
                      const isLowSpace = drive.usedPercent >= 90;
                      return (
                        <tr
                          key={drive.rootPath}
                          className={isLowSpace ? 'drive-row-warning' : ''}
                        >
                          <td className="drive-cell-name">
                            <span className="drive-root">{drive.rootPath}</span>
                            <span className="drive-name">({drive.volumeName})</span>
                            {isLowSpace && (
                              <span className="system-badge low-space-badge">Low Space</span>
                            )}
                          </td>
                          <td className="drive-cell-fs">
                            <span className="system-badge fs-badge">{drive.fileSystem}</span>
                          </td>
                          <td className="drive-cell-used">{formatBytes(drive.usedBytes)}</td>
                          <td className="drive-cell-total">{formatBytes(drive.totalBytes)}</td>
                          <td className="drive-cell-avail">{formatBytes(drive.availableBytes)}</td>
                          <td className="drive-cell-percent">
                            <div className="drive-percent-wrapper">
                              <div
                                className="system-progress-track drive-mini-track"
                                role="progressbar"
                                aria-valuenow={drive.usedPercent}
                                aria-valuemin={0}
                                aria-valuemax={100}
                                aria-label={`Drive ${drive.rootPath} capacity`}
                              >
                                <div
                                  className={`system-progress-fill ${getLoadColorClass(
                                    drive.usedPercent
                                  )}`}
                                  style={{
                                    width: `${Math.min(
                                      100,
                                      Math.max(0, drive.usedPercent)
                                    )}%`,
                                  }}
                                />
                              </div>
                              <span className="drive-percent-val">
                                {drive.usedPercent.toFixed(1)}%
                              </span>
                            </div>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </section>

          {/* System Runtime & Telemetry Ledger */}
          <section
            className="system-runtime-ledger-section"
            role="region"
            aria-label="Windows and DeskPilot uptime"
          >
            <div className="system-card">
              <div className="system-card-header">
                <span className="system-card-title">Uptime</span>
                <span className="system-badge os-badge">{metrics.system.osName}</span>
              </div>
              <div className="system-card-body">
                <div className="system-runtime-grid">
                  <div className="runtime-metric-tile">
                    <span className="runtime-label">PC running for</span>
                    <strong className="runtime-hero">
                      {formatUptime(metrics.system.uptimeSeconds)}
                    </strong>
                    <span className="runtime-caption">Since Windows last started</span>
                  </div>

                  <div className="runtime-metric-tile">
                    <span className="runtime-label">DeskPilot running for</span>
                    <strong className="runtime-hero">
                      {formatUptime(metrics.system.appUptimeSeconds)}
                    </strong>
                    <span className="runtime-caption">Since DeskPilot started</span>
                  </div>

                  <div className="runtime-metric-tile">
                    <span className="runtime-label">Architecture</span>
                    <strong className="runtime-val">{metrics.system.architecture}</strong>
                    <span className="runtime-caption">Processor architecture</span>
                  </div>

                  <div className="runtime-metric-tile">
                    <span className="runtime-label">Last updated</span>
                    <strong className="runtime-val">
                      {new Date(metrics.sampledAtMs).toLocaleTimeString()}
                    </strong>
                    <span className="runtime-caption">Time of the latest reading</span>
                  </div>
                </div>
              </div>
            </div>
          </section>
        </div>
      ) : null}
    </div>
  );
};
