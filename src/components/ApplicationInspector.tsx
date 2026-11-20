import type { ApplicationDto, IpcError } from '../lib/ipc';
import { LaunchButton, type LaunchStatus } from './LaunchButton';
import { getAppInitials, getAvailabilityBadgeClass, getAvailabilityLabel } from './ApplicationCard';
import { canLaunchWithClick, canLaunchWithKeyboard } from './launchPolicy';
import { describeLaunchError } from './launchErrorDescription';
import './ApplicationInspector.css';

export interface ApplicationInspectorProps {
  application: ApplicationDto | null;
  launchStatus?: LaunchStatus;
  launchedPid?: number | null;
  launchErrorMessage?: string | null;
  launchErrorCode?: string | null;
  onLaunchRequest?: () => void;
  onLaunchSuccess?: (applicationId: string, processId: number) => void;
  onLaunchError?: (applicationId: string, error: IpcError) => void;
  className?: string;
}

/**
 * ApplicationInspector renders detailed metadata and primary actions for a selected application.
 *
 * Conforms strictly to DeskPilot least-privilege security invariants:
 * - Operates purely on audited, high-level DTO metadata (`displayName`, `publisher`, `version`, `sources`, `availability`).
 * - Strictly excludes filesystem paths, registry subkeys, command-line arguments, or working directories.
 * - Delegates launching to the secure IPC boundary via `<LaunchButton />`.
 */
export function ApplicationInspector({
  application,
  launchStatus,
  launchedPid,
  launchErrorMessage,
  launchErrorCode,
  onLaunchRequest,
  onLaunchSuccess,
  onLaunchError,
  className = '',
}: ApplicationInspectorProps) {
  if (!application) {
    return (
      <aside
        className={`app-inspector ${className}`.trim()}
        aria-label="Details"
        data-testid="application-inspector-empty"
      >
        <div className="app-inspector-header">
          <h3 className="app-inspector-title">Details</h3>
        </div>
        <div className="app-inspector-body">
          <div className="app-inspector-empty">
            <svg
              className="app-inspector-empty-icon"
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
            <h4 className="app-inspector-empty-title">No app selected</h4>
            <p className="app-inspector-empty-text">
              Select an app on the left to see its details and open it.
            </p>
          </div>
        </div>
      </aside>
    );
  }

  const isLaunchDisabled = !canLaunchWithClick(application);
  const badgeClass = getAvailabilityBadgeClass(application.availability);
  const launchError = launchStatus === 'failed' ? describeLaunchError(launchErrorCode) : null;

  return (
    <aside
      className={`app-inspector ${className}`.trim()}
      aria-label={`Details for ${application.displayName}`}
      data-testid="application-inspector"
    >
      <div className="app-inspector-header">
        <h3 className="app-inspector-title">Details</h3>
        <span className={`badge ${badgeClass}`} role="status">
          {getAvailabilityLabel(application.availability)}
        </span>
      </div>

      <div className="app-inspector-body">
        {/* App Hero */}
        <div className="app-inspector-hero">
          <div className="app-inspector-icon-wrapper" aria-hidden="true">
            {getAppInitials(application.displayName)}
          </div>
          <div className="app-inspector-hero-info">
            <h4 className="app-inspector-app-name">{application.displayName}</h4>
            <span className="app-inspector-publisher">
              {application.publisher ?? 'Unknown Publisher'}
            </span>
          </div>
        </div>

        {/* Primary action: Open, with the inline launch error under it */}
        <div className="app-inspector-actions">
          <LaunchButton
            applicationId={application.id}
            disabled={isLaunchDisabled}
            status={launchStatus}
            launchedPid={launchedPid}
            errorMessage={launchErrorMessage}
            ariaLabel={`Open ${application.displayName}`}
            onLaunch={onLaunchRequest}
            onSuccess={(pid) => onLaunchSuccess?.(application.id, pid)}
            onError={(err) => onLaunchError?.(application.id, err)}
          />
          {launchError && (
            <div
              className="app-inspector-launch-error"
              role="alert"
              data-testid="launch-error-panel"
            >
              <p className="app-inspector-launch-error-title">{launchError.title}</p>
              {launchErrorMessage && (
                <p className="app-inspector-launch-error-detail">{launchErrorMessage}</p>
              )}
              <p className="app-inspector-launch-error-hint">{launchError.hint}</p>
              <button
                type="button"
                className="retry-button"
                onClick={() => onLaunchRequest?.()}
                aria-label={`Try opening ${application.displayName} again`}
              >
                Try again
              </button>
            </div>
          )}
          <p className="app-inspector-shortcut-hint">
            {canLaunchWithKeyboard(application) ? (
              <>
                Press <kbd>Enter</kbd> to open the selected app
              </>
            ) : isLaunchDisabled ? (
              "This app can't be opened because its program file is missing."
            ) : (
              <>
                <kbd>Enter</kbd> only opens verified apps. Use the button to open this
                one.
              </>
            )}
          </p>
        </div>

        {/* Metadata Ledger */}
        <div className="app-inspector-ledger" role="region" aria-label="Metadata Details">
          <div className="app-inspector-ledger-row">
            <span className="app-inspector-ledger-key">Publisher:</span>
            <span className="app-inspector-ledger-value">
              {application.publisher ?? 'Unknown Publisher'}
            </span>
          </div>

          <div className="app-inspector-ledger-row">
            <span className="app-inspector-ledger-key">Version:</span>
            <span className="app-inspector-ledger-value">
              {application.version ? `v${application.version}` : 'Unspecified'}
            </span>
          </div>

          <div className="app-inspector-ledger-row">
            <span className="app-inspector-ledger-key">Status:</span>
            <span className="app-inspector-ledger-value">
              {application.isInstalled ? 'Installed Application' : 'Unregistered'}
            </span>
          </div>

          <div className="app-inspector-ledger-row">
            <span className="app-inspector-ledger-key">Availability:</span>
            <span className="app-inspector-ledger-value">
              {getAvailabilityLabel(application.availability)}
            </span>
          </div>

        </div>

        {/* Provenance: where the app was found, plus its internal id. Collapsed by default. */}
        <details className="app-inspector-provenance">
          <summary>Where DeskPilot found it</summary>
          {application.sources && application.sources.length > 0 && (
            <div className="app-inspector-sources-tags" aria-label="Discovery Sources">
              {application.sources.map((src, idx) => (
                <span key={idx} className="app-inspector-source-pill">
                  {src}
                </span>
              ))}
            </div>
          )}
          <span className="app-inspector-id-pill" title={application.id}>
            ID: {application.id}
          </span>
        </details>


      </div>
    </aside>
  );
}
