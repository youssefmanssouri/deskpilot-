import { useState, useEffect, useMemo, useCallback, useRef } from 'react';
import {
  deskpilotApi,
  type ApplicationDto,
  type DiscoveryResultDto,
  IpcError,
} from '../lib/ipc';
import { ApplicationCard } from './ApplicationCard';
import './ApplicationCatalog.css';

export interface ApplicationCatalogProps {
  initialApplications?: ApplicationDto[];
  initialScannedSources?: string[];
  initialPartialFailures?: string[];
  autoDiscover?: boolean;
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
 * aggregate metrics, and application card rendering.
 *
 * Guarantees:
 * - Employs existing discover_applications Tauri IPC command.
 * - Manages explicit visual states: Loading, Empty, Error with Retry, Loaded.
 * - Search is completely client-side with zero backend queries.
 * - Passes only safe identity metadata down to child cards.
 */
export function ApplicationCatalog({
  initialApplications,
  initialScannedSources = [],
  initialPartialFailures = [],
  autoDiscover = true,
  onLaunchSuccess,
  onLaunchError,
  className = '',
}: ApplicationCatalogProps) {
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
  const [hasDiscovered, setHasDiscovered] = useState<boolean>(initialApplications !== undefined);
  const discoveryRequestIdRef = useRef<number>(0);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const hasAutoFocusedRef = useRef<boolean>(false);
  const cardRefs = useRef<(HTMLElement | null)[]>([]);

  const fetchDiscovery = useCallback(async () => {
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

  const filteredApps = useMemo(() => {
    return filterApplications(applications, searchFilter);
  }, [applications, searchFilter]);

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
      // Launch the first available application in filtered results
      const firstAvailable = filteredApps.find((app) => app.availability === 'Available');
      if (firstAvailable) {
        const cardEl = document.querySelector(
          `[data-testid="app-card-${firstAvailable.id}"]`
        );
        const btn = cardEl?.querySelector('button.launch-button') as HTMLButtonElement | null;
        btn?.click();
      }
    } else if (e.key === 'ArrowDown') {
      if (filteredApps.length > 0) {
        e.preventDefault();
        cardRefs.current[0]?.focus();
      }
    }
  };

  const handleCardKeyDown = (index: number): React.KeyboardEventHandler<HTMLElement> => (e) => {
    if (e.key === 'ArrowDown' || e.key === 'ArrowRight') {
      e.preventDefault();
      if (index + 1 < filteredApps.length) {
        cardRefs.current[index + 1]?.focus();
      }
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowLeft') {
      e.preventDefault();
      if (index > 0) {
        cardRefs.current[index - 1]?.focus();
      } else {
        searchInputRef.current?.focus();
      }
    } else if (e.key === 'Home') {
      e.preventDefault();
      cardRefs.current[0]?.focus();
    } else if (e.key === 'End') {
      e.preventDefault();
      cardRefs.current[filteredApps.length - 1]?.focus();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      searchInputRef.current?.focus();
    }
  };

  return (
    <div className={`catalog-container ${className}`.trim()}>
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
            placeholder="Search applications by name or publisher..."
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
        >
          {isLoading ? 'Scanning...' : 'Refresh Catalog'}
        </button>
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
            Total Discovered: <strong>{counts.total}</strong>
          </div>
          <div className="summary-item">
            Available: <strong style={{ color: '#58a6ff' }}>{counts.available}</strong>
          </div>
          <div className="summary-item">
            Unverified: <strong style={{ color: '#d29922' }}>{counts.unverified}</strong>
          </div>
          {counts.incomplete > 0 && (
            <div className="summary-item">
              Incomplete: <strong style={{ color: '#8b949e' }}>{counts.incomplete}</strong>
            </div>
          )}
          {counts.missing > 0 && (
            <div className="summary-item">
              Missing Target: <strong style={{ color: '#f85149' }}>{counts.missing}</strong>
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

      {isLoading && !hasDiscovered ? (
        <div className="loading-state" role="status">
          <p>Loading applications...</p>
        </div>
      ) : applications.length === 0 && hasDiscovered && !errorMsg ? (
        <div className="empty-state" role="status">
          <p className="empty-title">No applications found.</p>
          <p className="empty-subtitle">Try refreshing the application catalog.</p>
        </div>
      ) : filteredApps.length === 0 && applications.length > 0 ? (
        <div className="empty-state" role="status">
          <p>No applications match your filter.</p>
        </div>
      ) : (
        <div className="catalog-grid" role="region" aria-label="Discovered applications list">
          {filteredApps.map((app, index) => (
            <ApplicationCard
              key={app.id}
              cardRef={(el) => {
                cardRefs.current[index] = el;
              }}
              tabIndex={0}
              onKeyDown={handleCardKeyDown(index)}
              application={app}
              onLaunchSuccess={onLaunchSuccess}
              onLaunchError={onLaunchError}
            />
          ))}
        </div>
      )}
    </div>
  );
}

