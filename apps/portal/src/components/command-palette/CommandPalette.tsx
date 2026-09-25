'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import { Command } from 'cmdk';
import { useRouter } from 'next/navigation';
import { AnimatePresence, motion } from 'framer-motion';
import {
  LayoutDashboard,
  Briefcase,
  ListTodo,
  Layers,
  Inbox,
  Bot,
  MessageSquare,
  GitBranch,
  Brain,
  Database,
  Sparkles,
  GraduationCap,
  TestTube2,
  Eye,
  Zap,
  AlertTriangle,
  Shield,
  GitCommit,
  DollarSign,
  Cpu,
  BarChart2,
  Settings,
  KeyRound,
  Plug,
  Rss,
  Server,
  Wifi,
  Terminal,
  Stethoscope,
  RefreshCw,
  FilePlus,
  Play,
  Clock,
} from 'lucide-react';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type ActionKind = 'navigate' | 'action';

interface CommandItem {
  id: string;
  label: string;
  group: string;
  kind: ActionKind;
  icon: React.ComponentType<{ size?: number; strokeWidth?: number; className?: string }>;
  href?: string;
  shortcut?: string;
  onSelect?: () => void;
}

interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
}

// ---------------------------------------------------------------------------
// Recent pages — stored in localStorage under this key
// ---------------------------------------------------------------------------

const RECENT_KEY = 'roko:cmd-recent';
const RECENT_MAX = 10;

function getRecent(): string[] {
  if (typeof window === 'undefined') return [];
  try {
    const raw = localStorage.getItem(RECENT_KEY);
    return raw ? (JSON.parse(raw) as string[]) : [];
  } catch {
    return [];
  }
}

function pushRecent(id: string): void {
  if (typeof window === 'undefined') return;
  try {
    const prev = getRecent().filter((x) => x !== id);
    const next = [id, ...prev].slice(0, RECENT_MAX);
    localStorage.setItem(RECENT_KEY, JSON.stringify(next));
  } catch {
    // localStorage may be unavailable (private browsing, full quota, etc.)
  }
}

// ---------------------------------------------------------------------------
// All navigate items — single source of truth
// ---------------------------------------------------------------------------

const NAV_ITEMS: Omit<CommandItem, 'group' | 'kind'>[] = [
  // Top-level
  { id: 'nav:overview',                label: 'Overview',                      icon: LayoutDashboard,  href: '/',                             shortcut: 'G O' },
  { id: 'nav:work',                    label: 'Work',                          icon: Briefcase,        href: '/work' },
  { id: 'nav:work:plans',              label: 'Work > Plans',                  icon: ListTodo,         href: '/work/plans' },
  { id: 'nav:work:jobs',               label: 'Work > Jobs',                   icon: Layers,           href: '/work/jobs' },
  { id: 'nav:work:backlog',            label: 'Work > Backlog',                icon: Inbox,            href: '/work/backlog' },
  { id: 'nav:agents',                  label: 'Agents',                        icon: Bot,              href: '/agents' },
  { id: 'nav:agents:chat',             label: 'Agents > Chat',                 icon: MessageSquare,    href: '/agents/chat' },
  { id: 'nav:agents:topology',         label: 'Agents > Topology',             icon: GitBranch,        href: '/agents/topology' },
  { id: 'nav:intelligence',            label: 'Intelligence',                  icon: Brain,            href: '/intelligence' },
  { id: 'nav:intelligence:knowledge',  label: 'Intelligence > Knowledge',      icon: Database,         href: '/intelligence/knowledge' },
  { id: 'nav:intelligence:dreams',     label: 'Intelligence > Dreams',         icon: Sparkles,         href: '/intelligence/dreams' },
  { id: 'nav:intelligence:learning',   label: 'Intelligence > Learning',       icon: GraduationCap,    href: '/intelligence/learning' },
  { id: 'nav:intelligence:experiments',label: 'Intelligence > Experiments',    icon: TestTube2,        href: '/intelligence/experiments' },
  { id: 'nav:observe',                 label: 'Observe',                       icon: Eye,              href: '/observe' },
  { id: 'nav:observe:signals',         label: 'Observe > Signals',             icon: Zap,              href: '/observe/signals' },
  { id: 'nav:observe:errors',          label: 'Observe > Errors',              icon: AlertTriangle,    href: '/observe/errors' },
  { id: 'nav:observe:safety',          label: 'Observe > Safety',              icon: Shield,           href: '/observe/safety' },
  { id: 'nav:observe:git',             label: 'Observe > Git',                 icon: GitCommit,        href: '/observe/git' },
  { id: 'nav:observe:cost',            label: 'Observe > Cost',                icon: DollarSign,       href: '/observe/cost' },
  { id: 'nav:providers',              label: 'Providers',                      icon: Cpu,              href: '/providers' },
  { id: 'nav:providers:cost',          label: 'Providers > Cost',              icon: BarChart2,        href: '/providers/cost' },
  { id: 'nav:providers:models',        label: 'Providers > Models',            icon: Cpu,              href: '/providers/models' },
  { id: 'nav:settings',               label: 'Settings',                       icon: Settings,         href: '/settings' },
  { id: 'nav:settings:config',         label: 'Settings > Config',             icon: Settings,         href: '/settings/config' },
  { id: 'nav:settings:keys',           label: 'Settings > Keys',               icon: KeyRound,         href: '/settings/keys' },
  { id: 'nav:settings:mcp',            label: 'Settings > MCP',                icon: Plug,             href: '/settings/mcp' },
  { id: 'nav:settings:plugins',        label: 'Settings > Plugins',            icon: Plug,             href: '/settings/plugins' },
  { id: 'nav:settings:feeds',          label: 'Settings > Feeds',              icon: Rss,              href: '/settings/feeds' },
  { id: 'nav:settings:system',         label: 'Settings > System',             icon: Server,           href: '/settings/system' },
  { id: 'nav:settings:connection',     label: 'Settings > Connection',         icon: Wifi,             href: '/settings/connection' },
];

