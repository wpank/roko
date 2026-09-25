'use client';

import SubNav from '@/components/layout/SubNav';
import type { SubNavTab } from '@/components/layout/SubNav';
import { ErrorBoundary } from '@/components/ErrorBoundary';

const TABS: SubNavTab[] = [
  { label: 'Active',   href: '/work' },
  { label: 'Plans',    href: '/work/plans' },
  { label: 'Editor',   href: '/work/editor' },
  { label: 'Jobs',     href: '/work/jobs' },
  { label: 'Backlog',  href: '/work/backlog' },
];

export default function WorkLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex flex-col h-full min-h-0">
      <SubNav tabs={TABS} />
      <div className="flex-1 overflow-y-auto overflow-x-hidden">
        <ErrorBoundary name="Work">
          {children}
        </ErrorBoundary>
      </div>
    </div>
  );
}
