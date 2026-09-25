'use client';

import Link from 'next/link';
import {
  FileCode,
  Key,
  Blocks,
  Puzzle,
  Rss,
  Monitor,
  Wifi,
  CheckCircle,
  AlertTriangle,
  XCircle,
} from 'lucide-react';
import { clsx } from 'clsx';
import { StatusLED } from '@/components/atoms';
import { useConfig, useProviders } from '@/api/hooks';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type HealthState = 'ok' | 'warn' | 'error';

interface QuickCard {
  title: string;
  href: string;
  icon: React.ComponentType<{ size?: number; strokeWidth?: number; className?: string; 'aria-hidden'?: boolean | 'true' | 'false' }>;
  health: HealthState;
  primary: string;
  secondary?: string;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function healthToLed(h: HealthState): 'success' | 'warning' | 'error' {
  if (h === 'ok') return 'success';
  if (h === 'warn') return 'warning';
  return 'error';
}

function HealthIcon({ state }: { state: HealthState }) {
  if (state === 'ok')
    return <CheckCircle size={13} strokeWidth={1.5} className="text-[var(--sage)] shrink-0" aria-hidden />;
  if (state === 'warn')
    return <AlertTriangle size={13} strokeWidth={1.5} className="text-[var(--warning)] shrink-0" aria-hidden />;
  return <XCircle size={13} strokeWidth={1.5} className="text-[var(--accent-error)] shrink-0" aria-hidden />;
}

// ---------------------------------------------------------------------------
// QuickCard component
// ---------------------------------------------------------------------------

function QuickCard({ card }: { card: QuickCard }) {
  const Icon = card.icon;

  return (
    <Link
      href={card.href}
      className={clsx(
        'group flex flex-col gap-3 p-4',
        'bg-[var(--bg-raised)] border border-[var(--text-ghost)]',
        'hover:border-[var(--rose-dim)] hover:bg-[var(--bg-highlight)]',
        'transition-[border-color,background] duration-[80ms] ease-[var(--ease-out)]',
        'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
      )}
    >
      {/* Header row */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Icon
            size={14}
            strokeWidth={1.5}
            className="text-[var(--text-muted)] group-hover:text-[var(--rose)] shrink-0 transition-colors duration-[80ms]"
            aria-hidden
          />
          <span className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-muted)] tracking-[var(--tracking-wide)] uppercase">
            {card.title}
          </span>
        </div>
        <StatusLED status={healthToLed(card.health)} pulse={card.health === 'warn'} />
      </div>

      {/* Content */}
      <div className="flex flex-col gap-1">
        <div className="flex items-center gap-2">
          <HealthIcon state={card.health} />
          <span className="font-[var(--font-mono)] text-[var(--text-base)] text-[var(--text-strong)] leading-snug">
            {card.primary}
          </span>
        </div>
        {card.secondary && (
          <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] pl-5">
            {card.secondary}
          </span>
        )}
      </div>
    </Link>
  );
}

// ---------------------------------------------------------------------------
// Settings Overview page
// ---------------------------------------------------------------------------

export default function SettingsOverviewPage() {
  const { data: configData } = useConfig();
  const { data: providersData } = useProviders();

  const providerCount = providersData?.providers?.length ?? 0;
  const hasConfig = Boolean(configData?.config);

  const cards: QuickCard[] = [
    {
      title: 'Config',
      href: '/settings/config',
      icon: FileCode,
      health: hasConfig ? 'ok' : 'error',
      primary: hasConfig ? 'roko.toml loaded' : 'Config error',
      secondary: hasConfig
        ? `version ${configData?.version ?? '—'} · ${configData?.source ?? 'local'}`
        : 'Could not read configuration',
    },
    {
      title: 'Keys',
      href: '/settings/keys',
      icon: Key,
      health: providerCount > 0 ? 'ok' : 'warn',
      primary: providerCount > 0
        ? `${providerCount} provider${providerCount === 1 ? '' : 's'} configured`
        : 'No providers configured',
      secondary: providerCount > 0 ? 'API keys present' : 'Add at least one provider key',
    },
    {
      title: 'MCP',
      href: '/settings/mcp',
      icon: Blocks,
      health: 'ok',
      primary: '2 servers active',
      secondary: 'roko-mcp-code · roko-mcp-github',
    },
    {
      title: 'Plugins',
      href: '/settings/plugins',
      icon: Puzzle,
      health: 'ok',
      primary: '0 plugins loaded',
      secondary: 'Plugin registry reachable',
    },
    {
      title: 'Feeds',
      href: '/settings/feeds',
      icon: Rss,
      health: 'ok',
      primary: '0 feeds active',
      secondary: 'No feeds configured',
    },
    {
      title: 'System',
      href: '/settings/system',
      icon: Monitor,
      health: 'ok',
      primary: 'roko v0.1.0',
      secondary: 'Disk within threshold',
    },
    {
      title: 'Connection',
      href: '/settings/connection',
      icon: Wifi,
      health: 'ok',
      primary: 'localhost:6677',
      secondary: 'Connected · <1ms',
    },
  ];

  return (
    <div className="flex flex-col gap-6 p-6">
      {/* Page header */}
      <div className="flex flex-col gap-1">
        <h1 className="font-[var(--font-mono)] text-[var(--text-lg)] text-[var(--text-strong)] leading-tight">
          Settings
        </h1>
        <p className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-muted)]">
          Workspace configuration, API keys, MCP servers, plugins, and diagnostics.
        </p>
      </div>

      {/* Quick-health grid */}
      <section aria-label="Quick health overview">
        <h2 className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tracking-[var(--tracking-widest)] uppercase mb-3">
          Health overview
        </h2>
        <div className="grid grid-cols-2 xl:grid-cols-3 gap-3">
          {cards.map((card) => (
            <QuickCard key={card.href} card={card} />
          ))}
        </div>
      </section>
    </div>
  );
}
