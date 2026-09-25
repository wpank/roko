'use client';

import SubNav from '@/components/layout/SubNav';
import type { SubNavTab } from '@/components/layout/SubNav';
import { ErrorBoundary } from '@/components/ErrorBoundary';

const TABS: SubNavTab[] = [
  { label: 'Overview',    href: '/intelligence' },
  { label: 'Knowledge',   href: '/intelligence/knowledge' },
  { label: 'Dreams',      href: '/intelligence/dreams' },
  { label: 'Learning',    href: '/intelligence/learning' },
  { label: 'Experiments', href: '/intelligence/experiments' },
];

export default function IntelligenceLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex flex-col h-full min-h-0">
      <SubNav tabs={TABS} />
      <div className="flex-1 overflow-y-auto overflow-x-hidden">
        <ErrorBoundary name="Intelligence">
          {children}
        </ErrorBoundary>
      </div>
    </div>
  );
}
