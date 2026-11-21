import type { ApplicationDto, LaunchHistoryEntryDto } from '../lib/ipc';
import { AppIcon } from './AppIcon';
import { CpuSparkline } from './CpuSparkline';
import { canLaunchWithClick } from './launchPolicy';
import { formatBytes, formatUptime } from './SystemMonitor';
import type { ActiveLaunchState } from './useApplicationLauncher';
import { useSharedSystemMetrics } from './useSystemMetrics';
import './HomeView.css';

/** Maximum recent launches kept, matching the backend's MAX_HISTORY_ENTRIES. */
export const MAX_RECENT_ENTRIES = 10;
/** Recent apps shown as cards on Home. */
export const RECENT_CARDS_SHOWN = 6;
/** Drives at or above this usage appear under "Needs attention". */
export const DRIVE_ATTENTION_PERCENT = 75;

/** "Good morning" (05:00-11:59), "Good afternoon" (12:00-17:59), "Good evening" otherwise. */
export function getGreeting(now: Date = new Date()): string {
  const h = now.getHours();
  if (h >= 5 && h < 12) return 'Good morning';
  if (h >= 12 && h < 18) return 'Good afternoon';
  return 'Good evening';
}

/** Short relative time: "Just now", "5 min ago", "3 h ago", "Yesterday", "4 days ago". */
export function formatRelativeTime(thenMs: number, nowMs: number = Date.now()): string {
  const diff = Math.max(0, nowMs - thenMs);
  const min = Math.floor(diff / 60_000);
  if (min < 1) return 'Just now';
  if (min < 60) return `${min} min ago`;
  const hours = Math.floor(min / 60);
  if (hours < 24) return `${hours} h ago`;
  const days = Math.floor(hours / 24);
  if (days === 1) return 'Yesterday';
  return `${days} days ago`;
}

/**
 * Frontend mirror of the backend's push rule: newest first, one entry per app,
 * at most MAX_RECENT_ENTRIES. Used to update Home immediately after a launch;
 * the backend file stays the source of truth on the next load.
 */
export function pushRecentEntry(
  entries: LaunchHistoryEntryDto[],
  entry: LaunchHistoryEntryDto
): LaunchHistoryEntryDto[] {
  return [entry, ...entries.filter((e) => e.applicationId !== entry.applicationId)].slice(
    0,
    MAX_RECENT_ENTRIES
  );
}

function attentionReason(app: ApplicationDto): string {
  switch (app.availability) {
    case 'MissingTarget':
      return 'Program file is missing';
    case 'Unverified':
      return "DeskPilot couldn't confirm its program file";
    case 'IncompleteMetadata':
      return 'No publisher or version listed';
    default:
      return '';
  }
}

export interface HomeViewProps {
  applications: ApplicationDto[];
  hasDiscovered: boolean;
  history: LaunchHistoryEntryDto[];
  historyLoaded: boolean;
  historyError: string | null;
  activeLaunch: ActiveLaunchState | null;
  onOpenApp: (applicationId: string) => void;
  onShowApp: (applicationId: string) => void;
  onShowApps: (filter: 'ready' | 'attention') => void;
  onShowSystem: () => void;
  onClearHistory: () => void;
  /** Injectable clock for deterministic tests. */
  nowMs?: number;
}

/**
 * Home: a summary of this PC built only from data DeskPilot already has
 * (catalog, shared system metrics) plus the local launch history.
 */
