import { useEffect, useState, useCallback } from 'react';
import { deskpilotApi, type AppStatusDto } from './lib/ipc';
import {
  ApplicationCatalog,
  type CatalogView,
  SystemMetricsProvider,
  useSharedSystemMetrics,
  formatUptime,
} from './components';
import './App.css';

/**
 * Returns the CSS modifier class for an availability state.
 * Preserves the exact ternary conditional formatting from DeskPilot Phase 3.
 */
export function getBadgeClass(availability: string): string {
  return availability === 'Available'
    ? 'badge-available'
    : availability === 'Unverified'
      ? 'badge-unverified'
      : availability === 'MissingTarget'
        ? 'badge-missing'
        : 'badge-incomplete';
}

/**
 * Returns header title and descriptive subtitle based on the active catalog view.
 */
export function getHeaderInfo(view: CatalogView): { title: string; subtitle: string } {
  switch (view) {
    case 'processes':
      return {
        title: 'Processes',
        subtitle: 'What is running on this PC right now. Read-only.',
      };
    case 'system':
      return {
        title: 'System',
        subtitle: 'Processor, memory, drives and uptime.',
      };
    case 'applications':
    default:
      return {
        title: 'Apps',
        subtitle: 'Search, filter and open the apps installed on this PC.',
      };
  }
}

function SidebarTelemetryPulse() {
  const shared = useSharedSystemMetrics();
  const uptimeStr = shared?.metrics ? formatUptime(shared.metrics.system.uptimeSeconds) : null;
  const cpuStr = shared?.metrics ? `${shared.metrics.cpu.utilizationPercent.toFixed(1)}%` : null;

  return (
    <div className="sidebar-pulse-card" role="region" aria-label="System summary">
      <div className="sidebar-pulse-header">
        <span className="sidebar-pulse-dot" aria-hidden="true" />
        <span>System</span>
      </div>
      <div className="sidebar-pulse-metrics">
        <div className="sidebar-pulse-item">
          <span className="sidebar-pulse-label">Uptime</span>
          <span className="sidebar-pulse-value">{uptimeStr ?? '—'}</span>
        </div>
        <div className="sidebar-pulse-item">
          <span className="sidebar-pulse-label">CPU Load</span>
          <span className="sidebar-pulse-value">{cpuStr ?? '—'}</span>
        </div>
      </div>
    </div>
  );
}