// Build a lookup by id for Recent resolution
const NAV_BY_ID = new Map(NAV_ITEMS.map((item) => [item.id, item]));

// ---------------------------------------------------------------------------
// Action items
// ---------------------------------------------------------------------------

// These are resolved at render time so the router reference is captured
// in the closure where it is safe.
function buildActionItems(
  router: ReturnType<typeof useRouter>,
  onClose: () => void,
): CommandItem[] {
  return [
    {
      id: 'action:run-prompt',
      label: 'Run Prompt...',
      group: 'Actions',
      kind: 'action',
      icon: Play,
      onSelect: () => {
        onClose();
        // Navigate to overview first, then fire the event so the modal opens
        // on the correct page whether already on '/' or navigating there.
        router.push('/');
        // Slight delay allows Next.js route transition to settle before the
        // overview page registers its event listener.
        setTimeout(() => {
          window.dispatchEvent(new CustomEvent('roko:open-run-prompt'));
        }, 120);
      },
    },
    {
      id: 'action:new-plan',
      label: 'New Plan...',
      group: 'Actions',
      kind: 'action',
      icon: FilePlus,
      onSelect: () => {
        onClose();
        router.push('/work/plans');
      },
    },
    {
      id: 'action:open-terminal',
      label: 'Open Terminal',
      group: 'Actions',
      kind: 'action',
      icon: Terminal,
      shortcut: '⌃`',
      onSelect: () => {
        onClose();
        // Terminal is surfaced via a drawer; fire a custom event so the
        // AppShell can react without a direct prop dependency.
        window.dispatchEvent(new CustomEvent('roko:open-terminal'));
      },
    },
    {
      id: 'action:diagnostics',
      label: 'Run Diagnostics',
      group: 'Actions',
      kind: 'action',
      icon: Stethoscope,
      onSelect: () => {
        onClose();
        router.push('/settings/system');
      },
    },
    {
      id: 'action:refresh',
      label: 'Refresh Data',
      group: 'Actions',
      kind: 'action',
      icon: RefreshCw,
      shortcut: '⌘R',
      onSelect: () => {
        onClose();
        router.refresh();
      },
    },
  ];
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

interface ItemRowProps {
  item: CommandItem;
}

function ItemRow({ item }: ItemRowProps) {
  const Icon = item.icon;

  return (
    <Command.Item
      key={item.id}
      value={item.label}
      onSelect={item.onSelect}
      className="group flex items-center gap-2.5 px-3 h-9 cursor-pointer select-none outline-none
        text-[var(--text-muted)]
        data-[selected=true]:bg-[var(--bg-highlight)]
        data-[selected=true]:text-[var(--text-strong)]"
    >
      {/* Icon */}
      <Icon
        size={16}
        strokeWidth={1.5}
        className="shrink-0 text-[var(--text-muted)] group-data-[selected=true]:text-[var(--rose-dim)]"
        aria-hidden
      />

      {/* Label */}
      <span className="flex-1 truncate font-[var(--font-mono)] text-[var(--text-sm)] leading-none">
        {item.label}
      </span>

      {/* Optional shortcut hint */}
      {item.shortcut && (
        <kbd
          className="shrink-0 font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)]
            leading-none tracking-[var(--tracking-wide)] ml-2"
        >
          {item.shortcut}
        </kbd>
      )}
    </Command.Item>
  );
}

// ---------------------------------------------------------------------------
// Main overlay
// ---------------------------------------------------------------------------

export function CommandPalette({ open, onClose }: CommandPaletteProps) {
  const router = useRouter();
  const inputRef = useRef<HTMLInputElement>(null);
  const [recentIds, setRecentIds] = useState<string[]>([]);

  // Load recent list when the palette opens
  useEffect(() => {
    if (open) {
      setRecentIds(getRecent());
      // Auto-focus the input on the next frame
      requestAnimationFrame(() => inputRef.current?.focus());
    }
  }, [open]);

  // Build the action items once — router/onClose are stable references
  const actionItems = buildActionItems(router, onClose);

  // Wrap an item's onSelect to also push to recent history if it's a nav item
  const makeSelectHandler = useCallback(
    (item: CommandItem) => () => {
      if (item.kind === 'navigate' && item.href) {
        pushRecent(item.id);
        onClose();
        router.push(item.href);
      } else if (item.onSelect) {
        item.onSelect();
      }
    },
    [router, onClose],
  );

  // Resolve recent items — only show items that still exist in NAV_BY_ID
  const recentItems: CommandItem[] = recentIds
    .map((id) => NAV_BY_ID.get(id))
    .filter((x): x is NonNullable<typeof x> => x !== undefined)
    .map((raw) => ({
      ...raw,
      group: 'Recent',
      kind: 'navigate' as ActionKind,
      onSelect: makeSelectHandler({ ...raw, group: 'Recent', kind: 'navigate' }),
    }));

  // All nav items with select handlers
  const navItems: CommandItem[] = NAV_ITEMS.map((raw) => ({
    ...raw,
    group: 'Navigate',
    kind: 'navigate',
    onSelect: makeSelectHandler({ ...raw, group: 'Navigate', kind: 'navigate' }),
  }));

  // All action items with select handlers (already have onSelect)
  const actions: CommandItem[] = actionItems.map((item) => ({
    ...item,
    onSelect: makeSelectHandler(item),
  }));

  // Dismiss on backdrop click
  const onBackdropClick = useCallback(() => onClose(), [onClose]);

  // Dismiss on Escape
  const onKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    },
    [onClose],
  );

  return (
    <AnimatePresence>
      {open && (
        // Backdrop
        <motion.div
          key="cmd-backdrop"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.15, ease: 'easeOut' }}
          className="fixed inset-0 z-[var(--z-modal)] flex justify-center"
          style={{ paddingTop: '20vh' }}
          onClick={onBackdropClick}
          aria-hidden="true"
        >
          {/* Panel */}
          <motion.div
            key="cmd-panel"
            initial={{ opacity: 0, scale: 0.97 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.97 }}
            transition={{ duration: 0.15, ease: 'easeOut' }}
            onClick={(e) => e.stopPropagation()}
            onKeyDown={onKeyDown}
            style={{
              width: '100%',
              maxWidth: 620,
              marginInline: 16,
              alignSelf: 'flex-start',
            }}
            aria-hidden={false}
          >
            <Command
              className="flex flex-col overflow-hidden
                bg-[var(--bg-glass)] backdrop-blur-md
                border border-[var(--text-ghost)]
                shadow-[var(--shadow-lg)]"
              label="Command palette"
              shouldFilter={true}
              loop
            >
              {/* ---- Search input ---- */}
              <div className="flex items-center gap-2 px-3 border-b border-[var(--text-ghost)]">
                <Command.Input
                  ref={inputRef}
                  placeholder="Type a command..."
                  className="flex-1 h-11 bg-transparent
                    font-[var(--font-mono)] text-[var(--text-base)] text-[var(--text-strong)]
                    placeholder:text-[var(--text-faint)]
                    border-0 outline-none
                    leading-none"
                  aria-label="Search commands"
                />
              </div>

              {/* ---- Results list ---- */}
              <Command.List
                className="overflow-y-auto overscroll-contain"
                style={{ maxHeight: 380 }}
              >
                {/* Empty state */}
                <Command.Empty className="px-3 py-6 text-center font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-faint)]">
                  No results found.
                </Command.Empty>

                {/* --- Recent --- */}
                {recentItems.length > 0 && (
                  <Command.Group
                    heading={
                      <span className="block px-3 pt-3 pb-1 font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] uppercase tracking-[var(--tracking-wider)] leading-none select-none">
                        Recent
                      </span>
                    }
                  >
                    {recentItems.map((item) => (
                      <ItemRow key={item.id} item={item} />
                    ))}
                  </Command.Group>
                )}

                {/* --- Navigate --- */}
                <Command.Group
                  heading={
                    <span className="block px-3 pt-3 pb-1 font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] uppercase tracking-[var(--tracking-wider)] leading-none select-none">
                      Navigate
                    </span>
                  }
                >
                  {navItems.map((item) => (
                    <ItemRow key={item.id} item={item} />
                  ))}
                </Command.Group>

                {/* --- Actions --- */}
                <Command.Group
                  heading={
                    <span className="block px-3 pt-3 pb-1 font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] uppercase tracking-[var(--tracking-wider)] leading-none select-none">
                      Actions
                    </span>
                  }
                >
                  {actions.map((item) => (
                    <ItemRow key={item.id} item={item} />
                  ))}
                </Command.Group>
              </Command.List>

              {/* ---- Footer hint ---- */}
              <div className="flex items-center gap-3 px-3 h-8 border-t border-[var(--text-ghost)]">
                <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)] leading-none">
                  <kbd>↑↓</kbd> navigate
                </span>
                <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)] leading-none">
                  <kbd>↵</kbd> select
                </span>
                <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)] leading-none">
                  <kbd>Esc</kbd> dismiss
                </span>
              </div>
            </Command>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