export function HomeView({
  applications,
  hasDiscovered,
  history,
  historyLoaded,
  historyError,
  activeLaunch,
  onOpenApp,
  onShowApp,
  onShowApps,
  onShowSystem,
  onClearHistory,
  nowMs,
}: HomeViewProps) {
  const shared = useSharedSystemMetrics();
  const metrics = shared?.metrics ?? null;
  const now = nowMs ?? Date.now();

  const readyCount = applications.filter((a) => a.availability === 'Available').length;
  const attentionApps = applications.filter((a) => a.availability !== 'Available');
  const fullDrives = (metrics?.storage.drives ?? []).filter(
    (d) => d.usedPercent >= DRIVE_ATTENTION_PERCENT
  );
  const appsById = new Map(applications.map((a) => [a.id, a]));
  const recent = history.slice(0, RECENT_CARDS_SHOWN);

  const cpu = metrics?.cpu.utilizationPercent ?? null;
  const memPct = metrics?.memory.usedPercent ?? null;
  const ringRadius = 46;
  const ringCirc = 2 * Math.PI * ringRadius;

  return (
    <div className="home-view" role="region" aria-label="Home">
      <div className="home-grid">
        {/* At a glance */}
        <section className="home-card home-card--ink" aria-labelledby="home-glance-title">
          <h2 id="home-glance-title" className="home-card-title">
            Your PC at a glance
          </h2>
          <div className="home-glance-numbers">
            <div className="home-glance-number">
              <strong>{hasDiscovered ? applications.length : '—'}</strong>
              <span>apps found</span>
            </div>
            <div className="home-glance-divider" aria-hidden="true" />
            <div className="home-glance-number">
              <strong className="home-glance-uptime">
                {metrics ? formatUptime(metrics.system.uptimeSeconds) : '—'}
              </strong>
              <span>PC running for</span>
            </div>
          </div>
          <div className="home-glance-tiles">
            <button type="button" className="home-tile" onClick={() => onShowApps('ready')}>
              <strong>{readyCount}</strong>
              <span>Ready</span>
            </button>
            <button type="button" className="home-tile" onClick={() => onShowApps('attention')}>
              <strong>{attentionApps.length}</strong>
              <span>Need attention</span>
            </button>
            <button type="button" className="home-tile" onClick={onShowSystem}>
              <strong>{metrics ? metrics.storage.drives.length : '—'}</strong>
              <span>{metrics?.storage.drives.length === 1 ? 'Drive' : 'Drives'}</span>
            </button>
          </div>
        </section>

        {/* Processor */}
        <section className="home-card" aria-labelledby="home-cpu-title">
          <div className="home-card-head">
            <h2 id="home-cpu-title" className="home-card-title">
              Processor
            </h2>
            <span className="home-pill">{cpu === null ? '—' : `${cpu.toFixed(0)}%`}</span>
          </div>
          <p className="home-card-meta">CPU load</p>
          <div className="home-sparkline">
            <CpuSparkline samples={shared?.cpuHistory ?? []} width={260} height={72} />
          </div>
          <p className="home-card-meta">{metrics?.cpu.brandName ?? 'Waiting for the first sample…'}</p>
        </section>

        {/* Memory */}
        <section className="home-card" aria-labelledby="home-mem-title">
          <h2 id="home-mem-title" className="home-card-title">
            Memory
          </h2>
          <div className="home-memory">
            <svg className="home-ring" viewBox="0 0 110 110" role="img" aria-label={memPct === null ? 'Memory use unknown' : `${memPct.toFixed(0)}% of memory in use`}>
              <circle cx="55" cy="55" r={ringRadius} className="home-ring-track" />
              <circle
                cx="55"
                cy="55"
                r={ringRadius}
                className="home-ring-value"
                strokeDasharray={`${((memPct ?? 0) / 100) * ringCirc} ${ringCirc}`}
                transform="rotate(-90 55 55)"
              />
              <text x="55" y="55" className="home-ring-label">
                {memPct === null ? '—' : `${memPct.toFixed(0)}%`}
              </text>
            </svg>
            <dl className="home-memory-legend">
              <dt>In use</dt>
              <dd>{metrics ? formatBytes(metrics.memory.usedBytes) : '—'}</dd>
              <dt>Free</dt>
              <dd>{metrics ? formatBytes(metrics.memory.availableBytes) : '—'}</dd>
            </dl>
          </div>
          <button type="button" className="home-link-button" onClick={onShowSystem}>
            Open System
          </button>
        </section>

        {/* Needs attention */}
        <section className="home-card" aria-labelledby="home-attention-title">
          <div className="home-card-head">
            <h2 id="home-attention-title" className="home-card-title">
              Needs attention
            </h2>
            <span className="home-card-meta">{attentionApps.length + fullDrives.length}</span>
          </div>
          {attentionApps.length + fullDrives.length === 0 ? (
            <p className="home-empty">{hasDiscovered ? 'Nothing needs attention.' : 'Checking…'}</p>
          ) : (
            <ul className="home-attention-list">
              {attentionApps.slice(0, 5).map((app) => (
                <li key={app.id}>
                  <span className={`home-attention-dot home-attention-dot--${app.availability}`} aria-hidden="true" />
                  <span className="home-attention-text">
                    <span className="home-attention-name">{app.displayName}</span>
                    <span className="home-attention-why">{attentionReason(app)}</span>
                  </span>
                  <button
                    type="button"
                    className="home-link-button"
                    onClick={() => onShowApp(app.id)}
                    aria-label={`View ${app.displayName}`}
                  >
                    View
                  </button>
                </li>
              ))}
              {fullDrives.map((d) => (
                <li key={d.rootPath}>
                  <span className="home-attention-dot home-attention-dot--drive" aria-hidden="true" />
                  <span className="home-attention-text">
                    <span className="home-attention-name">
                      Drive {d.rootPath} is {d.usedPercent.toFixed(0)}% full
                    </span>
                    <span className="home-attention-why">{formatBytes(d.availableBytes)} free</span>
                  </span>
                  <button
                    type="button"
                    className="home-link-button"
                    onClick={onShowSystem}
                    aria-label={`View drive ${d.rootPath}`}
                  >
                    View
                  </button>
                </li>
              ))}
              {attentionApps.length > 5 && (
                <li className="home-attention-more">
                  <button type="button" className="home-link-button" onClick={() => onShowApps('attention')}>
                    Show all {attentionApps.length} apps that need attention
                  </button>
                </li>
              )}
            </ul>
          )}
        </section>
        {/* Recent apps */}
        <section className="home-card home-card--wide home-recent" aria-labelledby="home-recent-title">
          <div className="home-card-head">
            <h2 id="home-recent-title" className="home-card-title">
              Recent apps
            </h2>
            {history.length > 0 && (
              <button
                type="button"
                className="home-link-button"
                onClick={onClearHistory}
                aria-label="Clear recent apps"
              >
                Clear
              </button>
            )}
          </div>
          {historyError && (
            <p className="home-error" role="alert">
              {historyError}
            </p>
          )}
          {!historyLoaded ? (
            <p className="home-card-meta">Loading recent apps…</p>
          ) : recent.length === 0 ? (
            <p className="home-empty">
              Apps you open from DeskPilot appear here. History stays on this PC.
            </p>
          ) : (
            <ul className="home-recent-list">
              {recent.map((entry) => {
                const app = appsById.get(entry.applicationId);
                const status =
                  activeLaunch?.applicationId === entry.applicationId ? activeLaunch.status : null;
                const canOpen = hasDiscovered && !!app && canLaunchWithClick(app);
                return (
                  <li key={entry.applicationId} className="home-recent-card">
                    <AppIcon
                      applicationId={entry.applicationId}
                      displayName={entry.displayName}
                      className="home-recent-tile"
                    />
                    <span className="home-recent-name">{entry.displayName}</span>
                    <span className="home-recent-when">
                      {status === 'launching'
                        ? 'Opening…'
                        : status === 'launched'
                          ? 'Opened just now'
                          : hasDiscovered && !app
                            ? 'No longer found'
                            : formatRelativeTime(entry.launchedAtMs, now)}
                    </span>
                    {status === 'failed' && (
                      <button
                        type="button"
                        className="home-recent-failed"
                        onClick={() => onShowApp(entry.applicationId)}
                      >
                        Couldn't open · Details
                      </button>
                    )}
                    <button
                      type="button"
                      className="home-recent-open"
                      disabled={!canOpen || status === 'launching'}
                      onClick={() => onOpenApp(entry.applicationId)}
                      aria-label={`Open ${entry.displayName}`}
                    >
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                        <path d="M7 17 17 7M9 7h8v8" />
                      </svg>
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </section>

      </div>
    </div>
  );
}
