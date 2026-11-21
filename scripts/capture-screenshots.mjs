// Captures real screenshots of the DeskPilot app on Windows.
//
// Starts the built app with WebView2's remote debugging port open, connects Playwright
// to the real webview, and drives the real UI: real discovery, real icons, real launches.
// Run on a throwaway machine (the GitHub Actions runner): it opens a few installed apps
// so "Recent apps" has real entries.
//
//   node scripts/capture-screenshots.mjs <path-to-deskpilot.exe> <output-dir>

import { spawn, execFileSync } from 'node:child_process';
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { chromium } from 'playwright-core';

const [exe, outDir = 'screenshots'] = process.argv.slice(2);
if (!exe) {
  console.error('usage: node scripts/capture-screenshots.mjs <deskpilot.exe> [output-dir]');
  process.exit(2);
}
mkdirSync(outDir, { recursive: true });

const PORT = 9222;
const VIEW = { width: 1280, height: 800 };
// Apps to open for "Recent apps", in order of preference; the runner image decides
// which exist. Only apps DeskPilot marks Ready are used.
const PREFERRED = [
  'Google Chrome', 'Mozilla Firefox', 'Microsoft Edge', 'Visual Studio Code',
  'PowerShell 7', 'Git Bash', '7-Zip File Manager', 'Docker Desktop', 'Notepad++',
];

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function waitForDebugger() {
  for (let i = 0; i < 120; i++) {
    try {
      const res = await fetch(`http://127.0.0.1:${PORT}/json/version`);
      if (res.ok) return;
    } catch {}
    await sleep(500);
  }
  throw new Error('WebView2 debugging port did not open');
}

async function findPage(browser, test) {
  for (let i = 0; i < 60; i++) {
    for (const context of browser.contexts()) {
      for (const page of context.pages()) if (test(page.url())) return page;
    }
    await sleep(500);
  }
  throw new Error('page not found');
}

async function prepare(page, scheme) {
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Emulation.setDeviceMetricsOverride', {
    width: VIEW.width, height: VIEW.height, deviceScaleFactor: 2, mobile: false,
  });
  await cdp.send('Emulation.setEmulatedMedia', {
    features: [{ name: 'prefers-color-scheme', value: scheme }],
  });
  await sleep(600);
}

async function shot(page, name) {
  const path = join(outDir, name);
  await page.screenshot({ path });
  console.log('saved', path);
}

const child = spawn(exe, [], {
  env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PORT}` },
  stdio: 'inherit',
});

let browser;
try {
  await waitForDebugger();
  browser = await chromium.connectOverCDP(`http://127.0.0.1:${PORT}`);
  const main = await findPage(browser, (u) => !u.includes('window=launcher') && u !== 'about:blank');
  await prepare(main, 'light');

  // Apps: wait for discovery and for real icons to be drawn.
  await main.getByRole('button', { name: 'Apps', exact: true }).click();
  const rows = main.locator('[data-testid^="app-card-"]');
  await rows.first().waitFor({ timeout: 60_000 });
  await main.waitForFunction(() => document.querySelectorAll('canvas.app-icon-canvas').length >= 3, null, { timeout: 60_000 });
  const total = await rows.count();
  const icons = await main.locator('canvas.app-icon-canvas').count();
  console.log(`apps listed: ${total}, real icons drawn: ${icons}`);

  // Open a few installed apps so Home shows real recent apps.
  const opened = [];
  for (const name of PREFERRED) {
    if (opened.length === 3) break;
    const row = rows.filter({ has: main.locator('.app-card-title', { hasText: new RegExp(`^${name}$`) }) });
    if ((await row.count()) === 0) continue;
    if (!(await row.first().textContent())?.includes('Ready')) continue;
    await row.first().click();
    const open = main.getByRole('button', { name: `Open ${name}`, exact: true });
    if (!(await open.isEnabled().catch(() => false))) continue;
    await open.click();
    await sleep(2500);
    opened.push(name);
  }
  console.log('opened for recent apps:', opened.join(', ') || '(none)');

  // Select the first opened app (or the first row) for the Apps screenshot.
  const pick = opened[0]
    ? rows.filter({ has: main.locator('.app-card-title', { hasText: new RegExp(`^${opened[0]}$`) }) }).first()
    : rows.first();
  await pick.click();
  await main.locator('#app-workspace-content').evaluate((el) => el.scrollTo(0, 0)).catch(() => {});
  await sleep(800);
  await shot(main, 'real_apps.png');

  await main.getByRole('button', { name: 'Home', exact: true }).click();
  await sleep(1500);
  await shot(main, 'real_home.png');

  await main.getByRole('button', { name: 'Settings', exact: true }).click();
  await sleep(800);
  await shot(main, 'real_settings.png');

  await prepare(main, 'dark');
  await main.getByRole('button', { name: 'Home', exact: true }).click();
  await sleep(1500);
  await shot(main, 'real_home_dark.png');

  // Quick launcher: its page is loaded while hidden; render it at its own size.
  try {
    const launcher = await findPage(browser, (u) => u.includes('window=launcher'));
    const cdp = await launcher.context().newCDPSession(launcher);
    await cdp.send('Emulation.setDeviceMetricsOverride', { width: 640, height: 440, deviceScaleFactor: 2, mobile: false });
    await cdp.send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: 'light' }] });
    await launcher.reload();
    const input = launcher.getByRole('combobox').or(launcher.locator('input')).first();
    await input.waitFor({ timeout: 15_000 });
    await input.fill(opened[0] ? opened[0].split(' ')[0] : 'win');
    await launcher.waitForFunction(() => document.querySelectorAll('canvas.app-icon-canvas').length >= 1, null, { timeout: 20_000 });
    await sleep(800);
    await shot(launcher, 'real_launcher.png');
  } catch (err) {
    console.log('launcher screenshot skipped:', err.message);
  }
} finally {
  await browser?.close().catch(() => {});
  child.kill();
  try {
    execFileSync('taskkill', ['/F', '/IM', 'deskpilot.exe'], { stdio: 'ignore' });
  } catch {}
}
