// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, cleanup, waitFor } from '@testing-library/react';
import * as tauriCore from '@tauri-apps/api/core';
import { AppIcon, clearAppIconCache, loadAppIcon } from '../AppIcon';
import { APP_ICON_SIZE, deskpilotApi } from '../../lib/ipc';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const ICON_BYTES = APP_ICON_SIZE * APP_ICON_SIZE * 4;
const id = (n: number) => `app_${n.toString(16).padStart(32, '0')}`;
const iconCalls = () =>
  vi.mocked(tauriCore.invoke).mock.calls.filter(([c]) => c === 'get_application_icon');

describe('getApplicationIcon', () => {
  beforeEach(() => vi.mocked(tauriCore.invoke).mockReset());

  it('asks by catalog id and accepts a full 64x64 RGBA buffer', async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(new ArrayBuffer(ICON_BYTES));
    const pixels = await deskpilotApi.getApplicationIcon(id(1));
    expect(pixels?.length).toBe(ICON_BYTES);
    expect(tauriCore.invoke).toHaveBeenCalledWith('get_application_icon', { applicationId: id(1) });
  });

  it('treats an empty or wrong-sized buffer as no icon', async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(new ArrayBuffer(0));
    expect(await deskpilotApi.getApplicationIcon(id(1))).toBeNull();
    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(new ArrayBuffer(12));
    expect(await deskpilotApi.getApplicationIcon(id(1))).toBeNull();
  });
});

describe('AppIcon', () => {
  beforeEach(() => {
    vi.mocked(tauriCore.invoke).mockReset();
    clearAppIconCache();
  });
  afterEach(cleanup);

  it('shows the initials when the app has no icon', async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValue(new ArrayBuffer(0));
    render(<AppIcon applicationId={id(1)} displayName="Visual Studio Code" className="app-row-tile" />);
    await waitFor(() => expect(iconCalls()).toHaveLength(1));
    expect(screen.getByText('VS')).toBeTruthy();
    expect(screen.queryByTestId('app-icon-canvas')).toBeNull();
  });

  it('shows the initials when loading the icon fails', async () => {
    vi.mocked(tauriCore.invoke).mockRejectedValue({ code: 'INTERNAL_ERROR', message: 'x' });
    render(<AppIcon applicationId={id(1)} displayName="Slack" className="app-row-tile" />);
    await waitFor(() => expect(iconCalls()).toHaveLength(1));
    expect(screen.getByText('S')).toBeTruthy();
  });

  it('draws the real icon and drops the initials when one is returned', async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValue(new ArrayBuffer(ICON_BYTES));
    const { container } = render(
      <AppIcon applicationId={id(1)} displayName="Visual Studio Code" className="app-row-tile" />,
    );
    await waitFor(() => expect(screen.getByTestId('app-icon-canvas')).toBeTruthy());
    expect(screen.queryByText('VS')).toBeNull();
    expect(container.querySelector('.app-row-tile.has-icon')).not.toBeNull();
  });

  it('loads each icon once and runs at most four requests at a time', async () => {
    let inFlight = 0;
    let peak = 0;
    vi.mocked(tauriCore.invoke).mockImplementation(async () => {
      inFlight += 1;
      peak = Math.max(peak, inFlight);
      await new Promise((r) => setTimeout(r, 5));
      inFlight -= 1;
      return new ArrayBuffer(0);
    });
    const ids = Array.from({ length: 10 }, (_, i) => id(i + 1));
    await Promise.all([...ids, ...ids].map((appId) => loadAppIcon(appId)));
    expect(iconCalls()).toHaveLength(10);
    expect(peak).toBeLessThanOrEqual(4);
  });
});
