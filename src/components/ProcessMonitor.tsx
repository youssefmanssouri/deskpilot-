import React, { useState, useEffect, useMemo, useCallback, useRef } from 'react';
import {
  deskpilotApi,
  type ProcessDto,
  type ProcessListResultDto,
  type ProcessQueryStatusDto,
  IpcError,
} from '../lib/ipc';
import { ProcessInspector } from './ProcessInspector';
import './ProcessMonitor.css';

export type SortField = 'name' | 'pid' | 'memory' | 'cpu' | 'status';
export type SortDirection = 'asc' | 'desc';
export type QuickFilter = 'all' | 'highMemory' | 'restricted';

export interface ProcessMonitorProps {
  /** Optional initial process list payload (e.g. for testing or SSR). */
  initialData?: ProcessListResultDto;
  /** Whether to automatically fetch processes on mount. Defaults to true. */
  autoFetch?: boolean;
  /** Optional additional CSS class names. */
  className?: string;
  /** Optional callback fired when a process row is selected. */
  onSelectProcess?: (process: ProcessDto) => void;
  /** Optional polling interval in ms. Defaults to 0 (manual refresh only). */
  pollingIntervalMs?: number;
}

/**
 * Formats physical resident working-set memory bytes into human-readable units.
 * Returns '—' when working set bytes are inaccessible or null.
 */
export function formatWorkingSet(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined) {
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
  return `${gb.toFixed(2)} GB`;
}

/**
 * Formats cumulative CPU execution duration in milliseconds into human-readable units.
 * Returns '—' when CPU duration is inaccessible or null.
 */
export function formatCpuTime(ms: number | null | undefined): string {
  if (ms === null || ms === undefined) {
    return '—';
  }
  if (ms < 1000) {
    return `${ms} ms`;
  }
  const seconds = ms / 1000;
  if (seconds < 60) {
    return `${seconds.toFixed(1)} s`;
  }
  const mins = Math.floor(seconds / 60);
  const remainingSecs = Math.floor(seconds % 60);
  return `${mins}m ${remainingSecs.toString().padStart(2, '0')}s`;
}

/**
 * Formats an ISO-8601 creation timestamp for tabular presentation.
 * Returns '—' when created_at is inaccessible, null, or unparseable.
 */
export function formatCreatedAt(iso: string | null | undefined): string {
  if (!iso || !iso.trim()) {
    return '—';
  }
  const d = new Date(iso);
  if (isNaN(d.getTime())) {
    return '—';
  }
  return iso.replace('T', ' ').replace('.000Z', ' UTC').replace('Z', ' UTC');
}

/**
 * Filters processes by quick-filter category.
 * - 'all': Retains all processes.
 * - 'highMemory': Strictly greater than 100 MiB (104,857,600 bytes). Nulls never qualify.
 * - 'restricted': Status is 'handleUnavailable'.
 */
export function applyQuickFilter(processes: ProcessDto[], filter: QuickFilter): ProcessDto[] {
  if (filter === 'all') {
    return processes;
  }
  if (filter === 'highMemory') {
    const threshold100Mib = 100 * 1024 * 1024; // 104,857,600 bytes
    return processes.filter(
      (p) => p.workingSetBytes !== null && p.workingSetBytes !== undefined && p.workingSetBytes > threshold100Mib
    );
  }
  if (filter === 'restricted') {
    return processes.filter((p) => p.status === 'handleUnavailable');
  }
  return processes;
}

/**
 * Pure search filtering over processes.
 * Matches case-insensitively across process name, numeric PID, and executable path.
 */
export function filterProcesses(processes: ProcessDto[], query: string): ProcessDto[] {
  if (!query || !query.trim()) {
    return processes;
  }
  const q = query.trim().toLowerCase();
  return processes.filter((proc) => {
    if (proc.name.toLowerCase().includes(q)) return true;
    if (proc.pid.toString().includes(q)) return true;
    if (proc.executablePath && proc.executablePath.toLowerCase().includes(q)) return true;
    return false;
  });
}

/**
 * Deterministic multi-attribute sorting for process rows.
 * Preserves deterministic order via secondary key (name, pid).
 */
