/**
 * Small, generic React hooks for the roko portal.
 *
 * Each hook is self-contained and has no dependencies on portal-specific
 * state or stores, making them safe to import anywhere.
 *
 * These are client-side hooks — components that use them must be marked
 * `'use client'` in Next.js App Router.
 */

'use client';

import { useState, useEffect, useCallback } from 'react';

// ---------------------------------------------------------------------------
// useLocalStorage
// ---------------------------------------------------------------------------

/**
 * Persist a value in `localStorage` and keep it in sync with React state.
 *
 * The initial value is read from `localStorage` on the first render (client
 * only). On the server the `defaultValue` is returned and no storage access
 * is attempted.
 *
 * @param key          — `localStorage` key.
 * @param defaultValue — Value to use when no stored entry exists.
 * @returns A `[value, setValue]` tuple.  `setValue` writes to both state and
 *          `localStorage` atomically.
 *
 * @example
 * const [theme, setTheme] = useLocalStorage('portal-theme', 'dark');
 */
export function useLocalStorage<T>(
  key: string,
  defaultValue: T,
): [T, (value: T) => void] {
  const [storedValue, setStoredValue] = useState<T>(() => {
    if (typeof window === 'undefined') return defaultValue;
    try {
      const item = window.localStorage.getItem(key);
      return item !== null ? (JSON.parse(item) as T) : defaultValue;
    } catch {
      return defaultValue;
    }
  });

  const setValue = useCallback(
    (value: T) => {
      try {
        setStoredValue(value);
        if (typeof window !== 'undefined') {
          window.localStorage.setItem(key, JSON.stringify(value));
        }
      } catch {
        // Ignore write errors (quota exceeded, private browsing, etc.).
      }
    },
    [key],
  );

  return [storedValue, setValue];
}

// ---------------------------------------------------------------------------
// useMediaQuery
// ---------------------------------------------------------------------------

/**
 * Reactively track whether a CSS media query matches.
 *
 * Returns `false` during SSR and on the initial client render before the
 * query has been evaluated, then immediately syncs with the actual match
 * result via `matchMedia`.
 *
 * @param query — A valid CSS media query string, e.g. `"(max-width: 768px)"`.
 *
 * @example
 * const isMobile = useMediaQuery('(max-width: 768px)');
 */
export function useMediaQuery(query: string): boolean {
  const [matches, setMatches] = useState<boolean>(() => {
    if (typeof window === 'undefined') return false;
    return window.matchMedia(query).matches;
  });

  useEffect(() => {
    if (typeof window === 'undefined') return;

    const mediaQueryList = window.matchMedia(query);
    setMatches(mediaQueryList.matches);

    const handler = (event: MediaQueryListEvent) => {
      setMatches(event.matches);
    };

    // `addEventListener` is preferred over the deprecated `addListener`.
    mediaQueryList.addEventListener('change', handler);
    return () => {
      mediaQueryList.removeEventListener('change', handler);
    };
  }, [query]);

  return matches;
}

// ---------------------------------------------------------------------------
// useClock
// ---------------------------------------------------------------------------

/**
 * Return the current wall-clock time as a formatted `"HH:MM:SS"` string,
 * updated once per second.
 *
 * The interval is cleared on unmount to prevent memory leaks.
 *
 * @example
 * const time = useClock(); // "14:05:09"
 */
export function useClock(): string {
  const getTimeString = (): string => {
    const now = new Date();
    const hh = String(now.getHours()).padStart(2, '0');
    const mm = String(now.getMinutes()).padStart(2, '0');
    const ss = String(now.getSeconds()).padStart(2, '0');
    return `${hh}:${mm}:${ss}`;
  };

  const [time, setTime] = useState<string>(getTimeString);

  useEffect(() => {
    const id = setInterval(() => {
      setTime(getTimeString());
    }, 1_000);
    return () => clearInterval(id);
  }, []);

  return time;
}
