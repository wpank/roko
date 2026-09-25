'use client';

import SubNav from '@/components/layout/SubNav';
import type { SubNavTab } from '@/components/layout/SubNav';
import { ErrorBoundary } from '@/components/ErrorBoundary';

const TABS: SubNavTab[] = [
  { label: 'Roster',   href: '/agents' },
  { label: 'Chat',     href: '/agents/chat' },
  { label: 'Topology', href: '/agents/topology' },
];

export default function AgentsLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex flex-col h-full min-h-0">
      <SubNav tabs={TABS} />
      <div className="flex-1 overflow-y-auto overflow-x-hidden">
        <ErrorBoundary name="Agents">
          {children}
        </ErrorBoundary>
      </div>
    </div>
  );
}
