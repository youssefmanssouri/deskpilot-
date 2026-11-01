import { useEffect, useState, useMemo } from 'react';
import { deskpilotApi, type DiscoveryResultDto, type AppStatusDto, IpcError } from './lib/ipc';
import './App.css';

function App() {
  const [appStatus, setAppStatus] = useState<AppStatusDto | null>(null);
  const [discoveryResult, setDiscoveryResult] = useState<DiscoveryResultDto | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [searchFilter, setSearchFilter] = useState<string>('');

  useEffect(() => {
    loadInitialData();
  }, []);

  async function loadInitialData() {
    setIsLoading(true);
    setErrorMsg(null);
    try {
      const [status, discovery] = await Promise.all([
        deskpilotApi.getAppStatus().catch(() => null),
        deskpilotApi.discoverApplications(),
      ]);
      if (status) setAppStatus(status);
      setDiscoveryResult(discovery);
    } catch (err) {
      const ipcErr = IpcError.fromResponse(err);
      setErrorMsg(`[${ipcErr.code}] ${ipcErr.message}`);
    } finally {
      setIsLoading(false);
    }
  }

  async function handleRefresh() {
    setIsLoading(true);
    setErrorMsg(null);
    try {
      const discovery = await deskpilotApi.discoverApplications();
      setDiscoveryResult(discovery);
    } catch (err) {
      const ipcErr = IpcError.fromResponse(err);
      setErrorMsg(`[${ipcErr.code}] ${ipcErr.message}`);
    } finally {
      setIsLoading(false);
    }
  }

  const filteredApps = useMemo(() => {
    if (!discoveryResult) return [];
    if (!searchFilter.trim()) return discoveryResult.applications;
    const term = searchFilter.toLowerCase();
    return discoveryResult.applications.filter(
      (app) =>
        app.displayName.toLowerCase().includes(term) ||
        (app.publisher && app.publisher.toLowerCase().includes(term))
    );
  }, [discoveryResult, searchFilter]);

  const availabilityCounts = useMemo(() => {
    if (!discoveryResult) return { available: 0, unverified: 0, incomplete: 0, missing: 0 };
    return discoveryResult.applications.reduce(
      (acc, app) => {
        if (app.availability === 'Available') acc.available++;
        else if (app.availability === 'Unverified') acc.unverified++;
        else if (app.availability === 'IncompleteMetadata') acc.incomplete++;
        else if (app.availability === 'MissingTarget') acc.missing++;
        return acc;
      },
      { available: 0, unverified: 0, incomplete: 0, missing: 0 }
    );
  }, [discoveryResult]);

  return (
    <div className="container">
      <header className="header">
        <div className="header-title">
          <h1>DeskPilot Application Discovery</h1>
          <p className="header-subtitle">
            Local-first Windows application metadata catalog & deterministic identity engine
            {appStatus && ` • Core v${appStatus.version} (${appStatus.environment})`}
          </p>
        </div>
        <button
          className="refresh-button"
          onClick={handleRefresh}
          disabled={isLoading}
          type="button"
        >
          {isLoading ? 'Scanning...' : 'Refresh Catalog'}
        </button>
      </header>

      {errorMsg && (
        <div className="error-banner" role="alert">
          <strong>Discovery Error:</strong> {errorMsg}
          <div style={{ marginTop: '0.5rem' }}>
            <button
              onClick={handleRefresh}
              style={{ padding: '0.3rem 0.8rem', fontSize: '0.8rem' }}
              type="button"
            >
              Retry
            </button>
          </div>
        </div>
      )}

      {discoveryResult && discoveryResult.partialFailures.length > 0 && (
        <div className="warning-banner" role="status">
          <strong>Partial Discovery Warning:</strong> Some registry or directory locations could not be scanned:
          <ul style={{ margin: '0.25rem 0 0 1rem', padding: 0 }}>
            {discoveryResult.partialFailures.map((failure, idx) => (
              <li key={idx}>{failure}</li>
            ))}
          </ul>
        </div>
      )}

      <div className="summary-bar">
        <div className="summary-item">
          Total Discovered: <strong>{discoveryResult?.totalDiscovered ?? 0}</strong>
        </div>
        <div className="summary-item">
          Available: <strong style={{ color: '#58a6ff' }}>{availabilityCounts.available}</strong>
        </div>
        <div className="summary-item">
          Unverified: <strong style={{ color: '#d29922' }}>{availabilityCounts.unverified}</strong>
        </div>
        {availabilityCounts.incomplete > 0 && (
          <div className="summary-item">
            Incomplete: <strong style={{ color: '#8b949e' }}>{availabilityCounts.incomplete}</strong>
          </div>
        )}
        <div style={{ marginLeft: 'auto' }}>
          <input
            className="filter-input"
            type="search"
            placeholder="Filter catalog..."
            value={searchFilter}
            onChange={(e) => setSearchFilter(e.target.value)}
            disabled={isLoading || !discoveryResult}
          />
        </div>
      </div>

      {discoveryResult && (
        <div className="sources-list" aria-label="Scanned sources">
          {discoveryResult.scannedSources.map((source, idx) => (
            <span key={idx} className="source-pill">
              ✓ {source}
            </span>
          ))}
        </div>
      )}

      {isLoading ? (
        <div className="loading-state">
          <p>Scanning Windows registry hives and Start Menu directories...</p>
        </div>
      ) : filteredApps.length === 0 ? (
        <div className="empty-state">
          {searchFilter ? 'No applications match your filter.' : 'No installed applications discovered.'}
        </div>
      ) : (
        <table className="catalog-table">
          <thead>
            <tr>
              <th>Application</th>
              <th>Publisher / Version</th>
              <th>Sources</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {filteredApps.map((app) => (
              <tr key={app.id}>
                <td>
                  <span className="app-name">{app.displayName}</span>
                  <span className="app-meta" style={{ fontFamily: 'monospace', fontSize: '0.7rem' }}>
                    {app.id}
                  </span>
                </td>
                <td>
                  <div className="app-meta">{app.publisher ?? 'Unknown Publisher'}</div>
                  {app.version && <div className="app-meta">v{app.version}</div>}
                </td>
                <td>
                  {app.sources.map((src, idx) => (
                    <span key={idx} className="source-tag">
                      {src}
                    </span>
                  ))}
                </td>
                <td>
                  <span
                    className={`badge ${
                      app.availability === 'Available'
                        ? 'badge-available'
                        : app.availability === 'Unverified'
                        ? 'badge-unverified'
                        : app.availability === 'MissingTarget'
                        ? 'badge-missing'
                        : 'badge-incomplete'
                    }`}
                  >
                    {app.availability}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

export default App;
