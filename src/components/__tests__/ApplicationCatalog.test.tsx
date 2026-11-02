import { describe, it, expect, vi, beforeEach } from 'vitest';
import ReactDOMServer from 'react-dom/server';
import {
  ApplicationCatalog,
  filterApplications,
  computeAvailabilityCounts,
} from '../ApplicationCatalog';
import type { ApplicationDto } from '../../lib/ipc';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('ApplicationCatalog Component & Pure Helpers', () => {
  const sampleApps: ApplicationDto[] = [
    {
      id: 'app_00000000000000000000000000000001',
      displayName: 'Visual Studio Code',
      publisher: 'Microsoft Corporation',
      version: '1.95.0',
      sources: ['Registry (HKCU User)'],
      availability: 'Available',
      isInstalled: true,
    },
    {
      id: 'app_00000000000000000000000000000002',
      displayName: 'Google Chrome',
      publisher: 'Google LLC',
      version: '130.0.0.0',
      sources: ['Registry (HKLM 64-bit)', 'Start Menu (Common)'],
      availability: 'Available',
      isInstalled: true,
    },
    {
      id: 'app_00000000000000000000000000000003',
      displayName: 'Broken Application',
      publisher: 'Unknown Publisher',
      sources: ['Start Menu (User)'],
      availability: 'MissingTarget',
      isInstalled: false,
    },
    {
      id: 'app_00000000000000000000000000000004',
      displayName: 'Legacy Utility',
      publisher: 'Acme Corp',
      sources: ['Registry (HKLM 32-bit)'],
      availability: 'Unverified',
      isInstalled: true,
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('filterApplications Pure Function', () => {
    it('returns all applications when search query is empty or whitespace', () => {
      expect(filterApplications(sampleApps, '')).toEqual(sampleApps);
      expect(filterApplications(sampleApps, '   ')).toEqual(sampleApps);
    });

    it('matches by displayName case-insensitively', () => {
      const result = filterApplications(sampleApps, 'visual studio');
      expect(result).toHaveLength(1);
      expect(result[0].displayName).toBe('Visual Studio Code');

      const upperResult = filterApplications(sampleApps, 'CHROME');
      expect(upperResult).toHaveLength(1);
      expect(upperResult[0].displayName).toBe('Google Chrome');
    });

    it('matches by publisher case-insensitively', () => {
      const result = filterApplications(sampleApps, 'microsoft');
      expect(result).toHaveLength(1);
      expect(result[0].displayName).toBe('Visual Studio Code');

      const googleResult = filterApplications(sampleApps, 'google');
      expect(googleResult).toHaveLength(1);
      expect(googleResult[0].displayName).toBe('Google Chrome');
    });

    it('returns empty array when query does not match any application', () => {
      const result = filterApplications(sampleApps, 'nonexistent query 12345');
      expect(result).toHaveLength(0);
    });

    it('does not mutate the source array', () => {
      const appsCopy = [...sampleApps];
      filterApplications(sampleApps, 'chrome');
      expect(sampleApps).toEqual(appsCopy);
    });

    it('ranks exact display-name match above prefix, substring, and publisher matches', () => {
      const rankingApps: ApplicationDto[] = [
        {
          id: 'app_pub_match',
          displayName: 'Terminal Tool',
          publisher: 'Code Corporation',
          sources: ['Registry'],
          availability: 'Available',
          isInstalled: true,
        },
        {
          id: 'app_substring_match',
          displayName: 'Visual Studio Code',
          publisher: 'Microsoft',
          sources: ['Registry'],
          availability: 'Available',
          isInstalled: true,
        },
        {
          id: 'app_prefix_match',
          displayName: 'Code Lite',
          publisher: 'Independent',
          sources: ['Registry'],
          availability: 'Available',
          isInstalled: true,
        },
        {
          id: 'app_exact_match',
          displayName: 'Code',
          publisher: 'Editor',
          sources: ['Registry'],
          availability: 'Available',
          isInstalled: true,
        },
      ];

      const ranked = filterApplications(rankingApps, 'code');

      expect(ranked).toHaveLength(4);
      // Rank 1: exact match ("Code")
      expect(ranked[0].id).toBe('app_exact_match');
      // Rank 2: prefix match ("Code Lite")
      expect(ranked[1].id).toBe('app_prefix_match');
      // Rank 3: substring match ("Visual Studio Code")
      expect(ranked[2].id).toBe('app_substring_match');
      // Rank 4: publisher match ("Terminal Tool" by "Code Corporation")
      expect(ranked[3].id).toBe('app_pub_match');
    });
  });

  describe('computeAvailabilityCounts Helper', () => {
    it('accurately computes aggregate availability counts', () => {
      const counts = computeAvailabilityCounts(sampleApps);
      expect(counts.total).toBe(4);
      expect(counts.available).toBe(2);
      expect(counts.missing).toBe(1);
      expect(counts.unverified).toBe(1);
      expect(counts.incomplete).toBe(0);
    });

    it('handles empty list cleanly', () => {
      const counts = computeAvailabilityCounts([]);
      expect(counts).toEqual({
        total: 0,
        available: 0,
        unverified: 0,
        incomplete: 0,
        missing: 0,
      });
    });
  });

  describe('ApplicationCatalog Rendering States', () => {
    it('renders loading state when autoDiscover is active and no initial applications are provided', () => {
      const html = ReactDOMServer.renderToString(
        <ApplicationCatalog autoDiscover={true} />
      );

      expect(html).toContain('Loading applications...');
      expect(html).toContain('role="status"');
      expect(html).toContain('loading-state');
    });

    it('renders empty state when catalog is empty and discovery completed', () => {
      const html = ReactDOMServer.renderToString(
        <ApplicationCatalog
          initialApplications={[]}
          autoDiscover={false}
        />
      );

      expect(html).toContain('No applications found.');
      expect(html).toContain('Try refreshing the application catalog.');
      expect(html).toContain('empty-state');
    });

    it('renders loaded catalog with application cards and summary metrics', () => {
      const html = ReactDOMServer.renderToString(
        <ApplicationCatalog
          initialApplications={sampleApps}
          initialScannedSources={['Registry (HKLM 64-bit)', 'Start Menu (User)']}
          autoDiscover={false}
        />
      );

      // Cards rendered
      expect(html).toContain('Visual Studio Code');
      expect(html).toContain('Google Chrome');
      expect(html).toContain('Broken Application');
      expect(html).toContain('Legacy Utility');

      // Metric summary bar rendered
      expect(html).toContain('Total Discovered: <strong>4</strong>');
      expect(html).toContain('Available: <strong style="color:#58a6ff">2</strong>');
      expect(html).toContain('Missing Target: <strong style="color:#f85149">1</strong>');

      // Scanned sources pills rendered
      expect(html).toContain('Registry (HKLM 64-bit)');
      expect(html).toContain('Start Menu (User)');

      // Search toolbar rendered
      expect(html).toContain('Search Applications');
      expect(html).toContain('Refresh Catalog');
    });

    it('renders warning banner when partialFailures are present', () => {
      const html = ReactDOMServer.renderToString(
        <ApplicationCatalog
          initialApplications={sampleApps}
          initialPartialFailures={['Access denied scanning protected system directory']}
          autoDiscover={false}
        />
      );

      expect(html).toContain('Partial Discovery Notice:');
      expect(html).toContain('Access denied scanning protected system directory');
      expect(html).toContain('warning-banner');
    });

    it('confirms security invariants: no arbitrary execution paths in rendered DOM', () => {
      const html = ReactDOMServer.renderToString(
        <ApplicationCatalog
          initialApplications={sampleApps}
          autoDiscover={false}
        />
      );

      // Verify no paths or command lines are present in the catalog HTML
      expect(html).not.toContain('C:\\Windows');
      expect(html).not.toContain('cmd.exe');
      expect(html).not.toContain('powershell.exe');
      expect(html).not.toContain('executablePath');
      expect(html).not.toContain('args');
      expect(html).not.toContain('cwd');
      expect(html).not.toContain('env');
    });
  });
});
