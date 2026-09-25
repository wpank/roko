'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { ErrorBoundary } from '@/components/ErrorBoundary';
import {
  LayoutGrid,
  FileCode,
  Key,
  Blocks,
  Puzzle,
  Rss,
  Monitor,
  Wifi,
} from 'lucide-react';
import { clsx } from 'clsx';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface SideNavItem {
  label: string;
  href: string;
  icon: React.ComponentType<{ size?: number; strokeWidth?: number; className?: string; 'aria-hidden'?: boolean | 'true' | 'false' }>;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const NAV_ITEMS: SideNavItem[] = [
  { label: 'Overview',   href: '/settings',            icon: LayoutGrid },
  { label: 'Config',     href: '/settings/config',     icon: FileCode },
  { label: 'Keys',       href: '/settings/keys',       icon: Key },
  { label: 'MCP',        href: '/settings/mcp',        icon: Blocks },
  { label: 'Plugins',    href: '/settings/plugins',    icon: Puzzle },
  { label: 'Feeds',      href: '/settings/feeds',      icon: Rss },
  { label: 'System',     href: '/settings/system',     icon: Monitor },
  { label: 'Connection', href: '/settings/connection', icon: Wifi },
];

// ---------------------------------------------------------------------------
// NavLink
// ---------------------------------------------------------------------------

interface NavLinkProps {
  item: SideNavItem;
  active: boolean;
}

function NavLink({ item, active }: NavLinkProps) {
  const Icon = item.icon;

  return (
    <Link
      href={item.href}
      aria-current={active ? 'page' : undefined}
      className={clsx(
        // Layout
        'relative flex items-center gap-3 h-9 px-3',
        // Active rose left border
        active
          ? 'before:absolute before:inset-y-1 before:left-0 before:w-[2px] before:bg-[var(--rose-glow)]'
          : '',
        // Background
        active
          ? 'bg-[var(--bg-highlight)]'
          : 'bg-transparent hover:bg-[var(--bg-highlight)]',
        // Text colour
        active
          ? 'text-[var(--rose-glow)]'
          : 'text-[var(--text-muted)] hover:text-[var(--text-strong)]',
        // Misc
        'transition-colors duration-[80ms] ease-[var(--ease-out)]',
        'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
      )}
    >
      <Icon
        size={14}
        strokeWidth={active ? 2 : 1.5}
        className="shrink-0"
        aria-hidden
      />
      <span className="font-[var(--font-mono)] text-[var(--text-sm)] leading-none tracking-[var(--tracking-wide)] truncate">
        {item.label}
      </span>
    </Link>
  );
}

// ---------------------------------------------------------------------------
// SettingsLayout
// ---------------------------------------------------------------------------

export default function SettingsLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();

  function isActive(href: string): boolean {
    if (href === '/settings') return pathname === '/settings';
    return pathname === href || pathname.startsWith(href + '/');
  }

  return (
    <div className="flex h-full min-h-0 overflow-hidden">
      {/* ---- Vertical sidebar (200px) ---- */}
      <aside
        className={clsx(
          'flex flex-col shrink-0 w-[200px]',
          'bg-[var(--bg-raised)]',
          'border-r border-r-[var(--text-ghost)]',
          'overflow-y-auto overflow-x-hidden',
        )}
        aria-label="Settings navigation"
      >
        {/* Section header */}
        <div className="px-3 py-3 shrink-0 border-b border-b-[var(--text-ghost)]">
          <span
            className={clsx(
              'font-[var(--font-mono)] text-[var(--text-xs)]',
              'text-[var(--text-faint)] tracking-[var(--tracking-widest)] uppercase leading-none',
            )}
          >
            Settings
          </span>
        </div>

        {/* Nav links */}
        <nav className="flex flex-col gap-0.5 p-2 flex-1">
          {NAV_ITEMS.map((item) => (
            <NavLink key={item.href} item={item} active={isActive(item.href)} />
          ))}
        </nav>
      </aside>

      {/* ---- Right content area (flex-grow) ---- */}
      <div className="flex-1 min-w-0 overflow-y-auto overflow-x-hidden">
        <ErrorBoundary name="Settings">
          {children}
        </ErrorBoundary>
      </div>
    </div>
  );
}
