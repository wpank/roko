'use client';

import { useState } from 'react';
import { clsx } from 'clsx';
import { ChevronRight, X, ShieldCheck, Puzzle, Trash2, Shield } from 'lucide-react';
import { Button, Badge, StatusLED, Spinner } from '@/components/atoms';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type PluginTier = 1 | 2 | 3 | 4 | 5;
type PluginStatus = 'active' | 'disabled' | 'error';

interface PluginCapability {
  name: string;
  description: string;
}

interface PluginHook {
  name: string;
  event: string;
}

interface PluginToolDef {
  name: string;
  description: string;
}

interface Plugin {
  id: string;
  name: string;
  version: string;
  tier: PluginTier;
  status: PluginStatus;
  description: string;
  author: string;
  capabilities: PluginCapability[];
  hooks: PluginHook[];
  tools: PluginToolDef[];
  signed: boolean;
}

// ---------------------------------------------------------------------------
// Placeholder data
// ---------------------------------------------------------------------------

const SAMPLE_PLUGINS: Plugin[] = [
  {
    id: 'roko-plugin-linter',
    name: 'roko-plugin-linter',
    version: '0.3.1',
    tier: 2,
    status: 'active',
    description: 'Adds extra linting gates: ESLint, Biome, and Prettier integration for JS/TS.',
    author: 'roko team',
    signed: true,
    capabilities: [
      { name: 'gate:linter', description: 'Run lint gates in the 7-rung pipeline' },
      { name: 'tool:lint_file', description: 'Lint a single file on demand' },
    ],
    hooks: [
      { name: 'pre_gate', event: 'gate_started' },
      { name: 'post_gate', event: 'gate_result' },
    ],
    tools: [
      { name: 'lint_file', description: 'Run the configured linters against a path' },
      { name: 'lint_changed', description: 'Lint files changed since HEAD' },
    ],
  },
  {
    id: 'roko-plugin-metrics',
    name: 'roko-plugin-metrics',
    version: '0.1.0',
    tier: 3,
    status: 'disabled',
    description: 'Emits Prometheus-compatible metrics for gate pass rates and agent costs.',
    author: 'community',
    signed: false,
    capabilities: [
      { name: 'telemetry:metrics', description: 'Emit Prometheus gauge/counter metrics' },
    ],
    hooks: [
      { name: 'post_task', event: 'task_completed' },
      { name: 'on_gate', event: 'gate_result' },
    ],
    tools: [],
  },
];

// ---------------------------------------------------------------------------
// Tier badge
// ---------------------------------------------------------------------------

const tierLabel: Record<PluginTier, string> = {
  1: 'Core',
  2: 'Standard',
  3: 'Extended',
  4: 'Community',
  5: 'Experimental',
};

const tierVariant: Record<PluginTier, 'info' | 'success' | 'default' | 'warning' | 'error'> = {
  1: 'info',
  2: 'success',
  3: 'default',
  4: 'warning',
  5: 'error',
};

function TierBadge({ tier }: { tier: PluginTier }) {
  return (
    <Badge variant={tierVariant[tier]}>
      T{tier} {tierLabel[tier]}
    </Badge>
  );
}

// ---------------------------------------------------------------------------
// Plugin detail drawer
// ---------------------------------------------------------------------------

interface DrawerProps {
  plugin: Plugin;
  onClose: () => void;
}

