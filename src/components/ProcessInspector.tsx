import { useState, useCallback } from 'react';
import type { ProcessDto } from '../lib/ipc';
import {
  formatWorkingSet,
  formatCpuTime,
  formatCreatedAt,
  getStatusBadge,
} from './ProcessMonitor';
import './ProcessInspector.css';

export interface ProcessInspectorProps {
  /** The selected process DTO to inspect, or null if no process is selected. */
  process: ProcessDto | null;
  /** Optional callback to close or clear the inspector selection. */
  onClose?: () => void;
  /** Optional additional CSS class names. */
  className?: string;
}

/**
 * Returns a human-readable security and diagnostic explanation of the process status.
 */
export function getStatusExplanation(status: ProcessDto['status']): string {
  switch (status) {
    case 'full':
      return 'DeskPilot can read all of the details below.';
    case 'partial':
      return 'Windows only shares some details for this process, so a few fields may be empty.';
    case 'handleUnavailable':
      return "Windows doesn't let normal apps read this process. That's expected for system services and apps running as administrator.";
    case 'exited':
      return 'This process ended while DeskPilot was reading the list.';
    default:
      return 'Windows reported a status DeskPilot does not recognise.';
  }
}

/**
 * ProcessInspector renders detailed, read-only diagnostic telemetry for a selected Windows process.
 *
 * Adheres strictly to DeskPilot's least-privilege security model:
 * - Diagnostic inspection only.
 * - Strictly read-only: does not expose or support process termination, suspension, memory alteration, or priority modification.
 * - Accurately labels cumulative CPU execution duration without fabricating instantaneous CPU percentages.
 */
