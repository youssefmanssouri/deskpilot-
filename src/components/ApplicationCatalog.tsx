import { useState, useEffect, useMemo, useCallback, useRef } from 'react';
import {
  deskpilotApi,
  type ApplicationDto,
  type DiscoveryResultDto,
  IpcError,
} from '../lib/ipc';
import { ApplicationCard } from './ApplicationCard';
import { ApplicationInspector } from './ApplicationInspector';
import { ProcessMonitor } from './ProcessMonitor';
import { SystemMonitor } from './SystemMonitor';
import { useApplicationLauncher } from './useApplicationLauncher';
import { canLaunchWithKeyboard } from './launchPolicy';
import './ApplicationCatalog.css';

export type CatalogView = 'applications' | 'processes' | 'system';

export type SourceFilterId = 'all' | 'hklm-64' | 'hklm-32' | 'hkcu' | 'start-menu';

export interface SourceFilterDef {
  id: SourceFilterId;
  label: string;
}

export const SOURCE_FILTERS: SourceFilterDef[] = [
  { id: 'all', label: 'All' },
  { id: 'hklm-64', label: 'HKLM 64-bit' },
  { id: 'hklm-32', label: 'HKLM 32-bit' },
  { id: 'hkcu', label: 'HKCU User' },
  { id: 'start-menu', label: 'Start Menu' },
];

export interface ApplicationCatalogProps {
  initialApplications?: ApplicationDto[];
  initialScannedSources?: string[];
  initialPartialFailures?: string[];
  autoDiscover?: boolean;
  defaultView?: CatalogView;
  activeView?: CatalogView;
  onViewChange?: (view: CatalogView) => void;
  onLaunchSuccess?: (applicationId: string, processId: number) => void;
  onLaunchError?: (applicationId: string, error: IpcError) => void;
  className?: string;
}

export interface AvailabilityCounts {
  total: number;
  available: number;
  unverified: number;
  incomplete: number;
  missing: number;
}

/**
 * Pure, deterministic source-filter predicate.
 * Matches application discovery sources against selected filter pill.
 */
export function matchesSourceFilter(app: ApplicationDto, filter: SourceFilterId): boolean {
  if (filter === 'all') return true;
  if (!app.sources || app.sources.length === 0) return false;
  return app.sources.some((src) => {
    const s = src.toLowerCase();
    switch (filter) {
      case 'hklm-64':
        return s.includes('hklm 64') || (s.includes('hklm') && s.includes('64'));
      case 'hklm-32':
        return s.includes('hklm 32') || (s.includes('hklm') && s.includes('32'));
      case 'hkcu':
        return s.includes('hkcu');
      case 'start-menu':
        return s.includes('start menu');
      default:
        return true;
    }
  });
}

/**
 * Pure, deterministic search filter and ranker for applications.
 *
 * Ranking precedence:
 * 1. Exact display-name match
 * 2. Display-name prefix match
 * 3. Other display-name substring match
 * 4. Publisher or secondary metadata match
 *
 * Does not invoke backend IPC, filesystem queries, or privileged APIs.
 */
export function filterApplications(
  applications: ApplicationDto[],
  searchQuery: string
): ApplicationDto[] {
  if (!searchQuery || !searchQuery.trim()) {
    return applications;
  }
  const q = searchQuery.trim().toLowerCase();

  type RankedApp = {
    app: ApplicationDto;
    rank: number;
    originalIndex: number;
  };

  const ranked: RankedApp[] = [];

  for (let i = 0; i < applications.length; i++) {
    const app = applications[i];
    const nameLower = app.displayName.toLowerCase();
    const pubLower = app.publisher ? app.publisher.toLowerCase() : '';

    let rank = -1;
    if (nameLower === q) {
      rank = 1;
    } else if (nameLower.startsWith(q)) {
      rank = 2;
    } else if (nameLower.includes(q)) {
      rank = 3;
    } else if (pubLower.includes(q)) {
      rank = 4;
    }

    if (rank !== -1) {
      ranked.push({ app, rank, originalIndex: i });
    }
  }

  ranked.sort((a, b) => {
    if (a.rank !== b.rank) {
      return a.rank - b.rank;
    }
    return a.originalIndex - b.originalIndex;
  });

  return ranked.map((r) => r.app);
}

/**
 * Computes availability aggregate counts for the summary bar.
 */
export function computeAvailabilityCounts(applications: ApplicationDto[]): AvailabilityCounts {
  return applications.reduce<AvailabilityCounts>(
    (acc, app) => {
      acc.total++;
      switch (app.availability) {
        case 'Available':
          acc.available++;
          break;
        case 'Unverified':
          acc.unverified++;
          break;
        case 'IncompleteMetadata':
          acc.incomplete++;
          break;
        case 'MissingTarget':
          acc.missing++;
          break;
      }
      return acc;
    },
    { total: 0, available: 0, unverified: 0, incomplete: 0, missing: 0 }
  );
}

