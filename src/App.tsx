import { useEffect, useState } from 'react';
import { deskpilotApi, type AppStatusDto } from './lib/ipc';
import { ApplicationCatalog } from './components';
import './App.css';

/**
 * Returns the CSS modifier class for an availability state.
 * Preserves the exact ternary conditional formatting from the user's pre-existing modification.
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

function App() {
  const [appStatus, setAppStatus] = useState<AppStatusDto | null>(null);

  useEffect(() => {
    deskpilotApi.getAppStatus().then(setAppStatus).catch(() => null);
  }, []);

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
      </header>

      <main className="main-content">
        <ApplicationCatalog />
      </main>
    </div>
  );
}

export default App;
