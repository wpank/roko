'use client';

import { useState } from 'react';
import { clsx } from 'clsx';
import { Plus, Eye, EyeOff, RefreshCw, Trash2, TestTube, AlertTriangle } from 'lucide-react';
import { Button, Badge, StatusLED } from '@/components/atoms';
import { useProviders, useTestProvider } from '@/api/hooks';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface ProviderKeyRow {
  id: string;
  name: string;
  displayName: string;
  maskedKey: string | null;
  configured: boolean;
  keyEnvVar: string;
}

// ---------------------------------------------------------------------------
// Placeholder data — real data would come from config / providers endpoints
// ---------------------------------------------------------------------------

const PROVIDER_KEY_DEFS: Omit<ProviderKeyRow, 'configured' | 'maskedKey'>[] = [
  { id: 'anthropic',   name: 'anthropic',   displayName: 'Anthropic',   keyEnvVar: 'ANTHROPIC_API_KEY' },
  { id: 'openai',      name: 'openai',      displayName: 'OpenAI',      keyEnvVar: 'OPENAI_API_KEY' },
  { id: 'gemini',      name: 'gemini',      displayName: 'Google Gemini', keyEnvVar: 'GEMINI_API_KEY' },
  { id: 'cerebras',    name: 'cerebras',    displayName: 'Cerebras',    keyEnvVar: 'CEREBRAS_API_KEY' },
  { id: 'perplexity',  name: 'perplexity',  displayName: 'Perplexity',  keyEnvVar: 'PERPLEXITY_API_KEY' },
  { id: 'hermes',      name: 'hermes',      displayName: 'Hermes',      keyEnvVar: 'HERMES_API_KEY' },
];

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

interface KeyRowProps {
  row: ProviderKeyRow;
  onTest: (id: string) => void;
  onDelete: (id: string) => void;
  testing: boolean;
}

function KeyRow({ row, onTest, onDelete, testing }: KeyRowProps) {
  const [revealed, setRevealed] = useState(false);

  return (
    <div
      className={clsx(
        'flex items-center gap-3 px-4 py-3',
        'border-b border-b-[var(--text-ghost)] last:border-b-0',
        'hover:bg-[var(--bg-highlight)]',
        'transition-colors duration-[80ms]',
      )}
    >
      {/* Status LED */}
      <StatusLED status={row.configured ? 'success' : 'offline'} />

      {/* Provider name */}
      <div className="flex flex-col gap-0.5 w-36 shrink-0">
        <span className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-strong)]">
          {row.displayName}
        </span>
        <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]">
          {row.keyEnvVar}
        </span>
      </div>

      {/* Key preview */}
      <div className="flex-1 flex items-center gap-2 min-w-0">
        {row.configured ? (
          <>
            <code className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)] truncate">
              {revealed && row.maskedKey ? row.maskedKey : '••••••••••••••••••••••••••••••••'}
            </code>
            <button
              type="button"
              title={revealed ? 'Hide key' : 'Reveal key preview'}
              onClick={() => setRevealed((r) => !r)}
              className={clsx(
                'shrink-0 p-1',
                'text-[var(--text-faint)] hover:text-[var(--text-muted)]',
                'transition-colors duration-[80ms]',
                'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
              )}
            >
              {revealed ? <EyeOff size={12} strokeWidth={1.5} aria-hidden /> : <Eye size={12} strokeWidth={1.5} aria-hidden />}
            </button>
          </>
        ) : (
          <Badge variant="warning">not configured</Badge>
        )}
      </div>

      {/* Actions */}
      <div className="flex items-center gap-1 shrink-0">
        <Button
          variant="ghost"
          size="sm"
          loading={testing}
          disabled={!row.configured}
          onClick={() => onTest(row.id)}
          title="Test provider connectivity"
        >
          <TestTube size={12} strokeWidth={1.5} aria-hidden />
          Test
        </Button>
        <Button
          variant="ghost"
          size="sm"
          disabled={!row.configured}
          title="Rotate API key"
        >
          <RefreshCw size={12} strokeWidth={1.5} aria-hidden />
          Rotate
        </Button>
        <Button
          variant="ghost"
          size="sm"
          disabled={!row.configured}
          onClick={() => onDelete(row.id)}
          title="Remove API key"
          className="hover:text-[var(--accent-error)] hover:border-[var(--accent-error)]"
        >
          <Trash2 size={12} strokeWidth={1.5} aria-hidden />
        </Button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Add-key form
// ---------------------------------------------------------------------------

interface AddKeyFormProps {
  onAdd: (provider: string, key: string) => void;
  adding: boolean;
}

