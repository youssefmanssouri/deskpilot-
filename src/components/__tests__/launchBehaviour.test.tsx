// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup, act } from '@testing-library/react';
import * as tauriCore from '@tauri-apps/api/core';
import { ApplicationCatalog } from '../ApplicationCatalog';
import { canLaunchWithClick, canLaunchWithKeyboard } from '../launchPolicy';
import { describeLaunchError } from '../launchErrorDescription';
import { shortenPathMiddle } from '../formatPath';
import type { ApplicationDto } from '../../lib/ipc';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const apps: ApplicationDto[] = [
  {
    id: 'app_00000000000000000000000000000011',
    displayName: 'Legacy Utility',
    publisher: 'Acme Corp',
    sources: ['Registry (HKLM 32-bit)'],
    availability: 'Unverified',
    isInstalled: true,
  },
  {
    id: 'app_00000000000000000000000000000012',
    displayName: 'Visual Studio Code',
    publisher: 'Microsoft Corporation',
    version: '1.95.0',
    sources: ['Registry (HKCU User)'],
    availability: 'Available',
    isInstalled: true,
  },
];

const launchCalls = () =>
  vi.mocked(tauriCore.invoke).mock.calls.filter(([cmd]) => cmd === 'launch_application');

describe('launch policy', () => {
  it('lets the keyboard launch only verified applications', () => {
    expect(canLaunchWithKeyboard({ availability: 'Available' })).toBe(true);
    expect(canLaunchWithKeyboard({ availability: 'Unverified' })).toBe(false);
    expect(canLaunchWithKeyboard({ availability: 'IncompleteMetadata' })).toBe(false);
    expect(canLaunchWithKeyboard({ availability: 'MissingTarget' })).toBe(false);
  });

  it('lets an explicit click launch everything except a missing target', () => {
    expect(canLaunchWithClick({ availability: 'Available' })).toBe(true);
    expect(canLaunchWithClick({ availability: 'Unverified' })).toBe(true);
    expect(canLaunchWithClick({ availability: 'IncompleteMetadata' })).toBe(true);
    expect(canLaunchWithClick({ availability: 'MissingTarget' })).toBe(false);
  });
});

describe('describeLaunchError', () => {
  it('maps each sanitized error category to its own guidance', () => {
    expect(describeLaunchError('PERMISSION_DENIED').title).toMatch(/blocked/i);
    expect(describeLaunchError('NOT_FOUND').title).toMatch(/could not be found/i);
    expect(describeLaunchError('UNAVAILABLE').title).toMatch(/could not be found/i);
    expect(describeLaunchError('INVALID_INPUT').title).toMatch(/cannot be launched safely/i);
  });

  it('falls back to a generic message for unknown or missing codes', () => {
    expect(describeLaunchError('OPERATION_FAILED').title).toBe('Windows could not start this app');
    expect(describeLaunchError(undefined).title).toBe('Windows could not start this app');
  });
});

describe('shortenPathMiddle', () => {
  it('keeps the drive and the last two segments of a long path', () => {
    expect(
      shortenPathMiddle('C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe')
    ).toBe('C:\\…\\Application\\chrome.exe');
  });

  it('leaves short paths unchanged', () => {
    expect(shortenPathMiddle('C:\\Windows\\explorer.exe')).toBe('C:\\Windows\\explorer.exe');
    expect(shortenPathMiddle('C:\\Programs\\VSCode\\Code.exe')).toBe(
      'C:\\Programs\\VSCode\\Code.exe'
    );
  });
});

describe('ApplicationCatalog launch behaviour', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it('does not launch an unverified selection on Enter, and never falls back to another app', () => {
    render(<ApplicationCatalog initialApplications={apps} autoDiscover={false} />);
    const search = screen.getByRole('searchbox', { name: /search applications/i });

    // First (selected) app is Unverified; a verified app is also in the list.
    fireEvent.keyDown(search, { key: 'Enter' });

    expect(launchCalls()).toHaveLength(0);
    expect(screen.getByText(/only opens verified apps/i)).toBeDefined();
  });

  it('does not launch an unverified app when Enter is pressed on its focused card', () => {
    render(<ApplicationCatalog initialApplications={apps} autoDiscover={false} />);
    const card = screen.getByTestId(`app-card-${apps[0].id}`);

    fireEvent.keyDown(card, { key: 'Enter' });

    expect(launchCalls()).toHaveLength(0);
  });

  it('still launches an unverified app from an explicit button click', async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValueOnce({
      applicationId: apps[0].id,
      processId: 4242,
    });
    render(<ApplicationCatalog initialApplications={apps} autoDiscover={false} />);

    fireEvent.click(screen.getByRole('button', { name: `Open ${apps[0].displayName}` }));

    await waitFor(() => {
      expect(tauriCore.invoke).toHaveBeenCalledWith('launch_application', {
        applicationId: apps[0].id,
      });
    });
  });

  it('shows the failure reason inline, keeps it visible, and retries from the panel', async () => {
    vi.useFakeTimers();
    vi.mocked(tauriCore.invoke).mockRejectedValueOnce({
      code: 'PERMISSION_DENIED',
      message: 'Access to execute target binary was denied by the operating system.',
    });
    render(<ApplicationCatalog initialApplications={apps} autoDiscover={false} />);

    // Select the verified app and launch it with Enter.
    const search = screen.getByRole('searchbox', { name: /search applications/i });
    fireEvent.change(search, { target: { value: 'visual' } });
    await act(async () => {
      fireEvent.keyDown(search, { key: 'Enter' });
    });

    const panel = screen.getByTestId('launch-error-panel');
    expect(panel.getAttribute('role')).toBe('alert');
    expect(panel.textContent).toContain('Windows or DeskPilot blocked this launch');
    expect(panel.textContent).toContain('denied by the operating system');

    // The explanation must not disappear on its own.
    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(screen.getByTestId('launch-error-panel')).toBeDefined();

    // "Try again" launches the same application again.
    vi.mocked(tauriCore.invoke).mockResolvedValueOnce({
      applicationId: apps[1].id,
      processId: 5151,
    });
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: /try opening visual studio code again/i }));
    });
    expect(launchCalls()).toHaveLength(2);
    expect(launchCalls()[1][1]).toEqual({ applicationId: apps[1].id });
    expect(screen.queryByTestId('launch-error-panel')).toBeNull();
  });
});
