'use client';

import SubNav from '@/components/layout/SubNav';
import type { SubNavTab } from '@/components/layout/SubNav';
import { ErrorBoundary } from '@/components/ErrorBoundary';

const TABS: SubNavTab[] = [
  { label: 'Monitor', href: '/providers' },
  { label: 'Cost',    href: '/providers/cost' },
  { label: 'Models',  href: '/providers/models' },
];

export default function ProvidersLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex flex-col h-full min-h-0">
      <SubNav tabs={TABS} />
      <div className="flex-1 overflow-y-auto overflow-x-hidden">
        <ErrorBoundary name="Providers">
          {children}
        </ErrorBoundary>
      </div>
    </div>
  );
}