function AppContent() {
  const [appStatus, setAppStatus] = useState<AppStatusDto | null>(null);
  const [activeView, setActiveView] = useState<CatalogView>('applications');

  useEffect(() => {
    deskpilotApi.getAppStatus().then(setAppStatus).catch(() => null);
  }, []);

  const focusSearchInput = useCallback(() => {
    if (activeView === 'applications') {
      const searchInput = document.getElementById('catalog-search-input') as HTMLInputElement | null;
      if (searchInput) {
        searchInput.focus();
        searchInput.select();
      }
    } else if (activeView === 'processes') {
      const processSearchInput = document.getElementById('process-search-input') as HTMLInputElement | null;
      if (processSearchInput) {
        processSearchInput.focus();
        processSearchInput.select();
      }
    }
  }, [activeView]);

  // Context-aware Ctrl+K / Cmd+K hotkey focusing active view search input
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        const target = e.target as HTMLElement | null;
        const isFormElement =
          target &&
          (target.tagName === 'INPUT' ||
            target.tagName === 'TEXTAREA' ||
            target.tagName === 'SELECT' ||
            target.isContentEditable);

        const isSearchInput =
          target &&
          (target.id === 'catalog-search-input' || target.id === 'process-search-input');

        // Do not hijack Ctrl+K while typing in non-search form elements
        if (isFormElement && !isSearchInput) {
          return;
        }

        if (activeView === 'applications' || activeView === 'processes') {
          e.preventDefault();
          focusSearchInput();
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [activeView, focusSearchInput]);

  const headerInfo = getHeaderInfo(activeView);

  return (
    <div className="app-shell">
      {/* Persistent Navigation Sidebar */}
      <aside className="app-sidebar" aria-label="Main Navigation">
        <div className="app-sidebar-top">
          <div className="app-sidebar-brand">
            <svg
              className="app-brand-icon"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
            >
              <polygon points="12 2 2 7 12 12 22 7 12 2" />
              <polyline points="2 17 12 22 22 17" />
              <polyline points="2 12 12 17 22 12" />
            </svg>
            <span className="app-brand-title">DeskPilot</span>
            <span className="app-brand-badge">
              {appStatus ? `v${appStatus.version}` : 'v0.1.1'}
            </span>
          </div>

          <nav className="app-sidebar-nav" aria-label="Views">
            <button
              type="button"
              className={`app-nav-button ${activeView === 'applications' ? 'active' : ''}`}
              onClick={() => setActiveView('applications')}
              aria-current={activeView === 'applications' ? 'page' : undefined}
              aria-label="Apps"
            >
              <svg
                className="app-nav-icon"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden="true"
              >
                <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
                <line x1="3" y1="9" x2="21" y2="9" />
                <line x1="9" y1="21" x2="9" y2="9" />
              </svg>
              <span className="app-nav-label">Apps</span>
            </button>

            <button
              type="button"
              className={`app-nav-button ${activeView === 'processes' ? 'active' : ''}`}
              onClick={() => setActiveView('processes')}
              aria-current={activeView === 'processes' ? 'page' : undefined}
              aria-label="Processes"
            >
              <svg
                className="app-nav-icon"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden="true"
              >
                <circle cx="12" cy="12" r="3" />
                <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
              </svg>
              <span className="app-nav-label">Processes</span>
            </button>

            <button
              type="button"
              className={`app-nav-button ${activeView === 'system' ? 'active' : ''}`}
              onClick={() => setActiveView('system')}
              aria-current={activeView === 'system' ? 'page' : undefined}
              aria-label="System"
            >
              <svg
                className="app-nav-icon"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden="true"
              >
                <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
              </svg>
              <span className="app-nav-label">System</span>
            </button>
          </nav>
        </div>

        <div className="app-sidebar-bottom">
          <SidebarTelemetryPulse />
        </div>
      </aside>

      {/* Main Column */}
      <div className="app-main-layout">
        {/* Top Header */}
        <header className="app-header">
          <div className="app-header-left">
            <div className="app-header-title">
              <h1>{headerInfo.title}</h1>
            </div>
            <p className="app-header-subtitle">
              {headerInfo.subtitle}
              {appStatus && ` • Core v${appStatus.version} (${appStatus.environment})`}
            </p>
          </div>

          <div className="app-header-right">
            <button
              type="button"
              className="global-search-trigger"
              onClick={focusSearchInput}
              disabled={activeView === 'system'}
              aria-label={
                activeView === 'processes'
                  ? 'Search processes (Ctrl+K)'
                  : activeView === 'system'
                    ? 'System'
                    : 'Search applications (Ctrl+K)'
              }
            >
              <svg
                className="global-search-icon"
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
              <span className="global-search-placeholder">
                {activeView === 'processes'
                  ? 'Search processes...'
                  : activeView === 'system'
                    ? 'System'
                    : 'Search applications...'}
              </span>
              {activeView !== 'system' && <kbd className="global-search-hotkey">Ctrl+K</kbd>}
            </button>
          </div>
        </header>

        {/* Main Workspace */}
        <main className="app-workspace" id="app-workspace-content">
          <div className="container">
            <ApplicationCatalog
              activeView={activeView}
              onViewChange={setActiveView}
            />
          </div>
        </main>
      </div>
    </div>
  );
}

function App() {
  return (
    <SystemMetricsProvider defaultRefreshMode="5s">
      <AppContent />
    </SystemMetricsProvider>
  );
}

export default App;
