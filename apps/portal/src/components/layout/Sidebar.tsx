'use client';

import { useState } from 'react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import {
  LayoutDashboard,
  Briefcase,
  Bot,
  Brain,
  Eye,
  Activity,
  Settings,
  ChevronLeft,
  ChevronRight,
} from 'lucide-react';
import { clsx } from 'clsx';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type ConnectionStatus = 'connected' | 'connecting' | 'disconnected';

interface NavItem {
  label: string;
  href: string;
  icon: React.ComponentType<{ size?: number; strokeWidth?: number; className?: string; 'aria-hidden'?: boolean | 'true' | 'false' }>;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const NAV_ITEMS: NavItem[] = [
  { label: 'Overview',      href: '/',             icon: LayoutDashboard },
  { label: 'Work',          href: '/work',         icon: Briefcase },
  { label: 'Agents',        href: '/agents',       icon: Bot },
  { label: 'Intelligence',  href: '/intelligence', icon: Brain },
  { label: 'Observe',       href: '/observe',      icon: Eye },
  { label: 'Providers',     href: '/providers',    icon: Activity },
  { label: 'Settings',      href: '/settings',     icon: Settings },
];

const STATUS_CONFIG: Record<ConnectionStatus, { label: string; color: string }> = {
  connected:    { label: 'Connected',     color: 'bg-[var(--sage)]' },
  connecting:   { label: 'Connecting...', color: 'bg-[var(--warning)]' },
  disconnected: { label: 'Disconnected',  color: 'bg-[var(--accent-error)]' },
};

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

interface NavItemRowProps {
  item: NavItem;
  isActive: boolean;
  collapsed: boolean;
}

function NavItemRow({ item, isActive, collapsed }: NavItemRowProps) {
  const Icon = item.icon;

  return (
    <Link
      href={item.href}
      title={collapsed ? item.label : undefined}
      className={clsx(
        // Layout
        'relative flex items-center gap-3 rounded-sm px-3',
        collapsed ? 'h-10 w-10 justify-center' : 'h-9',
        // Active left border
        isActive && 'before:absolute before:inset-y-1 before:left-0 before:w-[2px] before:rounded-full before:bg-[var(--rose-glow)]',
        // Background
        isActive
          ? 'bg-[var(--bg-highlight)]'
          : 'bg-transparent hover:bg-[var(--bg-highlight)]',
        // Text
        isActive
          ? 'text-[var(--rose-glow)]'
          : 'text-[var(--text-muted)] hover:text-[var(--text-strong)]',
        // Transition — hover is fast (80ms), layout collapse shares the parent 200ms
        'transition-colors duration-[80ms] ease-[var(--ease-out)]',
        // Remove default focus ring noise; keep accessible outline
        'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
      )}
    >
      <Icon
        size={15}
        strokeWidth={isActive ? 2 : 1.5}
        // Icon shrinks slightly on collapse but stays sharp
        className="shrink-0"
        aria-hidden
      />
      {!collapsed && (
        <span
          className={clsx(
            'truncate font-[var(--font-mono)] text-[var(--text-sm)] leading-none tracking-[var(--tracking-wide)]',
            'transition-opacity duration-[80ms]',
          )}
        >
          {item.label}
        </span>
      )}
    </Link>
  );
}

// ---------------------------------------------------------------------------
// Connection indicator
// ---------------------------------------------------------------------------

interface ConnectionIndicatorProps {
  status: ConnectionStatus;
  collapsed: boolean;
}

function ConnectionIndicator({ status, collapsed }: ConnectionIndicatorProps) {
  const cfg = STATUS_CONFIG[status];

  return (
    <div
      className={clsx(
        'flex items-center gap-2',
        collapsed ? 'justify-center' : 'px-3',
      )}
    >
      {/* Pulsing dot */}
      <span className="relative flex h-2 w-2 shrink-0">
        {status === 'connecting' && (
          <span
            className={clsx(
              'absolute inline-flex h-full w-full animate-ping rounded-full opacity-60',
              cfg.color,
            )}
          />
        )}
        <span
          className={clsx(
            'relative inline-flex h-2 w-2 rounded-full',
            cfg.color,
          )}
        />
      </span>

      {!collapsed && (
        <span
          className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] leading-none"
        >
          {cfg.label}
        </span>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Sidebar
// ---------------------------------------------------------------------------

interface SidebarProps {
  /**
   * Connection status passed in from the parent layout so the Sidebar stays
   * a pure display component that can be tested without a live WebSocket.
   */
  connectionStatus?: ConnectionStatus;
}

export default function Sidebar({ connectionStatus = 'connected' }: SidebarProps) {
  const pathname = usePathname();
  const [collapsed, setCollapsed] = useState(false);

  // Exact-match for root, prefix-match for every other route.
  function isActive(href: string): boolean {
    if (href === '/') return pathname === '/';
    return pathname === href || pathname.startsWith(href + '/');
  }

  return (
    <aside
      style={{
        width: collapsed ? 56 : 200,
        // 200ms ease drives the sidebar width change
        transition: 'width 200ms ease',
      }}
      className={clsx(
        'relative flex flex-col shrink-0',
        'bg-[var(--bg-raised)]',
        // Vertical separator on the right
        'border-r border-r-[var(--text-ghost)]',
        // Full viewport height minus header (48px) and status bar (24px)
        'h-[calc(100vh-72px)]',
        // Clip overflow during collapse animation
        'overflow-hidden',
      )}
      aria-label="Main navigation"
    >
      {/* Nav items */}
      <nav className="flex flex-col gap-0.5 p-2 flex-1 overflow-y-auto overflow-x-hidden">
        {NAV_ITEMS.map((item) => (
          <NavItemRow
            key={item.href}
            item={item}
            isActive={isActive(item.href)}
            collapsed={collapsed}
          />
        ))}
      </nav>

      {/* Bottom controls */}
      <div className="flex flex-col gap-3 p-2 pb-3 border-t border-t-[var(--text-ghost)]">
        {/* Connection status */}
        <ConnectionIndicator status={connectionStatus} collapsed={collapsed} />

        {/* Collapse toggle */}
        <button
          type="button"
          onClick={() => setCollapsed((c) => !c)}
          title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          className={clsx(
            'flex items-center justify-center rounded-sm',
            collapsed ? 'h-8 w-8 mx-auto' : 'h-7 w-full gap-2',
            'text-[var(--text-faint)] hover:text-[var(--text-muted)]',
            'hover:bg-[var(--bg-highlight)]',
            'transition-colors duration-[80ms]',
            'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
          )}
          aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        >
          {collapsed ? (
            <ChevronRight size={13} strokeWidth={1.5} aria-hidden />
          ) : (
            <>
              <ChevronLeft size={13} strokeWidth={1.5} aria-hidden />
              <span className="font-[var(--font-mono)] text-[var(--text-xs)] leading-none">
                Collapse
              </span>
            </>
          )}
        </button>
      </div>
    </aside>
  );
}
