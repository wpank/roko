'use client';

/**
 * useSelection — read the plan/task selection from the URL and write it back.
 *
 * The URL is the single source of truth. There is no router navigation:
 * `window.history.replaceState` is used so the back button leaves the app
 * rather than stepping through every panel interaction. Next 15 keeps
 * `useSearchParams()` in sync with `replaceState`.
 *
 * Components using this hook must render under a <Suspense> boundary (required
 * for Next.js static export compatibility).
 */

import { useSearchParams } from 'next/navigation';
import { parseSelection, selectionSearch } from './selection';
import type { Selection } from './selection';

export type UseSelectionResult = Selection & {
  /** Apply a partial update to the selection. */
  select(patch: Partial<Selection>): void;
};

export function useSelection(): UseSelectionResult {
  const searchParams = useSearchParams();

  // Parse the current selection from the live search params.
  const sel = parseSelection(searchParams.toString());

  function select(patch: Partial<Selection>): void {
    const next = selectionSearch(searchParams.toString(), patch);
    // replaceState (not pushState) keeps the back button leaving the app.
    window.history.replaceState(null, '', location.pathname + next);
  }

  return { ...sel, select };
}
