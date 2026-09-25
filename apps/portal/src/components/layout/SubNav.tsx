'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { clsx } from 'clsx';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface SubNavTab {
  label: string;
  href: string;
  /** Optional count badge shown next to the label. */
  badge?: number;
}

export interface SubNavProps {
  tabs: SubNavTab[];
}

// ---------------------------------------------------------------------------
// Badge pill
// ---------------------------------------------------------------------------

function Badge({ count }: { count: number }) {
  return (
    <span
      className={clsx(
        'inline-flex items-center justify-center',
        'min-w-[16px] h-4 px-1 rounded-full',
        'font-[var(--font-mono)] text-[10px] leading-none tabular-nums',
        'bg-[var(--bg-highlight)] text-[var(--text-faint)]',
        // Inherit active-state colour from the parent link via `currentColor`
        // (handled in the parent by toggling the text colour class)
      )}
      aria-label={`${count} items`}
    >
      {count > 99 ? '99+' : count}
    </span>
  );
}

// ---------------------------------------------------------------------------
// SubNav
// ---------------------------------------------------------------------------

export default function SubNav({ tabs }: SubNavProps) {
  const pathname = usePathname();

  function isActive(href: string): boolean {
    // The first tab is often the index route of a section (e.g. /agents → /agents),
    // so we match exactly; deeper tabs also accept prefix-match.
    return pathname === href || pathname.startsWith(href + '/');
  }

  return (
    <nav
      className={clsx(
        'flex items-end gap-0',
        'h-9 px-4 shrink-0',
        'bg-[var(--bg-raised)]',
        'border-b border-b-[var(--text-ghost)]',
        'overflow-x-auto overflow-y-hidden',
        // Hide the scrollbar on WebKit while keeping scroll behaviour.
        '[&::-webkit-scrollbar]:hidden',
        'scrollbar-none',
      )}
      aria-label="Section navigation"
    >
      {tabs.map((tab) => {
        const active = isActive(tab.href);

        return (
          <Link
            key={tab.href}
            href={tab.href}
            className={clsx(
              // Layout — sit inside the nav track, bottom-aligned
              'relative flex items-center gap-1.5 shrink-0',
              'h-full px-3',
              // Typography
              'font-[var(--font-mono)] text-[var(--text-xs)] leading-none tracking-[var(--tracking-wide)]',
              // Colour states
              active
                ? 'text-[var(--text-strong)]'
                : 'text-[var(--text-muted)] hover:text-[var(--text-strong)]',
              // Active rose underline — 2px, sits flush at the bottom of the nav
              active && [
                'after:absolute after:bottom-0 after:inset-x-0',
                'after:h-[2px] after:rounded-t-full',
                'after:bg-[var(--rose-glow)]',
                'after:shadow-[0_0_6px_var(--rose-glow)]',
              ],
              // Transition
              'transition-colors duration-[80ms] ease-[var(--ease-out)]',
              // Focus ring
              'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
            )}
            aria-current={active ? 'page' : undefined}
          >
            {tab.label}

            {typeof tab.badge === 'number' && tab.badge > 0 && (
              <Badge count={tab.badge} />
            )}
          </Link>
        );
      })}
    </nav>
  );
}