function PluginDrawer({ plugin, onClose }: DrawerProps) {
  return (
    <div
      className={clsx(
        'fixed inset-y-0 right-0 w-[380px]',
        'flex flex-col',
        'bg-[var(--bg-raised)] border-l border-l-[var(--text-ghost)]',
        'shadow-[var(--shadow-lg)]',
        'z-[var(--z-overlay)]',
        'overflow-y-auto',
      )}
    >
      {/* Drawer header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-b-[var(--text-ghost)] shrink-0">
        <div className="flex items-center gap-2">
          <Puzzle size={14} strokeWidth={1.5} className="text-[var(--text-muted)]" aria-hidden />
          <span className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-strong)]">
            {plugin.name}
          </span>
        </div>
        <button
          type="button"
          onClick={onClose}
          className={clsx(
            'p-1 text-[var(--text-faint)] hover:text-[var(--text-muted)]',
            'transition-colors duration-[80ms]',
            'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
          )}
          aria-label="Close drawer"
        >
          <X size={14} strokeWidth={1.5} aria-hidden />
        </button>
      </div>

      {/* Drawer content */}
      <div className="flex flex-col gap-6 p-4">
        {/* Meta */}
        <div className="flex flex-col gap-2">
          <div className="flex items-center gap-2 flex-wrap">
            <TierBadge tier={plugin.tier} />
            {plugin.signed ? (
              <Badge variant="success">
                <ShieldCheck size={9} strokeWidth={2} className="mr-1" aria-hidden />
                signed
              </Badge>
            ) : (
              <Badge variant="warning">unsigned</Badge>
            )}
            <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]">
              v{plugin.version}
            </span>
          </div>
          <p className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)] leading-relaxed">
            {plugin.description}
          </p>
          <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]">
            Author: {plugin.author}
          </span>
        </div>

        {/* Capabilities */}
        {plugin.capabilities.length > 0 && (
          <section>
            <h3 className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tracking-[var(--tracking-widest)] uppercase mb-2">
              Capabilities
            </h3>
            <div className="flex flex-col gap-1">
              {plugin.capabilities.map((cap) => (
                <div key={cap.name} className="flex flex-col gap-0.5 px-2 py-1.5 border border-[var(--text-ghost)]">
                  <code className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--accent-cyan)]">
                    {cap.name}
                  </code>
                  <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]">
                    {cap.description}
                  </span>
                </div>
              ))}
            </div>
          </section>
        )}

        {/* Hook points */}
        {plugin.hooks.length > 0 && (
          <section>
            <h3 className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tracking-[var(--tracking-widest)] uppercase mb-2">
              Hook points
            </h3>
            <div className="flex flex-col gap-1">
              {plugin.hooks.map((hook) => (
                <div key={hook.name} className="flex items-center justify-between px-2 py-1.5 border border-[var(--text-ghost)]">
                  <code className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)]">
                    {hook.name}
                  </code>
                  <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]">
                    on {hook.event}
                  </span>
                </div>
              ))}
            </div>
          </section>
        )}

        {/* Tool definitions */}
        {plugin.tools.length > 0 && (
          <section>
            <h3 className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tracking-[var(--tracking-widest)] uppercase mb-2">
              Tool definitions
            </h3>
            <div className="flex flex-col gap-1">
              {plugin.tools.map((tool) => (
                <div key={tool.name} className="flex flex-col gap-0.5 px-2 py-1.5 border border-[var(--text-ghost)]">
                  <code className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--rose)]">
                    {tool.name}()
                  </code>
                  <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]">
                    {tool.description}
                  </span>
                </div>
              ))}
            </div>
          </section>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// PluginRow
// ---------------------------------------------------------------------------

interface PluginRowProps {
  plugin: Plugin;
  onToggle: (id: string) => void;
  onAudit: (id: string) => void;
  onRemove: (id: string) => void;
  onInspect: (plugin: Plugin) => void;
}

