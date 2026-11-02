import { describe, it, expect } from 'vitest';
import ReactDOMServer from 'react-dom/server';
import { ApplicationCard, getAvailabilityBadgeClass } from '../ApplicationCard';
import type { ApplicationDto } from '../../lib/ipc';

describe('ApplicationCard Component', () => {
  const sampleApp: ApplicationDto = {
    id: 'app_a1b2c3d4e5f6789012345678abcdef01',
    displayName: 'DeskPilot Test Utility',
    publisher: 'DeskPilot Authors',
    version: '1.2.3',
    sources: ['Registry (HKLM 64-bit)', 'Start Menu (User)'],
    availability: 'Available',
    isInstalled: true,
  };

  it('renders safe application metadata correctly', () => {
    const html = ReactDOMServer.renderToString(
      <ApplicationCard application={sampleApp} />
    );

    // Metadata present
    expect(html).toContain('DeskPilot Test Utility');
    expect(html).toContain('DeskPilot Authors');
    expect(html).toContain('v1.2.3');
    expect(html).toContain('Registry (HKLM 64-bit)');
    expect(html).toContain('Start Menu (User)');
    expect(html).toContain('Available');
    expect(html).toContain('badge-available');

    // Accessibility attributes present
    expect(html).toContain('aria-labelledby="app-card-title-app_a1b2c3d4e5f6789012345678abcdef01"');
    expect(html).toContain('id="app-card-title-app_a1b2c3d4e5f6789012345678abcdef01"');

    // Renders LaunchButton with applicationId
    expect(html).toContain('aria-label="Launch application app_a1b2c3d4e5f6789012345678abcdef01"');
    expect(html).toContain('Launch');
  });

  it('renders "Unknown Publisher" when publisher is undefined', () => {
    const appWithoutPublisher: ApplicationDto = {
      ...sampleApp,
      publisher: undefined,
    };

    const html = ReactDOMServer.renderToString(
      <ApplicationCard application={appWithoutPublisher} />
    );

    expect(html).toContain('Unknown Publisher');
  });

  it('omits version display when version is undefined', () => {
    const appWithoutVersion: ApplicationDto = {
      ...sampleApp,
      version: undefined,
    };

    const html = ReactDOMServer.renderToString(
      <ApplicationCard application={appWithoutVersion} />
    );

    expect(html).not.toContain('app-card-version');
  });

  it('disables LaunchButton when availability is MissingTarget', () => {
    const missingApp: ApplicationDto = {
      ...sampleApp,
      availability: 'MissingTarget',
    };

    const html = ReactDOMServer.renderToString(
      <ApplicationCard application={missingApp} />
    );

    expect(html).toContain('badge-missing');
    expect(html).toContain('disabled=""');
  });

  it('maps each availability state to its corresponding badge class', () => {
    expect(getAvailabilityBadgeClass('Available')).toBe('badge-available');
    expect(getAvailabilityBadgeClass('Unverified')).toBe('badge-unverified');
    expect(getAvailabilityBadgeClass('MissingTarget')).toBe('badge-missing');
    expect(getAvailabilityBadgeClass('IncompleteMetadata')).toBe('badge-incomplete');
  });

  it('strictly adheres to security boundary by exposing no paths, args, or shell', () => {
    const html = ReactDOMServer.renderToString(
      <ApplicationCard application={sampleApp} />
    );

    // Verify forbidden attributes/concepts are absent from rendered markup
    expect(html).not.toContain('executablePath');
    expect(html).not.toContain('C:\\');
    expect(html).not.toContain('powershell');
    expect(html).not.toContain('cmd.exe');
    expect(html).not.toContain('args');
    expect(html).not.toContain('env');
    expect(html).not.toContain('cwd');
  });
});