export function sortProcesses(
  processes: ProcessDto[],
  field: SortField,
  direction: 'asc' | 'desc'
): ProcessDto[] {
  const sorted = [...processes];
  const mult = direction === 'asc' ? 1 : -1;

  sorted.sort((a, b) => {
    let cmp = 0;
    switch (field) {
      case 'name':
        cmp = a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
        break;
      case 'pid':
        cmp = a.pid - b.pid;
        break;
      case 'memory': {
        const memA = a.workingSetBytes ?? -1;
        const memB = b.workingSetBytes ?? -1;
        cmp = memA - memB;
        break;
      }
      case 'cpu': {
        const cpuA = a.cpuTimeMs ?? -1;
        const cpuB = b.cpuTimeMs ?? -1;
        cmp = cpuA - cpuB;
        break;
      }
      case 'status':
        cmp = a.status.localeCompare(b.status);
        break;
    }

    if (cmp !== 0) {
      return cmp * mult;
    }
    // Secondary deterministic tie-breaker: name ascending, then pid ascending
    const nameTie = a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
    if (nameTie !== 0) return nameTie;
    return a.pid - b.pid;
  });

  return sorted;
}

/**
 * Returns accessible badge label and class for a process query status.
 */
export function getStatusBadge(status: ProcessQueryStatusDto): { label: string; className: string } {
  switch (status) {
    case 'full':
      return { label: 'Full', className: 'status-badge status-badge-full' };
    case 'partial':
      return { label: 'Partial', className: 'status-badge status-badge-partial' };
    case 'handleUnavailable':
      return { label: 'Handle Unavailable', className: 'status-badge status-badge-unavailable' };
    case 'exited':
      return { label: 'Exited', className: 'status-badge status-badge-exited' };
    default:
      return { label: status, className: 'status-badge' };
  }
}

/**
 * Read-Only Windows Process Monitor Component (Precision Command).
 *
 * Implements a Master-Detail inspection workspace:
 * - Master Process Ledger with search, sorting, and quick filters.
 * - Detail Process Inspector displaying detailed telemetry.
 * - Strictly read-only: does not support process termination, suspension, or execution.
 */