/**
 * ApplicationCatalog coordinates application discovery, search filtering,
 * aggregate metrics, master-detail list selection, and ApplicationInspector rendering.
 *
 * Guarantees:
 * - Employs existing discover_applications Tauri IPC command.
 * - Manages explicit visual states: Loading, Empty, Error with Retry, Loaded.
 * - Preserves deterministic 4-tier ranking across source filters.
 * - Synchronizes master selection with inspector metadata ledger and primary launch action.
 * - Passes only safe identity metadata down to child cards and inspector.
 */
export function ApplicationCatalog({
  initialApplications,
  initialScannedSources = [],
  initialPartialFailures = [],
  autoDiscover = true,
  defaultView = 'applications',
  activeView: controlledActiveView,
  onViewChange: _onViewChange,
  onLaunchSuccess,
  onLaunchError,
  className = '',
}: ApplicationCatalogProps) {
  const activeView = controlledActiveView ?? defaultView;
  const [applications, setApplications] = useState<ApplicationDto[]>(
    initialApplications ?? []
  );
  const [scannedSources, setScannedSources] = useState<string[]>(initialScannedSources);
  const [partialFailures, setPartialFailures] = useState<string[]>(initialPartialFailures);
  const [isLoading, setIsLoading] = useState<boolean>(
    autoDiscover && initialApplications === undefined
  );
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [searchFilter, setSearchFilter] = useState<string>('');
  const [activeSourceFilter, setActiveSourceFilter] = useState<SourceFilterId>('all');
  const [selectedAppId, setSelectedAppId] = useState<string | null>(() => {
    return initialApplications && initialApplications.length > 0 ? initialApplications[0].id : null;
  });
  const [hasDiscovered, setHasDiscovered] = useState<boolean>(initialApplications !== undefined);
  const discoveryRequestIdRef = useRef<number>(0);
  const isLoadingRef = useRef<boolean>(false);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const hasAutoFocusedRef = useRef<boolean>(false);
  const cardRefs = useRef<(HTMLElement | null)[]>([]);

  const { activeLaunch, launch } = useApplicationLauncher({
    onSuccess: onLaunchSuccess,
    onError: onLaunchError,
  });

  const fetchDiscovery = useCallback(async () => {
    if (isLoadingRef.current) {
      return;
    }
    isLoadingRef.current = true;
    const requestId = ++discoveryRequestIdRef.current;
    setIsLoading(true);
    setErrorMsg(null);
    try {
      const result: DiscoveryResultDto = await deskpilotApi.discoverApplications();
      if (requestId === discoveryRequestIdRef.current) {
        setApplications(result.applications);
        setScannedSources(result.scannedSources);
        setPartialFailures(result.partialFailures);
        setHasDiscovered(true);
      }
    } catch (err) {
      if (requestId === discoveryRequestIdRef.current) {
        const ipcErr = IpcError.fromResponse(err);
        setErrorMsg(ipcErr.message || 'Applications could not be loaded.');
      }
    } finally {
      if (requestId === discoveryRequestIdRef.current) {
        isLoadingRef.current = false;
        setIsLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    if (autoDiscover && initialApplications === undefined) {
      fetchDiscovery();
    }
  }, [autoDiscover, initialApplications, fetchDiscovery]);

  useEffect(() => {
    if (hasDiscovered && !isLoading && !hasAutoFocusedRef.current) {
      hasAutoFocusedRef.current = true;
      searchInputRef.current?.focus();
    }
  }, [hasDiscovered, isLoading]);

  // Step 1: Filter applications by active source pill
  const sourceFilteredApps = useMemo(() => {
    if (activeSourceFilter === 'all') return applications;
    return applications.filter((app) => matchesSourceFilter(app, activeSourceFilter));
  }, [applications, activeSourceFilter]);

  // Step 2: Apply 4-tier search ranking over source-filtered applications
  const filteredApps = useMemo(() => {
    return filterApplications(sourceFilteredApps, searchFilter);
  }, [sourceFilteredApps, searchFilter]);

  // Selection reconciliation: ensure valid selection whenever filtered list changes
  useEffect(() => {
    if (filteredApps.length === 0) {
      setSelectedAppId(null);
    } else if (!selectedAppId || !filteredApps.some((a) => a.id === selectedAppId)) {
      setSelectedAppId(filteredApps[0].id);
    }
  }, [filteredApps, selectedAppId]);

  const selectedApp = useMemo(() => {
    if (!selectedAppId) return null;
    return filteredApps.find((a) => a.id === selectedAppId) ?? null;
  }, [filteredApps, selectedAppId]);

  const counts = useMemo(() => {
    return computeAvailabilityCounts(applications);
  }, [applications]);

  const handleSearchKeyDown: React.KeyboardEventHandler<HTMLInputElement> = (e) => {
    if (e.key === 'Escape') {
      if (searchFilter.length > 0) {
        e.preventDefault();
        setSearchFilter('');
      }
    } else if (e.key === 'Enter') {
      e.preventDefault();
      // Enter launches exactly the selected application, and only when it is verified.
      // It never falls back to a different application.
      if (selectedApp && canLaunchWithKeyboard(selectedApp)) {
        void launch(selectedApp.id);
      }
    } else if (e.key === 'ArrowDown') {
      if (filteredApps.length > 0) {
        e.preventDefault();
        setSelectedAppId(filteredApps[0].id);
        cardRefs.current[0]?.focus();
      }
    }
  };

  const handleCardKeyDown = (index: number): React.KeyboardEventHandler<HTMLElement> => (e) => {
    if (e.key === 'ArrowDown' || e.key === 'ArrowRight') {
      e.preventDefault();
      if (index + 1 < filteredApps.length) {
        const nextApp = filteredApps[index + 1];
        setSelectedAppId(nextApp.id);
        cardRefs.current[index + 1]?.focus();
      }
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowLeft') {
      e.preventDefault();
      if (index > 0) {
        const prevApp = filteredApps[index - 1];
        setSelectedAppId(prevApp.id);
        cardRefs.current[index - 1]?.focus();
      } else {
        searchInputRef.current?.focus();
      }
    } else if (e.key === 'Home') {
      e.preventDefault();
      if (filteredApps.length > 0) {
        setSelectedAppId(filteredApps[0].id);
        cardRefs.current[0]?.focus();
      }
    } else if (e.key === 'End') {
      e.preventDefault();
      if (filteredApps.length > 0) {
        setSelectedAppId(filteredApps[filteredApps.length - 1].id);
        cardRefs.current[filteredApps.length - 1]?.focus();
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      searchInputRef.current?.focus();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const app = filteredApps[index];
      if (app) {
        setSelectedAppId(app.id);
        if (canLaunchWithKeyboard(app)) {
          void launch(app.id);
        }
      }
    }
  };

  return (
    <div className={`catalog-container ${className}`.trim()}>
      {activeView === 'system' ? (
        <div id="panel-system" role="region" aria-label="System">
          <SystemMonitor />
        </div>
      ) : activeView === 'processes' ? (
        <div id="panel-processes" role="region" aria-label="Processes">
          <ProcessMonitor />
        </div>
      ) : (
        <div id="panel-applications" role="region" aria-label="Applications" className="catalog-tab-content">
          <div className="catalog-toolbar">
            <div className="catalog-search-wrapper">
              <label htmlFor="catalog-search-input" className="catalog-search-label">
                Search Applications
              </label>
              <input
                ref={searchInputRef}
                id="catalog-search-input"
                className="catalog-search-input"
                type="search"
                placeholder={
                  applications.length > 0
                    ? `Filter ${applications.length} discovered applications... (Ctrl+K)`
                    : 'Search applications by name or publisher...'
                }
                value={searchFilter}
                onChange={(e) => setSearchFilter(e.target.value)}
                onKeyDown={handleSearchKeyDown}
                disabled={isLoading && !hasDiscovered}
                aria-label="Search applications"
              />
            </div>

            <button
              type="button"
              className="refresh-button"
              onClick={fetchDiscovery}
              disabled={isLoading}
              aria-label="Refresh catalog"
              title="Refresh application catalog"
            >
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
                <path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67" />
              </svg>
              <span>{isLoading ? 'Scanning...' : 'Refresh Catalog'}</span>
            </button>
          </div>

          {/* Source Filter Pills */}
          <div className="catalog-source-filters" role="group" aria-label="Source filters">
            {SOURCE_FILTERS.map((filter) => (
              <button
                key={filter.id}
                type="button"
                className={`source-filter-pill ${activeSourceFilter === filter.id ? 'active' : ''}`}
                onClick={() => setActiveSourceFilter(filter.id)}
                aria-pressed={activeSourceFilter === filter.id}
              >
                {filter.label}
              </button>
            ))}
          </div>

          {errorMsg && (
            <div className="error-banner" role="alert">
              <p className="error-message">Applications could not be loaded.</p>
              <p className="error-detail">{errorMsg}</p>
              <div className="error-actions">
                <button
                  type="button"
                  className="retry-button"
                  onClick={fetchDiscovery}
                  aria-label="Retry application discovery"
                >
                  Retry
                </button>
              </div>
            </div>
          )}

          {partialFailures.length > 0 && (
            <div className="warning-banner" role="status">
              <strong>Partial Discovery Notice:</strong> Some registry or directory locations could not be scanned:
              <ul>
                {partialFailures.map((failure, idx) => (
                  <li key={idx}>{failure}</li>
                ))}
              </ul>
            </div>
          )}

          {hasDiscovered && !errorMsg && (
            <div className="summary-bar" role="region" aria-label="Catalog summary metrics">
              <div className="summary-item">
                Found: <strong>{counts.total}</strong>
              </div>
              <div className="summary-item">
                Ready: <strong className="summary-value-available">{counts.available}</strong>
              </div>
              <div className="summary-item">
                Unverified: <strong className="summary-value-unverified">{counts.unverified}</strong>
              </div>
              {counts.incomplete > 0 && (
                <div className="summary-item">
                  Limited info: <strong className="summary-value-incomplete">{counts.incomplete}</strong>
                </div>
              )}
              {counts.missing > 0 && (
                <div className="summary-item">
                  Missing: <strong className="summary-value-missing">{counts.missing}</strong>
                </div>
              )}
            </div>
          )}

          {scannedSources.length > 0 && (
            <div className="sources-list" aria-label="Scanned sources">
              {scannedSources.map((source, idx) => (
                <span key={idx} className="source-pill">
                  ✓ {source}
                </span>
              ))}
            </div>
          )}

          {/* Dynamic count indicator */}
          {hasDiscovered && !errorMsg && (
            <div className="catalog-count-row">
              <span className="catalog-count-indicator" aria-live="polite">
                {`Showing ${filteredApps.length} ${filteredApps.length === 1 ? 'application' : 'applications'}`}
                {applications.length !== filteredApps.length && ` (filtered from ${applications.length} total)`}
              </span>
            </div>
          )}

          {isLoading && !hasDiscovered ? (
            <div className="loading-state" role="status">
              <p>Loading applications...</p>
            </div>
          ) : applications.length === 0 && hasDiscovered && !errorMsg ? (
            <div className="empty-state" role="status">
              <p className="empty-title">No applications found.</p>
              <p className="empty-subtitle">Try refreshing the application catalog.</p>
            </div>
          ) : (
            <div className="catalog-master-detail">
              <div className="catalog-master-pane">
                {filteredApps.length === 0 ? (
                  <div className="empty-state" role="status">
                    <p>No applications match your filter.</p>
                  </div>
                ) : (
                  <div className="catalog-grid" role="region" aria-label="Discovered applications list">
                    {filteredApps.map((app, index) => {
                      const isAppActive = activeLaunch?.applicationId === app.id;
                      const isSelected = selectedAppId === app.id;
                      return (
                        <ApplicationCard
                          key={app.id}
                          cardRef={(el) => {
                            cardRefs.current[index] = el;
                          }}
                          tabIndex={0}
                          isSelected={isSelected}
                          onClick={() => setSelectedAppId(app.id)}
                          onKeyDown={handleCardKeyDown(index)}
                          application={app}
                          showLaunchButton={false}
                          launchStatus={isAppActive ? activeLaunch.status : undefined}
                          launchedPid={isAppActive ? activeLaunch.launchedPid : undefined}
                          launchErrorMessage={isAppActive ? activeLaunch.errorMessage : undefined}
                          onLaunchRequest={() => launch(app.id)}
                          onLaunchSuccess={onLaunchSuccess}
                          onLaunchError={onLaunchError}
                        />
                      );
                    })}
                  </div>
                )}
              </div>

              <div className="catalog-inspector-pane">
                <ApplicationInspector
                  application={selectedApp}
                  launchStatus={activeLaunch && selectedApp && activeLaunch.applicationId === selectedApp.id ? activeLaunch.status : undefined}
                  launchedPid={activeLaunch && selectedApp && activeLaunch.applicationId === selectedApp.id ? activeLaunch.launchedPid : undefined}
                  launchErrorMessage={activeLaunch && selectedApp && activeLaunch.applicationId === selectedApp.id ? activeLaunch.errorMessage : undefined}
                  launchErrorCode={activeLaunch && selectedApp && activeLaunch.applicationId === selectedApp.id ? activeLaunch.errorCode : undefined}
                  onLaunchRequest={() => selectedApp && launch(selectedApp.id)}
                  onLaunchSuccess={onLaunchSuccess}
                  onLaunchError={onLaunchError}
                />
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
