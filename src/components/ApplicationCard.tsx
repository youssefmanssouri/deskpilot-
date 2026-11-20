import type { ApplicationDto, IpcError } from '../lib/ipc';
import { LaunchButton, type LaunchStatus } from './LaunchButton';
import { canLaunchWithClick } from './launchPolicy';

export interface ApplicationCardProps {
  application: ApplicationDto;
  isSelected?: boolean;
  onClick?: () => void;
  onLaunchSuccess?: (applicationId: string, processId: number) => void;
  onLaunchError?: (applicationId: string, error: IpcError) => void;
  disabled?: boolean;
  className?: string;
  tabIndex?: number;
  onKeyDown?: React.KeyboardEventHandler<HTMLElement>;
  cardRef?: (el: HTMLElement | null) => void;
  autoResetDelayMs?: number;
  launchStatus?: LaunchStatus;
  launchedPid?: number | null;
  launchErrorMessage?: string | null;
  onLaunchRequest?: () => void;
  showLaunchButton?: boolean;
}

/**
 * Returns the user-facing label for an availability state.
 * The DTO values stay as they are; only the displayed words change.
 */
export function getAvailabilityLabel(availability: ApplicationDto['availability']): string {
  switch (availability) {
    case 'Available':
      return 'Ready';
    case 'Unverified':
      return 'Unverified';
    case 'IncompleteMetadata':
      return 'Limited info';
    case 'MissingTarget':
      return 'Missing';
    default:
      return availability;
  }
}

/**
 * Returns the CSS modifier class for the availability badge.
 * Preserves the exact ternary logic and formatting from DeskPilot Phase 3.
 */
export function getAvailabilityBadgeClass(availability: ApplicationDto['availability']): string {
  return availability === 'Available'
    ? 'badge-available'
    : availability === 'Unverified'
      ? 'badge-unverified'
      : availability === 'MissingTarget'
        ? 'badge-missing'
        : 'badge-incomplete';
}

/**
 * Two-letter monogram shown in place of an app icon (real icons would need a new
 * backend command, which is out of scope). Example: "Visual Studio Code" -> "VS".
 */
export function getAppInitials(displayName: string): string {
  const words = displayName.split(/[\s\-_.]+/).filter((w) => /[A-Za-z0-9]/.test(w));
  const letters = words.slice(0, 2).map((w) => w.match(/[A-Za-z0-9]/)![0]);
  return (letters.join('') || '?').toUpperCase();
}

/**
 * ApplicationCard renders presentation metadata for a single discovered application.
 *
 * Conforms to least-privilege security guarantees:
 * - Operates purely on high-level identity (`id`, `displayName`, `publisher`, `version`).
 * - Never receives, formats, or renders filesystem paths, usernames, or execution commands.
 * - Delegates launching to `<LaunchButton applicationId={application.id} />`.
 */
export function ApplicationCard({
  application,
  isSelected = false,
  onClick,
  onLaunchSuccess,
  onLaunchError,
  disabled = false,
  className = '',
  tabIndex,
  onKeyDown,
  cardRef,
  autoResetDelayMs,
  launchStatus,
  launchedPid,
  launchErrorMessage,
  onLaunchRequest,
  showLaunchButton = true,
}: ApplicationCardProps) {
  const isLaunchDisabled = disabled || !canLaunchWithClick(application);
  const badgeClass = getAvailabilityBadgeClass(application.availability);
  const cardTitleId = `app-card-title-${application.id}`;

  const handleKeyDown: React.KeyboardEventHandler<HTMLElement> = (e) => {
    if (e.target === e.currentTarget && (e.key === 'Enter' || e.key === ' ')) {
      e.preventDefault();
      onClick?.();
      if (showLaunchButton && !isLaunchDisabled) {
        const btn = e.currentTarget.querySelector('button.launch-button') as HTMLButtonElement | null;
        btn?.click();
      }
    }
    onKeyDown?.(e);
  };

  if (!showLaunchButton) {
    // List-row mode, used by the catalog. Rendered as a listbox option, so it must not
    // contain interactive children; launching happens from the inspector or with Enter.
    return (
      <div
        ref={cardRef}
        role="option"
        tabIndex={tabIndex ?? -1}
        className={`app-card app-card--row ${isSelected ? 'selected' : ''} ${className}`.trim()}
        aria-labelledby={cardTitleId}
        aria-selected={isSelected}
        data-testid={`app-card-${application.id}`}
        onClick={onClick}
        onKeyDown={onKeyDown}
      >
        <span className="app-row-tile" aria-hidden="true">
          {getAppInitials(application.displayName)}
        </span>
        <span id={cardTitleId} className="app-card-title">
          {application.displayName}
        </span>
        <span className="app-card-publisher">{application.publisher ?? 'Unknown Publisher'}</span>
        <span className="app-card-version">{application.version ? `v${application.version}` : ''}</span>
        <span className={`app-row-status ${badgeClass}`}>
          {getAvailabilityLabel(application.availability)}
        </span>
      </div>
    );
  }

  return (
    <article
      ref={cardRef}
      tabIndex={tabIndex ?? 0}
      className={`app-card ${isSelected ? 'selected' : ''} ${className}`.trim()}
      aria-labelledby={cardTitleId}
      aria-selected={isSelected}
      data-testid={`app-card-${application.id}`}
      onClick={onClick}
      onKeyDown={handleKeyDown}
    >
      <div className="app-card-header">
        <div className="app-card-title-group">
          <h3 id={cardTitleId} className="app-card-title">
            {application.displayName}
          </h3>
          <span className={`badge ${badgeClass}`} role="status">
            {getAvailabilityLabel(application.availability)}
          </span>
        </div>
      </div>

      <div className="app-card-meta">
        <div className="app-card-publisher">
          {application.publisher ?? 'Unknown Publisher'}
        </div>
        {application.version && (
          <div className="app-card-version">
            {`v${application.version}`}
          </div>
        )}
      </div>

      {application.sources && application.sources.length > 0 && (
        <div className="app-card-sources" aria-label="Discovery sources">
          {application.sources.map((source, idx) => (
            <span key={idx} className="source-tag">
              {source}
            </span>
          ))}
        </div>
      )}

      {showLaunchButton && (
        <div className="app-card-actions">
          <LaunchButton
            applicationId={application.id}
            disabled={isLaunchDisabled}
            autoResetDelayMs={autoResetDelayMs}
            status={launchStatus}
            launchedPid={launchedPid}
            errorMessage={launchErrorMessage}
            onLaunch={onLaunchRequest}
            onSuccess={(pid) => onLaunchSuccess?.(application.id, pid)}
            onError={(err) => onLaunchError?.(application.id, err)}
          />
        </div>
      )}
    </article>
  );
}
