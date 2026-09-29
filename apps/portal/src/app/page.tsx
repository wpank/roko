import { Suspense } from 'react';
import { Workspace } from '@/components/shell/Workspace';

/**
 * Root page — renders the Workspace client component.
 *
 * Workspace uses `useSearchParams()` (via `useSelection()`), which requires a
 * <Suspense> boundary for Next.js static export compatibility.
 */
export default function Page() {
  return (
    <Suspense fallback={null}>
      <Workspace />
    </Suspense>
  );
}
