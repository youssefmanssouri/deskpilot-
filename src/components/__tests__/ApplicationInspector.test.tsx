// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/react';
import ReactDOMServer from 'react-dom/server';
import { ApplicationInspector } from '../ApplicationInspector';
import type { ApplicationDto } from '../../lib/ipc';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('ApplicationInspector Component', () => {
  const sampleApp: ApplicationDto = {
    id: 'app_sample_1234567890abcdef',
    displayName: 'Windows Terminal',
    publisher: 'Microsoft Corporation',
    version: '1.18.3181.0',
    sources: ['Registry (HKLM 64-bit)', 'Start Menu (Common)'],
    availability: 'Available',
    isInstalled: true,
  };

  const missingTargetApp: ApplicationDto = {
    id: 'app_missing_1234567890abcdef',
    displayName: 'Orphaned App',
    publisher: 'Unknown Publisher',
    version: undefined,
    sources: ['Start Menu (User)'],
    availability: 'MissingTarget',
    isInstalled: false,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  describe('Empty State', () => {
    it('renders empty selection state when application prop is null', () => {
      render(<ApplicationInspector application={null} />);

      expect(screen.getByTestId('application-inspector-empty')).toBeDefined();
      expect(screen.getByText('No app selected')).toBeDefined();
      expect(
        screen.getByText(/Select an app on the left/i)
      ).toBeDefined();
    });

    it('renders empty state via SSR cleanly', () => {
      const html = ReactDOMServer.renderToString(<ApplicationInspector application={null} />);
      expect(html).toContain('No app selected');
      expect(html).toContain('application-inspector-empty');
    });
  });

  describe('Selected Application Presentation', () => {
    it('renders application details and metadata ledger accurately', () => {
      render(<ApplicationInspector application={sampleApp} />);

      expect(screen.getByTestId('application-inspector')).toBeDefined();
      expect(screen.getByText('Windows Terminal')).toBeDefined();
      expect(screen.getAllByText('Microsoft Corporation').length).toBeGreaterThanOrEqual(1);
      expect(screen.getByText('ID: app_sample_1234567890abcdef')).toBeDefined();
      expect(screen.getByText('v1.18.3181.0')).toBeDefined();
      expect(screen.getByText('Installed Application')).toBeDefined();
      expect(screen.getAllByText('Ready').length).toBeGreaterThanOrEqual(1);
      expect(screen.getByText('Registry (HKLM 64-bit)')).toBeDefined();
      expect(screen.getByText('Start Menu (Common)')).toBeDefined();
    });

    it('displays fallback when publisher or version are omitted', () => {
      const minimalApp: ApplicationDto = {
        id: 'app_min',
        displayName: 'Minimal Utility',
        sources: [],
        availability: 'Unverified',
        isInstalled: true,
      };

      render(<ApplicationInspector application={minimalApp} />);

      expect(screen.getByText('Minimal Utility')).toBeDefined();
      expect(screen.getAllByText('Unknown Publisher').length).toBeGreaterThanOrEqual(1);
      expect(screen.getByText('Unspecified')).toBeDefined();
      expect(screen.getAllByText('Unverified').length).toBeGreaterThanOrEqual(1);
    });

    it('renders launch button enabled for Available application', () => {
      render(<ApplicationInspector application={sampleApp} />);

      const launchBtn = screen.getByRole('button', { name: 'Open Windows Terminal' });
      expect((launchBtn as HTMLButtonElement).disabled).toBe(false);
      expect(screen.getByText(/Press/i)).toBeDefined();
      expect(screen.getByText('Enter')).toBeDefined();
    });

    it('disables launch button when availability is MissingTarget', () => {
      render(<ApplicationInspector application={missingTargetApp} />);

      const launchBtn = screen.getByRole('button', { name: 'Open Orphaned App' });
      expect((launchBtn as HTMLButtonElement).disabled).toBe(true);
      expect(screen.getAllByText('Missing').length).toBeGreaterThanOrEqual(1);
      expect(screen.getByText('Unregistered')).toBeDefined();
    });

    it('delegates launch request when primary button is clicked', () => {
      const handleLaunchRequest = vi.fn();
      render(
        <ApplicationInspector
          application={sampleApp}
          onLaunchRequest={handleLaunchRequest}
        />
      );

      const launchBtn = screen.getByRole('button', { name: 'Open Windows Terminal' });
      fireEvent.click(launchBtn);

      expect(handleLaunchRequest).toHaveBeenCalledTimes(1);
    });

    it('strictly preserves least privilege by exposing no filesystem paths, args, or registry keys', () => {
      const html = ReactDOMServer.renderToString(<ApplicationInspector application={sampleApp} />);

      expect(html).not.toContain('C:\\');
      expect(html).not.toContain('.exe');
      expect(html).not.toContain('HKEY_');
      expect(html).not.toContain('arguments');
      expect(html).not.toContain('workingDirectory');
    });
  });
});