function AddKeyForm({ onAdd, adding }: AddKeyFormProps) {
  const [provider, setProvider] = useState('anthropic');
  const [key, setKey] = useState('');

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!key.trim()) return;
    onAdd(provider, key.trim());
    setKey('');
  }

  return (
    <form onSubmit={handleSubmit} className="flex items-end gap-3">
      <div className="flex flex-col gap-1">
        <label
          htmlFor="add-key-provider"
          className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]"
        >
          Provider
        </label>
        <select
          id="add-key-provider"
          value={provider}
          onChange={(e) => setProvider(e.target.value)}
          className={clsx(
            'font-[var(--font-mono)] text-[var(--text-sm)]',
            'bg-[var(--bg-highlight)] text-[var(--text-strong)]',
            'border border-[var(--text-ghost)]',
            'px-2 py-1.5',
            'focus:border-[var(--border-active)]',
            'outline-none',
          )}
        >
          {PROVIDER_KEY_DEFS.map((p) => (
            <option key={p.id} value={p.id}>{p.displayName}</option>
          ))}
        </select>
      </div>

      <div className="flex flex-col gap-1 flex-1">
        <label
          htmlFor="add-key-value"
          className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]"
        >
          API key
        </label>
        <input
          id="add-key-value"
          type="password"
          value={key}
          onChange={(e) => setKey(e.target.value)}
          placeholder="sk-…"
          autoComplete="off"
          className={clsx(
            'font-[var(--font-mono)] text-[var(--text-sm)]',
            'bg-[var(--bg-highlight)] text-[var(--text-strong)]',
            'border border-[var(--text-ghost)]',
            'px-2 py-1.5',
            'placeholder:text-[var(--text-ghost)]',
            'focus:border-[var(--border-active)]',
            'outline-none',
          )}
        />
      </div>

      <Button
        type="submit"
        variant="primary"
        size="sm"
        loading={adding}
        disabled={!key.trim()}
      >
        <Plus size={12} strokeWidth={1.5} aria-hidden />
        Add key
      </Button>
    </form>
  );
}

// ---------------------------------------------------------------------------
// Keys page
// ---------------------------------------------------------------------------

export default function KeysPage() {
  const { data } = useProviders();
  const testProvider = useTestProvider();
  const [testResults, setTestResults] = useState<Record<string, 'ok' | 'fail'>>({});

  const configuredIds = new Set(data?.providers?.map((p) => p.id) ?? []);

  const rows: ProviderKeyRow[] = PROVIDER_KEY_DEFS.map((def) => ({
    ...def,
    configured: configuredIds.has(def.id),
    maskedKey: configuredIds.has(def.id) ? `${def.id.toUpperCase()}_KEY_sk-…(hidden)` : null,
  }));

  function handleTest(id: string) {
    testProvider.mutate(id, {
      onSuccess: () => setTestResults((r) => ({ ...r, [id]: 'ok' })),
      onError: () => setTestResults((r) => ({ ...r, [id]: 'fail' })),
    });
  }

  function handleDelete(_id: string) {
    // TODO: wire to config patch endpoint
  }

  function handleAdd(_provider: string, _key: string) {
    // TODO: wire to config patch endpoint
  }

  return (
    <div className="flex flex-col gap-6 p-6">
      {/* Page header */}
      <div className="flex flex-col gap-1">
        <h1 className="font-[var(--font-mono)] text-[var(--text-lg)] text-[var(--text-strong)]">
          API Keys
        </h1>
        <p className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-muted)]">
          Manage provider API keys used by roko agents.
        </p>
      </div>

      {/* Warning banner */}
      <div
        className={clsx(
          'flex items-start gap-3 px-4 py-3',
          'bg-[var(--warning)]/8 border border-[var(--warning)]',
        )}
      >
        <AlertTriangle size={14} strokeWidth={1.5} className="text-[var(--warning)] shrink-0 mt-0.5" aria-hidden />
        <p className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)] leading-relaxed">
          API keys are stored locally in{' '}
          <code className="text-[var(--bone)]">roko.toml</code> or via environment
          variables. Avoid committing your config file to version control.
        </p>
      </div>

      {/* Provider key list */}
      <section>
        <h2 className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tracking-[var(--tracking-widest)] uppercase mb-3">
          Provider keys
        </h2>
        <div className="border border-[var(--text-ghost)]">
          {rows.map((row) => (
            <KeyRow
              key={row.id}
              row={row}
              onTest={handleTest}
              onDelete={handleDelete}
              testing={testProvider.isPending && testProvider.variables === row.id}
            />
          ))}
        </div>

        {/* Test result feedback */}
        {Object.entries(testResults).map(([id, result]) => (
          <p
            key={id}
            className={clsx(
              'font-[var(--font-mono)] text-[var(--text-xs)] mt-2',
              result === 'ok' ? 'text-[var(--sage)]' : 'text-[var(--accent-error)]',
            )}
          >
            {id}: {result === 'ok' ? 'Connection successful' : 'Connection failed'}
          </p>
        ))}
      </section>

      {/* Add key form */}
      <section>
        <h2 className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tracking-[var(--tracking-widest)] uppercase mb-3">
          Add key
        </h2>
        <div className="border border-[var(--text-ghost)] p-4">
          <AddKeyForm onAdd={handleAdd} adding={false} />
        </div>
      </section>
    </div>
  );
}
