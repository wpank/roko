'use client';

import SubNav from '@/components/layout/SubNav';
import type { SubNavTab } from '@/components/layout/SubNav';
import { ErrorBoundary } from '@/components/ErrorBoundary';

const TABS: SubNavTab[] = [
  { label: 'Log',     href: '/observe' },
  { label: 'Signals', href: '/observe/signals' },
  { label: 'Errors',  href: '/observe/errors' },
  { label: 'Safety',  href: '/observe/safety' },
  { label: 'Git',     href: '/observe/git' },
  { label: 'Cost',    href: '/observe/cost' },
];

export default function ObserveLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex flex-col h-full min-h-0">
      <SubNav tabs={TABS} />
      <div className="flex-1 overflow-y-auto overflow-x-hidden">
        <ErrorBoundary name="Observe">
          {children}
        </ErrorBoundary>
      </div>
    </div>
  );
}
