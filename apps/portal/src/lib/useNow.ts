'use client';

import { useEffect, useState } from 'react';

/**
 * useNow — a wall clock that ticks only while something is running.
 *
 * Returns Date.now() captured at mount. While `active` is true it re-reads the
 * clock the moment it becomes active and then every `intervalMs` via
 * setInterval. While inactive it holds its last value and keeps no timer, so
 * an idle page costs nothing.
 *
 * Typical usage: pass the "something is running" boolean from your component
 * and use the returned timestamp to compute elapsed times that keep moving
 * between events.
 */
export function useNow(active: boolean, intervalMs = 1_000): number {
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    if (!active) return;

    // Catch up to the current time the moment we become active.
    setNow(Date.now());

    // Keep ticking every intervalMs while active.
    const id = setInterval(() => {
      setNow(Date.now());
    }, intervalMs);

    // Clear the interval when we become inactive or on unmount.
    return () => clearInterval(id);
  }, [active, intervalMs]);

  return now;
}
