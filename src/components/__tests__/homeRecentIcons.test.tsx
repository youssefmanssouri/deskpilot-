// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, cleanup, waitFor } from '@testing-library/react';
import * as tauriCore from '@tauri-apps/api/core';
import { HomeView, type HomeViewProps } from '../HomeView';
import { clearAppIconCache } from '../AppIcon';
import { APP_ICON_SIZE, type ApplicationDto } from '../../lib/ipc';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const ID = `app_${'1'.padStart(32, '0')}`;
const iconCalls = () =>
  vi.mocked(tauriCore.invoke).mock.calls.filter(([c]) => c === 'get_application_icon');

const code: ApplicationDto = {
  id: ID,
  displayName: 'Visual Studio Code',
  publisher: 'Microsoft',
  sources: ['Start Menu (User)'],
  availability: 'Available',
  isInstalled: true,
};

function props(over: Partial<HomeViewProps>): HomeViewProps {
  return {
    applications: [],
    hasDiscovered: false,
    history: [{ applicationId: ID, displayName: 'Visual Studio Code', launchedAtMs: 1_000 }],
    historyLoaded: true,
    historyError: null,
    activeLaunch: null,
    onOpenApp: () => {},
    onShowApp: () => {},
    onShowApps: () => {},
    onShowSystem: () => {},
    onClearHistory: () => {},
    nowMs: 60_000,
    ...over,
  };
}

describe('Home recent apps and real icons', () => {
  beforeEach(() => {
    clearAppIconCache();
    vi.mocked(tauriCore.invoke).mockReset();
    vi.mocked(tauriCore.invoke).mockImplementation(async (cmd: string) =>
      cmd === 'get_application_icon' ? new ArrayBuffer(APP_ICON_SIZE * APP_ICON_SIZE * 4) : undefined,
    );
  });
  afterEach(cleanup);

  it('waits for discovery before asking for an icon, then shows the real one', async () => {
    // History loads before discovery: the backend cannot know the executable yet, and a
    // "no icon" answer at this point used to be remembered for the whole session.
    const { rerender } = render(<HomeView {...props({})} />);
    expect(screen.getByText('VS')).toBeTruthy();
    expect(iconCalls()).toHaveLength(0);

    rerender(<HomeView {...props({ applications: [code], hasDiscovered: true })} />);
    await waitFor(() => expect(screen.getByTestId('app-icon-canvas')).toBeTruthy());
    expect(iconCalls()).toHaveLength(1);
  });

  it('keeps initials for a recent app that is no longer installed', async () => {
    render(<HomeView {...props({ hasDiscovered: true })} />);
    await Promise.resolve();
    expect(screen.getByText('VS')).toBeTruthy();
    expect(iconCalls()).toHaveLength(0);
  });
});