export function ProcessMonitor({
  initialData,
  autoFetch = true,
  className = '',
  onSelectProcess,
  pollingIntervalMs = 0,
}: ProcessMonitorProps) {
  const [processes, setProcesses] = useState<ProcessDto[]>(initialData?.processes ?? []);
  const [totalProcesses, setTotalProcesses] = useState<number>(initialData?.totalProcesses ?? 0);
  const [capturedAtMs, setCapturedAtMs] = useState<number | null>(initialData?.capturedAtMs ?? null);
  const [isTruncated, setIsTruncated] = useState<boolean>(initialData?.isTruncated ?? false);

  const [isLoading, setIsLoading] = useState<boolean>(autoFetch && initialData === undefined);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [quickFilter, setQuickFilter] = useState<QuickFilter>('all');
  const [hasLoaded, setHasLoaded] = useState<boolean>(initialData !== undefined);

  const [sortField, setSortField] = useState<SortField>('name');
  const [sortDirection, setSortDirection] = useState<SortDirection>('asc');

  const [selectedProcessId, setSelectedProcessId] = useState<string | null>(null);

  const requestIdRef = useRef<number>(0);
  const isMountedRef = useRef<boolean>(true);
  const searchInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  const fetchProcesses = useCallback(async () => {
    const requestId = ++requestIdRef.current;
    setIsLoading(true);
    setErrorMsg(null);

    try {
      const result: ProcessListResultDto = await deskpilotApi.listProcesses();
      if (isMountedRef.current && requestId === requestIdRef.current) {
        setProcesses(result.processes);
        setTotalProcesses(result.totalProcesses);
        setCapturedAtMs(result.capturedAtMs);
        setIsTruncated(result.isTruncated);
        setHasLoaded(true);
      }
    } catch (err) {
      if (isMountedRef.current && requestId === requestIdRef.current) {
        const ipcErr = IpcError.fromResponse(err);
        setErrorMsg(ipcErr.message || 'Failed to enumerate Windows processes.');
      }
    } finally {
      if (isMountedRef.current && requestId === requestIdRef.current) {
        setIsLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    if (autoFetch && initialData === undefined) {
      fetchProcesses();
    }
  }, [autoFetch, initialData, fetchProcesses]);

  // Optional recurring polling interval if configured
  useEffect(() => {
    if (pollingIntervalMs <= 0) return;
    const timer = setInterval(() => {
      fetchProcesses();
    }, pollingIntervalMs);
    return () => clearInterval(timer);
  }, [pollingIntervalMs, fetchProcesses]);

  const handleSort = (field: SortField) => {
    if (sortField === field) {
      setSortDirection((prev) => (prev === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortField(field);
      setSortDirection('asc');
    }
  };

  const handleSearchKeyDown: React.KeyboardEventHandler<HTMLInputElement> = (e) => {
    if (e.key === 'Escape') {
      if (searchQuery.length > 0) {
        e.preventDefault();
        setSearchQuery('');
      }
    }
  };

  const handleSelectProcess = useCallback(
    (proc: ProcessDto) => {
      setSelectedProcessId(proc.id);
      onSelectProcess?.(proc);
    },
    [onSelectProcess]
  );

  // Quick filter counts for badges
  const quickFilterCounts = useMemo(() => {
    const highMemCount = applyQuickFilter(processes, 'highMemory').length;
    const restrictedCount = applyQuickFilter(processes, 'restricted').length;
    return {
      all: processes.length,
      highMemory: highMemCount,
      restricted: restrictedCount,
    };
  }, [processes]);

  // Composition: quickFilter -> searchQuery -> sort
  const quickFiltered = useMemo(() => {
    return applyQuickFilter(processes, quickFilter);
  }, [processes, quickFilter]);

  const searchFiltered = useMemo(() => {
    return filterProcesses(quickFiltered, searchQuery);
  }, [quickFiltered, searchQuery]);

  const sortedProcesses = useMemo(() => {
    return sortProcesses(searchFiltered, sortField, sortDirection);
  }, [searchFiltered, sortField, sortDirection]);

  // Synchronized selected process DTO
  const selectedProcess = useMemo(() => {
    if (!selectedProcessId) return null;
    return processes.find((p) => p.id === selectedProcessId) ?? null;
  }, [processes, selectedProcessId]);

  // Keyboard navigation for process selection (ArrowUp / ArrowDown)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (
        target &&
        (target.tagName === 'INPUT' ||
          target.tagName === 'TEXTAREA' ||
          target.isContentEditable)
      ) {
        return; // Avoid hijacking keyboard events while the user is typing in inputs
      }

      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        if (sortedProcesses.length === 0) return;
        e.preventDefault();

        const currentIndex = sortedProcesses.findIndex((p) => p.id === selectedProcessId);

        if (e.key === 'ArrowDown') {
          let nextIndex = 0;
          if (currentIndex !== -1) {
            nextIndex = Math.min(currentIndex + 1, sortedProcesses.length - 1);
          }
          const nextProc = sortedProcesses[nextIndex];
          setSelectedProcessId(nextProc.id);
          onSelectProcess?.(nextProc);
        } else if (e.key === 'ArrowUp') {
          let prevIndex = sortedProcesses.length - 1;
          if (currentIndex !== -1) {
            prevIndex = Math.max(currentIndex - 1, 0);
          }
          const prevProc = sortedProcesses[prevIndex];
          setSelectedProcessId(prevProc.id);
          onSelectProcess?.(prevProc);
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [sortedProcesses, selectedProcessId, onSelectProcess]);

  const formattedCaptureTime = useMemo(() => {
    if (!capturedAtMs) return null;
    return new Date(capturedAtMs).toLocaleTimeString();
  }, [capturedAtMs]);

  const getSortIndicator = (field: SortField) => {
    if (sortField !== field) return null;
    return (
      <span className="process-sort-indicator" aria-hidden="true">
        {sortDirection === 'asc' ? '▲' : '▼'}
      </span>
    );
  };

  return (
    <div
      className={`process-monitor-container ${className}`.trim()}
      role="region"
      aria-label="Process Monitor"
    >
      {/* Top Toolbar */}
      <div className="process-toolbar">
        <div className="process-search-wrapper">
          <label htmlFor="process-search-input" className="process-search-label">
            Search Processes
          </label>
          <div className="process-search-input-box">
            <svg
              className="process-search-icon"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
            >
              <circle cx="11" cy="11" r="8" />
              <line x1="21" y1="21" x2="16.65" y2="16.65" />
            </svg>
            <input
              ref={searchInputRef}
              id="process-search-input"
              className="process-search-input"
              type="search"
              placeholder="Filter by process name, PID, or executable path..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              onKeyDown={handleSearchKeyDown}
              disabled={isLoading && !hasLoaded}
              aria-label="Filter processes"
            />
          </div>
        </div>

        <div className="process-toolbar-actions">
          <button
            type="button"
            className="process-refresh-button"
            onClick={fetchProcesses}
            disabled={isLoading}
            aria-label="Refresh process list"
          >
            <svg
              className={`process-refresh-icon ${isLoading ? 'spinning' : ''}`}
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
            >
              <path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67" />
            </svg>
            <span>{isLoading ? 'Scanning...' : 'Refresh Processes'}</span>
          </button>
        </div>
      </div>

      {/* Quick Filter Chips */}
      <div className="process-quick-filters" role="group" aria-label="Process category quick filters">
        <button
          type="button"
          className={`process-filter-chip ${quickFilter === 'all' ? 'active' : ''}`}
          onClick={() => setQuickFilter('all')}
          aria-pressed={quickFilter === 'all'}
        >
          All Processes
          <span className="process-filter-badge">{quickFilterCounts.all}</span>
        </button>
        <button
          type="button"
          className={`process-filter-chip ${quickFilter === 'highMemory' ? 'active' : ''}`}
          onClick={() => setQuickFilter('highMemory')}
          aria-pressed={quickFilter === 'highMemory'}
          title="Processes using strictly greater than 100 MiB of working set memory"
        >
          High Memory (&gt; 100 MiB)
          <span className="process-filter-badge">{quickFilterCounts.highMemory}</span>
        </button>
        <button
          type="button"
          className={`process-filter-chip ${quickFilter === 'restricted' ? 'active' : ''}`}
          onClick={() => setQuickFilter('restricted')}
          aria-pressed={quickFilter === 'restricted'}
          title="Protected system services with handleUnavailable status"
        >
          Restricted Handles
          <span className="process-filter-badge">{quickFilterCounts.restricted}</span>
        </button>
      </div>

      {/* Error Banner */}
      {errorMsg && (
        <div className="process-error-banner" role="alert">
          <div className="process-error-content">
            <strong>Process Query Error:</strong> {errorMsg}
          </div>
          <button
            type="button"
            className="process-retry-button"
            onClick={fetchProcesses}
            aria-label="Retry process enumeration"
          >
            Retry
          </button>
        </div>
      )}

      {/* Truncation Notice */}
      {isTruncated && (
        <div className="process-truncation-banner" role="status">
          <strong>Notice:</strong> Process list truncated at 2,000 entries (total active:{' '}
          {totalProcesses.toLocaleString()}). Retaining top candidates by name and PID.
        </div>
      )}

      {/* Telemetry Summary Bar */}
      {hasLoaded && !errorMsg && (
        <div className="process-summary-bar" role="region" aria-label="Process summary metrics">
          <div className="process-summary-item">
            Retained Processes: <strong>{processes.length.toLocaleString()}</strong>
          </div>
          <div className="process-summary-item">
            Total Observed: <strong>{totalProcesses.toLocaleString()}</strong>
          </div>
          {formattedCaptureTime && (
            <div className="process-summary-item">
              Snapshot Captured: <strong>{formattedCaptureTime}</strong>
            </div>
          )}
          {(searchQuery.trim() || quickFilter !== 'all') && (
            <div className="process-summary-item">
              Filtered Matches: <strong>{sortedProcesses.length.toLocaleString()}</strong>
            </div>
          )}
        </div>
      )}

      {/* Main Master-Detail Workspace */}
      <div className="process-master-detail-workspace">
        {/* Master Ledger Column */}
        <div className="process-ledger-column">
          {isLoading && !hasLoaded ? (
            <div className="process-loading-state" role="status">
              <div className="process-spinner" aria-hidden="true" />
              <p>Scanning active processes...</p>
            </div>
          ) : processes.length === 0 && hasLoaded && !errorMsg ? (
            <div className="process-empty-state" role="status">
              <p className="process-empty-title">No processes discovered.</p>
              <p>Try refreshing the process list.</p>
            </div>
          ) : sortedProcesses.length === 0 && processes.length > 0 ? (
            <div className="process-empty-state" role="status">
              <p className="process-empty-title">No processes match your filter.</p>
              <p>Clear or adjust your search query or quick filter.</p>
            </div>
          ) : (
            <div className="process-table-container">
              <table className="process-table" aria-label="Running processes table">
                <thead>
                  <tr>
                    <th
                      className="sortable"
                      onClick={() => handleSort('name')}
                      onKeyDown={(e) => e.key === 'Enter' && handleSort('name')}
                      tabIndex={0}
                      role="columnheader"
                      aria-sort={
                        sortField === 'name' ? (sortDirection === 'asc' ? 'ascending' : 'descending') : 'none'
                      }
                    >
                      Process {getSortIndicator('name')}
                    </th>
                    <th
                      className="sortable align-right"
                      onClick={() => handleSort('pid')}
                      onKeyDown={(e) => e.key === 'Enter' && handleSort('pid')}
                      tabIndex={0}
                      role="columnheader"
                      aria-sort={
                        sortField === 'pid' ? (sortDirection === 'asc' ? 'ascending' : 'descending') : 'none'
                      }
                    >
                      PID {getSortIndicator('pid')}
                    </th>
                    <th
                      className="sortable"
                      onClick={() => handleSort('status')}
                      onKeyDown={(e) => e.key === 'Enter' && handleSort('status')}
                      tabIndex={0}
                      role="columnheader"
                      aria-sort={
                        sortField === 'status' ? (sortDirection === 'asc' ? 'ascending' : 'descending') : 'none'
                      }
                    >
                      Status {getSortIndicator('status')}
                    </th>
                    <th
                      className="sortable align-right"
                      onClick={() => handleSort('memory')}
                      onKeyDown={(e) => e.key === 'Enter' && handleSort('memory')}
                      tabIndex={0}
                      role="columnheader"
                      aria-sort={
                        sortField === 'memory' ? (sortDirection === 'asc' ? 'ascending' : 'descending') : 'none'
                      }
                    >
                      Memory {getSortIndicator('memory')}
                    </th>
                    <th
                      className="sortable align-right"
                      onClick={() => handleSort('cpu')}
                      onKeyDown={(e) => e.key === 'Enter' && handleSort('cpu')}
                      tabIndex={0}
                      role="columnheader"
                      aria-sort={
                        sortField === 'cpu' ? (sortDirection === 'asc' ? 'ascending' : 'descending') : 'none'
                      }
                    >
                      CPU Time {getSortIndicator('cpu')}
                    </th>
                    <th role="columnheader">Created</th>
                    <th role="columnheader">Executable Path</th>
                  </tr>
                </thead>
                <tbody>
                  {sortedProcesses.map((proc) => {
                    const statusBadge = getStatusBadge(proc.status);
                    const isSelected = proc.id === selectedProcessId;

                    return (
                      <tr
                        key={proc.id}
                        data-testid={`process-row-${proc.id}`}
                        className={`process-row ${isSelected ? 'process-row-selected' : ''}`}
                        aria-selected={isSelected}
                        tabIndex={0}
                        onClick={() => handleSelectProcess(proc)}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter' || e.key === ' ') {
                            e.preventDefault();
                            handleSelectProcess(proc);
                          }
                        }}
                      >
                        <td className="process-name-cell">
                          <span className="process-name-text">{proc.name}</span>
                        </td>
                        <td className="process-mono-cell align-right">{proc.pid}</td>
                        <td>
                          <span className={statusBadge.className}>{statusBadge.label}</span>
                        </td>
                        <td className="process-mono-cell align-right">
                          {formatWorkingSet(proc.workingSetBytes)}
                        </td>
                        <td className="process-mono-cell align-right">
                          {formatCpuTime(proc.cpuTimeMs)}
                        </td>
                        <td className="process-mono-cell">{formatCreatedAt(proc.createdAt)}</td>
                        <td
                          className="process-path-cell"
                          title={proc.executablePath ?? 'Path unavailable or restricted by operating system'}
                        >
                          {proc.executablePath ?? '—'}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </div>

        {/* Detail Inspector Column */}
        <div className="process-inspector-column">
          <ProcessInspector
            process={selectedProcess}
            onClose={() => setSelectedProcessId(null)}
          />
        </div>
      </div>
    </div>
  );
}
