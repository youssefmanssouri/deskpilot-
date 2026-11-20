import { useEffect, useState, useCallback } from 'react';
import {
  deskpilotApi,
  type AppStatusDto,
  type NavigateRequestDto,
  type ShortcutStatusDto,
} from './lib/ipc';
import { isTauriRuntime, NAVIGATE_EVENT, onTauriEvent } from './lib/tauriEnv';
import { applyAppearance } from './lib/appearance';
import {
  ApplicationCatalog,
  type CatalogView,
  SystemMetricsProvider,
  useSharedSystemMetrics,
  formatUptime,
  getGreeting,
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
export function getHeaderInfo(
  view: CatalogView,
  now: Date = new Date()
): { title: string; subtitle: string } {
  switch (view) {
    case 'settings':
      return {
        title: 'Settings',
        subtitle: 'Theme, shortcut, startup and privacy.',
      };
    case 'home':
      return {
        title: getGreeting(now),
        subtitle: 'Everything on this PC, one shortcut away.',
      };
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
  const [activeView, setActiveView] = useState<CatalogView>('home');
  const [selectRequest, setSelectRequest] = useState<{ applicationId: string; nonce: number } | null>(null);
  const [shortcut, setShortcut] = useState<ShortcutStatusDto | null>(null);

  // The quick launcher asks the main window to show a view or a specific app.
  useEffect(() => {
    return onTauriEvent<NavigateRequestDto>(NAVIGATE_EVENT, (req) => {
      const views: CatalogView[] = ['home', 'applications', 'processes', 'system'];
      if (!views.includes(req.view)) return;
      setActiveView(req.view);
      if (req.applicationId) {
        setSelectRequest({ applicationId: req.applicationId, nonce: Date.now() });
      }
    });
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    deskpilotApi.getLauncherShortcut().then(setShortcut).catch(() => {});
    deskpilotApi
      .getSettings()
      .then((s) => s && applyAppearance(s))
      .catch(() => {});
  }, []);

  // Keep the sidebar shortcut hint current after it is changed in Settings.
  useEffect(() => {
    if (activeView !== 'settings' || !isTauriRuntime()) return;
    return () => {
      deskpilotApi.getLauncherShortcut().then(setShortcut).catch(() => {});
    };
  }, [activeView]);

  useEffect(() => {
    deskpilotApi.getAppStatus().then(setAppStatus).catch(() => null);
  }, []);

  // Set when Ctrl+K is pressed on Home: switch to Apps, then focus its search once rendered.
  const [pendingSearchFocus, setPendingSearchFocus] = useState(false);

  useEffect(() => {
    if (pendingSearchFocus && activeView === 'applications') {
      setPendingSearchFocus(false);
      const searchInput = document.getElementById('catalog-search-input') as HTMLInputElement | null;
      searchInput?.focus();
      searchInput?.select();
    }
  }, [pendingSearchFocus, activeView]);

  const focusSearchInput = useCallback(() => {
    if (activeView === 'home') {
      setPendingSearchFocus(true);
      setActiveView('applications');
    } else if (activeView === 'applications') {
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

        if (activeView === 'home' || activeView === 'applications' || activeView === 'processes') {
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
            <span
              className="app-brand-badge"
              title={appStatus ? `Core v${appStatus.version} (${appStatus.environment})` : undefined}
            >
              {appStatus ? `v${appStatus.version}` : 'v0.1.1'}
            </span>
          </div>

          <nav className="app-sidebar-nav" aria-label="Views">
            <button
              type="button"
              className={`app-nav-button ${activeView === 'home' ? 'active' : ''}`}
              onClick={() => setActiveView('home')}
              aria-current={activeView === 'home' ? 'page' : undefined}
              aria-label="Home"
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
                <rect x="3" y="3" width="7" height="9" rx="2" />
                <rect x="14" y="3" width="7" height="5" rx="2" />
                <rect x="14" y="12" width="7" height="9" rx="2" />
                <rect x="3" y="16" width="7" height="5" rx="2" />
              </svg>
              <span className="app-nav-label">Home</span>
            </button>

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
          <button
            type="button"
            className={`app-nav-button ${activeView === 'settings' ? 'active' : ''}`}
            onClick={() => setActiveView('settings')}
            aria-current={activeView === 'settings' ? 'page' : undefined}
            aria-label="Settings"
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
              <line x1="4" y1="6" x2="20" y2="6" />
              <line x1="4" y1="12" x2="20" y2="12" />
              <line x1="4" y1="18" x2="20" y2="18" />
              <circle cx="9" cy="6" r="2" />
              <circle cx="15" cy="12" r="2" />
              <circle cx="8" cy="18" r="2" />
            </svg>
            <span className="app-nav-label">Settings</span>
          </button>
          {shortcut && (
            <div
              className={`sidebar-launcher-hint ${shortcut.registered ? '' : 'warning'}`.trim()}
              role="status"
              title={shortcut.message ?? undefined}
            >
              {shortcut.registered ? (
                <>
                  <kbd>{shortcut.accelerator}</kbd>
                  <span>opens the quick launcher from anywhere</span>
                </>
              ) : (
                <span>Quick launcher shortcut unavailable. Hover for details.</span>
              )}
            </div>
          )}
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
            </p>
          </div>
        </header>

        {/* Main Workspace */}
        <main className="app-workspace" id="app-workspace-content">
          <div className="container">
            <ApplicationCatalog
              activeView={activeView}
              onViewChange={setActiveView}
              selectRequest={selectRequest}
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
