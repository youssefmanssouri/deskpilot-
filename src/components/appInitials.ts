/**
 * Two-letter monogram shown while an app's real icon loads, or when it has none
 * (see AppIcon). Example: "Visual Studio Code" -> "VS".
 */
export function getAppInitials(displayName: string): string {
  const words = displayName.split(/[\s\-_.]+/).filter((w) => /[A-Za-z0-9]/.test(w));
  const letters = words.slice(0, 2).map((w) => w.match(/[A-Za-z0-9]/)![0]);
  return (letters.join('') || '?').toUpperCase();
}
