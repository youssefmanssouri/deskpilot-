/**
 * Shortens a Windows path for display by keeping the drive or root and the last two
 * segments, e.g. `C:\Program Files\Google\Chrome\Application\chrome.exe`
 * becomes `C:\…\Application\chrome.exe`. The full path should stay available in a tooltip.
 */
export function shortenPathMiddle(path: string, keepTail = 2): string {
  const parts = path.split('\\');
  if (parts.length <= keepTail + 2) {
    return path;
  }
  return [parts[0], '…', ...parts.slice(-keepTail)].join('\\');
}
