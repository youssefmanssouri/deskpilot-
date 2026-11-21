import { useEffect, useRef, useState } from 'react';
import { APP_ICON_SIZE, deskpilotApi } from '../lib/ipc';
import { getAppInitials } from './appInitials';

/**
 * Loads real application icons from the Rust side.
 *
 * Results (including "no icon") are cached for the session, identical requests share one
 * call, and at most MAX_PARALLEL requests run at once so a long Apps list does not flood
 * the backend.
 */
const MAX_PARALLEL = 4;
const cache = new Map<string, Promise<Uint8ClampedArray<ArrayBuffer> | null>>();
const queue: Array<() => void> = [];
let running = 0;

function runNext() {
  while (running < MAX_PARALLEL && queue.length > 0) {
    const start = queue.shift()!;
    running += 1;
    start();
  }
}

export function loadAppIcon(applicationId: string): Promise<Uint8ClampedArray<ArrayBuffer> | null> {
  const cached = cache.get(applicationId);
  if (cached) return cached;
  const promise = new Promise<Uint8ClampedArray<ArrayBuffer> | null>((resolve) => {
    queue.push(() => {
      deskpilotApi
        .getApplicationIcon(applicationId)
        .then(resolve, () => resolve(null))
        .finally(() => {
          running -= 1;
          runNext();
        });
    });
    runNext();
  });
  cache.set(applicationId, promise);
  return promise;
}

/** Test hook: forget cached icons. */
export function clearAppIconCache() {
  cache.clear();
}

interface AppIconProps {
  applicationId: string;
  displayName: string;
  /** The tile class of the place it is shown (e.g. `app-row-tile`), which sets its size. */
  className: string;
}

/**
 * An application's real icon, or its initials until the icon loads or when it has none.
 * Drawn on a canvas from raw pixels, so the Content Security Policy needs no `data:` images.
 */
export function AppIcon({ applicationId, displayName, className }: AppIconProps) {
  const [pixels, setPixels] = useState<Uint8ClampedArray<ArrayBuffer> | null>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    let active = true;
    setPixels(null);
    loadAppIcon(applicationId).then((result) => {
      if (active) setPixels(result);
    });
    return () => {
      active = false;
    };
  }, [applicationId]);

  useEffect(() => {
    const ctx = pixels ? canvasRef.current?.getContext('2d') : null;
    if (!pixels || !ctx || typeof ImageData === 'undefined') return;
    ctx.putImageData(new ImageData(pixels, APP_ICON_SIZE, APP_ICON_SIZE), 0, 0);
  }, [pixels]);

  return (
    <span className={`${className}${pixels ? ' has-icon' : ''}`} aria-hidden="true">
      {pixels ? (
        <canvas
          ref={canvasRef}
          className="app-icon-canvas"
          width={APP_ICON_SIZE}
          height={APP_ICON_SIZE}
          data-testid="app-icon-canvas"
        />
      ) : (
        getAppInitials(displayName)
      )}
    </span>
  );
}