function PluginRow({ plugin, onToggle, onAudit, onRemove, onInspect }: PluginRowProps) {
  const isActive = plugin.status === 'active';

  return (
    <div
      className={clsx(
        'flex items-start gap-3 px-4 py-3',
        'border-b border-b-[var(--text-ghost)] last:border-b-0',
        'hover:bg-[var(--bg-highlight)]',
        'transition-colors duration-[80ms]',
      )}
    >
      {/* Status LED */}
      <div className="mt-1 shrink-0">
        <StatusLED
          status={plugin.status === 'active' ? 'success' : plugin.status === 'error' ? 'error' : 'offline'}
        />
      </div>

      {/* Info */}
      <div className="flex flex-col gap-1 flex-1 min-w-0">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-strong)]">
            {plugin.name}
          </span>
          <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]">
            v{plugin.version}
          </span>
          <TierBadge tier={plugin.tier} />
          {!plugin.signed && <Badge variant="warning">unsigned</Badge>}
        </div>
        <p className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] leading-relaxed truncate">
          {plugin.description}
        </p>
        <div className="flex items-center gap-3 mt-0.5">
          <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)]">
            {plugin.capabilities.length} cap · {plugin.hooks.length} hooks · {plugin.tools.length} tools
          </span>
        </div>
      </div>

      {/* Actions */}
      <div className="flex items-center gap-1 shrink-0">
        {/* Enable/Disable toggle */}
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onToggle(plugin.id)}
        >
          {isActive ? 'Disable' : 'Enable'}
        </Button>

        <Button
          variant="ghost"
          size="sm"
          onClick={() => onAudit(plugin.id)}
          title="Run plugin audit"
        >
          <Shield size={12} strokeWidth={1.5} aria-hidden />
          Audit
        </Button>

        {/* Expand to drawer */}
        <button
          type="button"
          onClick={() => onInspect(plugin)}
          title="Inspect plugin details"
          className={clsx(
            'flex items-center gap-1 px-2 py-1',
            'font-[var(--font-mono)] text-[var(--text-xs)]',
            'text-[var(--text-faint)] hover:text-[var(--text-muted)]',
            'border border-transparent hover:border-[var(--text-ghost)]',
            'transition-colors duration-[80ms]',
            'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
          )}
        >
          <ChevronRight size={12} strokeWidth={1.5} aria-hidden />
        </button>

        <Button
          variant="ghost"
          size="sm"
          onClick={() => onRemove(plugin.id)}
          title="Remove plugin"
          className="hover:text-[var(--accent-error)] hover:border-[var(--accent-error)]"
        >
          <Trash2 size={12} strokeWidth={1.5} aria-hidden />
        </Button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Plugins page
// ---------------------------------------------------------------------------

export default function PluginsPage() {
  const [plugins, setPlugins] = useState<Plugin[]>(SAMPLE_PLUGINS);
  const [drawerPlugin, setDrawerPlugin] = useState<Plugin | null>(null);
  const [auditingId, setAuditingId] = useState<string | null>(null);

  function handleToggle(id: string) {
    setPlugins((prev) =>
      prev.map((p) =>
        p.id === id
          ? { ...p, status: p.status === 'active' ? 'disabled' : 'active' }
          : p,
      ),
    );
  }

  function handleAudit(id: string) {
    setAuditingId(id);
    setTimeout(() => setAuditingId(null), 1800);
  }

  function handleRemove(id: string) {
    setPlugins((prev) => prev.filter((p) => p.id !== id));
    if (drawerPlugin?.id === id) setDrawerPlugin(null);
  }

  const activeCount = plugins.filter((p) => p.status === 'active').length;

  return (
    <div className="flex flex-col gap-6 p-6">
      {/* Page header */}
      <div className="flex items-start justify-between">
        <div className="flex flex-col gap-1">
          <h1 className="font-[var(--font-mono)] text-[var(--text-lg)] text-[var(--text-strong)]">
            Plugins
          </h1>
          <p className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-muted)]">
            Manage installed plugins — WASM hooks, tool definitions, and gate integrations.
          </p>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          {auditingId && <Spinner size="sm" />}
          <Badge variant={activeCount > 0 ? 'success' : 'default'}>
            {activeCount} active
          </Badge>
        </div>
      </div>

      {/* Plugin list */}
      <section>
        <h2 className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tracking-[var(--tracking-widest)] uppercase mb-3">
          Installed plugins
        </h2>

        {plugins.length === 0 ? (
          <div className="border border-[var(--text-ghost)] px-4 py-6 text-center">
            <span className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-faint)]">
              No plugins installed
            </span>
          </div>
        ) : (
          <div className="border border-[var(--text-ghost)]">
            {plugins.map((plugin) => (
              <PluginRow
                key={plugin.id}
                plugin={plugin}
                onToggle={handleToggle}
                onAudit={handleAudit}
                onRemove={handleRemove}
                onInspect={setDrawerPlugin}
              />
            ))}
          </div>
        )}
      </section>

      {/* Plugin detail drawer */}
      {drawerPlugin && (
        <>
          {/* Backdrop */}
          <div
            className="fixed inset-0 bg-black/40 z-[var(--z-overlay)]"
            onClick={() => setDrawerPlugin(null)}
            aria-hidden
          />
          <PluginDrawer plugin={drawerPlugin} onClose={() => setDrawerPlugin(null)} />
        </>
      )}
    </div>
  );
}
