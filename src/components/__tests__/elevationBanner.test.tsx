// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup, waitFor } from '@testing-library/react';
import * as tauriCore from '@tauri-apps/api/core';
import App from '../../App';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

function mockBackend(elevated: boolean | Error) {
  vi.mocked(tauriCore.invoke).mockImplementation(async (cmd: string) => {
    if (cmd === 'is_running_elevated') {
      if (elevated instanceof Error) throw elevated;
      return elevated;
    }
    if (cmd === 'discover_applications')
      return { applications: [], scannedSources: [], partialFailures: [], totalDiscovered: 0 };
    if (cmd === 'get_launch_history') return [];
    if (cmd === 'get_system_metrics')
      return {
        cpu: { brandName: 'Test CPU', logicalCores: 8, architecture: 'x86_64', utilizationPercent: 12 },
        memory: { totalBytes: 16e9, availableBytes: 8e9, usedBytes: 8e9, usedPercent: 50 },
        storage: { drives: [] },
        system: { uptimeSeconds: 3600, appUptimeSeconds: 10, osName: 'Windows 11', architecture: 'x86_64' },
        sampledAtMs: 0,
      };
    return undefined;
  });
}

// The sentence spans a <strong> and plain text, so match on the alert's full text.
const banner = () =>
  screen.queryAllByRole('alert').find((el) => /DeskPilot is running as administrator/.test(el.textContent ?? '')) ??
  null;

describe('administrator warning', () => {
  afterEach(() => {
    cleanup();
    vi.mocked(tauriCore.invoke).mockReset();
  });

  it('is shown when DeskPilot runs elevated', async () => {
    mockBackend(true);
    render(<App />);
    await waitFor(() => expect(banner()).not.toBeNull());
    expect(banner()?.textContent).toMatch(/also get administrator rights/);
  });

  it('is hidden when DeskPilot runs with standard rights', async () => {
    mockBackend(false);
    render(<App />);
    await waitFor(() =>
      expect(vi.mocked(tauriCore.invoke).mock.calls.some(([c]) => c === 'is_running_elevated')).toBe(true),
    );
    expect(banner()).toBeNull();
  });

  it('is hidden when the check fails', async () => {
    mockBackend(new Error('unavailable'));
    render(<App />);
    await waitFor(() =>
      expect(vi.mocked(tauriCore.invoke).mock.calls.some(([c]) => c === 'is_running_elevated')).toBe(true),
    );
    expect(banner()).toBeNull();
  });
});