export function ProcessInspector({
  process,
  onClose,
  className = '',
}: ProcessInspectorProps) {
  const [copied, setCopied] = useState<boolean>(false);

  const handleCopyPath = useCallback(async () => {
    if (!process?.executablePath) return;

    if (
      typeof navigator !== 'undefined' &&
      navigator.clipboard &&
      typeof navigator.clipboard.writeText === 'function'
    ) {
      try {
        await navigator.clipboard.writeText(process.executablePath);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      } catch {
        // Ignore clipboard failure in restricted webview contexts
      }
    }
  }, [process?.executablePath]);

  if (!process) {
    return (
      <aside
        className={`process-inspector ${className}`.trim()}
        aria-label="Process details"
        data-testid="process-inspector-empty"
      >
        <div className="process-inspector-header">
          <h3 className="process-inspector-title">Details</h3>
        </div>
        <div className="process-inspector-body">
          <div className="process-inspector-empty">
            <svg
              className="process-inspector-empty-icon"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
            >
              <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
              <line x1="8" y1="21" x2="16" y2="21" />
              <line x1="12" y1="17" x2="12" y2="21" />
            </svg>
            <h4 className="process-inspector-empty-title">No process selected</h4>
            <p className="process-inspector-empty-text">
              Select a process in the list to see its memory use, CPU time and location.
            </p>
          </div>
        </div>
      </aside>
    );
  }

  const statusBadge = getStatusBadge(process.status);
  const statusExplanation = getStatusExplanation(process.status);
  const hasClipboardSupport =
    typeof navigator !== 'undefined' &&
    Boolean(navigator.clipboard && typeof navigator.clipboard.writeText === 'function');

  return (
    <aside
      className={`process-inspector ${className}`.trim()}
      aria-label={`Details for ${process.name} (PID ${process.pid})`}
      data-testid="process-inspector"
    >
      <div className="process-inspector-header">
        <div className="process-inspector-header-left">
          <h3 className="process-inspector-title">Details</h3>
          <span className={`process-inspector-badge ${statusBadge.className}`} role="status">
            {statusBadge.label}
          </span>
        </div>
        {onClose && (
          <button
            type="button"
            className="process-inspector-close-btn"
            onClick={onClose}
            aria-label="Close process inspector"
          >
            ×
          </button>
        )}
      </div>

      <div className="process-inspector-body">
        {/* Process Hero Identification */}
        <div className="process-inspector-hero">
          <div className="process-inspector-icon" aria-hidden="true">
            <svg
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <rect x="4" y="4" width="16" height="16" rx="2" ry="2" />
              <rect x="9" y="9" width="6" height="6" />
              <line x1="9" y1="1" x2="9" y2="4" />
              <line x1="15" y1="1" x2="15" y2="4" />
              <line x1="9" y1="20" x2="9" y2="23" />
              <line x1="15" y1="20" x2="15" y2="23" />
              <line x1="20" y1="9" x2="23" y2="9" />
              <line x1="20" y1="14" x2="23" y2="14" />
              <line x1="1" y1="9" x2="4" y2="9" />
              <line x1="1" y1="14" x2="4" y2="14" />
            </svg>
          </div>
          <div className="process-inspector-hero-content">
            <h4 className="process-inspector-name">{process.name}</h4>
            <div className="process-inspector-id-pills">
              <span className="process-id-pill" title="Operating System Process ID">
                PID: <strong>{process.pid}</strong>
              </span>
              <span className="process-id-pill" title="Parent Process ID">
                PPID: <strong>{process.parentPid}</strong>
              </span>
              {process.sessionId !== null && (
                <span className="process-id-pill" title="Windows Terminal Services Session ID">
                  Session: <strong>{process.sessionId}</strong>
                </span>
              )}
            </div>
          </div>
        </div>

        {/* Status Explanation Card */}
        <div className="process-inspector-section">
          <h5 className="process-inspector-section-title">Query Status & Access Level</h5>
          <div className="process-inspector-status-box">
            <p className="process-status-desc">{statusExplanation}</p>
          </div>
        </div>

        {/* Primary Telemetry Grid */}
        <div className="process-inspector-section">
          <h5 className="process-inspector-section-title">Telemetry Ledger</h5>
          <div className="process-inspector-grid">
            <div className="process-metric-tile">
              <span className="process-metric-tile-label">Working Set Memory</span>
              <span className="process-metric-tile-value" data-testid="inspector-memory-formatted">
                {formatWorkingSet(process.workingSetBytes)}
              </span>
              <span className="process-metric-tile-detail" data-testid="inspector-memory-bytes">
                {process.workingSetBytes !== null && process.workingSetBytes !== undefined
                  ? `${process.workingSetBytes.toLocaleString()} bytes`
                  : 'Not reported by OS'}
              </span>
            </div>

            <div className="process-metric-tile">
              <span className="process-metric-tile-label">Total CPU time</span>
              <span className="process-metric-tile-value" data-testid="inspector-cpu-formatted">
                {formatCpuTime(process.cpuTimeMs)}
              </span>
              <span className="process-metric-tile-detail" data-testid="inspector-cpu-ms">
                {process.cpuTimeMs !== null && process.cpuTimeMs !== undefined
                  ? `${process.cpuTimeMs.toLocaleString()} ms`
                  : 'Not reported by OS'}
              </span>
            </div>
          </div>
        </div>

        {/* Metadata Details List */}
        <div className="process-inspector-section">
          <h5 className="process-inspector-section-title">Process Attributes</h5>
          <dl className="process-inspector-details-list">
            <div className="process-detail-item">
              <dt>Created Timestamp</dt>
              <dd className="process-mono-text" data-testid="inspector-created-at">
                {formatCreatedAt(process.createdAt)}
              </dd>
            </div>
            <div className="process-detail-item">
              <dt>Internal Identity Token</dt>
              <dd className="process-mono-text process-id-token" title={process.id} data-testid="inspector-id">
                {process.id}
              </dd>
            </div>
          </dl>
        </div>

        {/* Executable Path Section */}
        <div className="process-inspector-section">
          <div className="process-path-header">
            <h5 className="process-inspector-section-title">Executable Binary Path</h5>
            {process.executablePath && hasClipboardSupport && (
              <button
                type="button"
                className="process-copy-path-btn"
                onClick={handleCopyPath}
                aria-label="Copy executable path to clipboard"
                data-testid="copy-path-button"
              >
                {copied ? '✓ Copied' : 'Copy Path'}
              </button>
            )}
          </div>
          <div className="process-path-box">
            {process.executablePath ? (
              <code className="process-path-code" data-testid="inspector-executable-path">
                {process.executablePath}
              </code>
            ) : (
              <span className="process-path-unavailable" data-testid="inspector-executable-path-empty">
                Path restricted by operating system security policies
              </span>
            )}
          </div>
        </div>

        {/* Restrained Read-Only Security Disclosure */}
        <div className="process-inspector-disclosure" role="note" aria-label="Security Disclosure">
          <div className="process-disclosure-badge">
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
              <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
            </svg>
            <span>Read-Only Telemetry • Diagnostic Mode</span>
          </div>
          <p className="process-disclosure-text">
            Operating system process boundary is strictly non-invasive. Process termination, suspension, and injection are disabled.
          </p>
        </div>
      </div>
    </aside>
  );
}
