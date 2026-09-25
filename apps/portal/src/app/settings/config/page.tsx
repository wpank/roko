'use client';

import { useState, useEffect } from 'react';
import { clsx } from 'clsx';
import { Save, AlertCircle, Code, FormInput } from 'lucide-react';
import { Button, Badge, Spinner } from '@/components/atoms';
import { useConfig, useUpdateConfig } from '@/api/hooks';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type ViewMode = 'form' | 'raw';

type FieldType = 'text' | 'number' | 'toggle' | 'select';

interface FieldDef {
  key: string;
  label: string;
  type: FieldType;
  description: string;
  options?: string[];
  placeholder?: string;
}

interface CategoryDef {
  id: string;
  label: string;
  fields: FieldDef[];
}

// ---------------------------------------------------------------------------
// Config schema (form view categories / fields)
// ---------------------------------------------------------------------------

const CATEGORIES: CategoryDef[] = [
  {
    id: 'agent',
    label: 'Agent',
    fields: [
      {
        key: 'agent.default_model',
        label: 'Default model',
        type: 'text',
        placeholder: 'claude-sonnet',
        description: 'Model used when no routing preference is specified.',
      },
      {
        key: 'agent.default_backend',
        label: 'Default backend',
        type: 'select',
        options: ['claude', 'anthropic', 'openai', 'gemini', 'cerebras', 'perplexity', 'hermes', 'codex'],
        description: 'Provider backend used for agent dispatch.',
      },
      {
        key: 'agent.default_effort',
        label: 'Default effort',
        type: 'select',
        options: ['low', 'medium', 'high'],
        description: 'Token budget tier for agent turns.',
      },
      {
        key: 'agent.bare_mode',
        label: 'Bare mode',
        type: 'toggle',
        description: 'Replace the default system prompt instead of appending.',
      },
      {
        key: 'agent.context_limit_k',
        label: 'Context limit (K tokens)',
        type: 'number',
        placeholder: '200',
        description: 'Maximum context window in thousands of tokens.',
      },
      {
        key: 'agent.mode',
        label: 'Agent mode',
        type: 'select',
        options: ['ephemeral', 'persistent', 'long_running'],
        description: 'Default lifecycle mode for new agents.',
      },
    ],
  },
  {
    id: 'learning',
    label: 'Learning',
    fields: [
      {
        key: 'learning.replan_on_gate_failure',
        label: 'Replan on gate failure',
        type: 'toggle',
        description: 'Trigger a plan revision when a gate rung fails.',
      },
      {
        key: 'learning.dream_on_completion',
        label: 'Dream on completion',
        type: 'toggle',
        description: 'Run offline knowledge consolidation after a plan completes.',
      },
      {
        key: 'learning.auto_playbook_refresh',
        label: 'Auto playbook refresh',
        type: 'toggle',
        description: 'Automatically refresh when/then playbook entries from episodes.',
      },
      {
        key: 'learning.gate_threshold_flush_interval',
        label: 'Gate threshold flush interval',
        type: 'number',
        placeholder: '10',
        description: 'How many observations between EMA threshold persistence flushes.',
      },
    ],
  },
  {
    id: 'gates',
    label: 'Gates',
    fields: [
      {
        key: 'gates.mode',
        label: 'Gate mode',
        type: 'select',
        options: ['full', 'fast', 'off'],
        description: 'Gate pipeline execution mode.',
      },
      {
        key: 'gates.ema_alpha',
        label: 'EMA alpha',
        type: 'number',
        placeholder: '0.1',
        description: 'Exponential moving average weight for adaptive gate thresholds.',
      },
      {
        key: 'gates.max_iterations',
        label: 'Max iterations',
        type: 'number',
        placeholder: '3',
        description: 'Maximum gate retry iterations per task.',
      },
      {
        key: 'gates.clippy_enabled',
        label: 'Clippy enabled',
        type: 'toggle',
        description: 'Run cargo clippy as part of the gate pipeline.',
      },
      {
        key: 'gates.skip_tests',
        label: 'Skip tests',
        type: 'toggle',
        description: 'Skip cargo test in the gate pipeline (not recommended).',
      },
    ],
  },
  {
    id: 'serve',
    label: 'Serve',
    fields: [
      {
        key: 'serve.port',
        label: 'HTTP port',
        type: 'number',
        placeholder: '6677',
        description: 'Port for roko-serve to listen on (null = 6677).',
      },
      {
        key: 'serve.auto_orchestrate',
        label: 'Auto orchestrate',
        type: 'toggle',
        description: 'Automatically wire up PRD/plan publish triggers.',
      },
      {
        key: 'serve.auto_start',
        label: 'Auto start',
        type: 'toggle',
        description: 'Start roko-serve automatically with the daemon.',
      },
    ],
  },
  {
    id: 'budget',
    label: 'Budget',
    fields: [
      {
        key: 'budget.max_plan_usd',
        label: 'Max plan spend (USD)',
        type: 'number',
        placeholder: '50',
        description: 'Hard cost cap per plan execution.',
      },
      {
        key: 'budget.max_turn_usd',
        label: 'Max turn spend (USD)',
        type: 'number',
        placeholder: '0.5',
        description: 'Hard cost cap per agent turn.',
      },
      {
        key: 'budget.max_task_retry_usd',
        label: 'Max task retry spend (USD)',
        type: 'number',
        placeholder: '5',
        description: 'Cost cap for all retry attempts of a single task.',
      },
      {
        key: 'budget.prompt_token_budget',
        label: 'Prompt token budget',
        type: 'number',
        placeholder: '10000',
        description: 'Token budget for system prompt construction.',
      },
    ],
  },
];

// ---------------------------------------------------------------------------
// Sub-components: form fields
// ---------------------------------------------------------------------------

interface FieldRowProps {
  field: FieldDef;
  value: string;
  onChange: (key: string, value: string) => void;
}

function FieldRow({ field, value, onChange }: FieldRowProps) {
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex items-baseline justify-between gap-4">
        <label
          htmlFor={field.key}
          className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-strong)] leading-none shrink-0"
        >
          {field.label}
        </label>
        {field.type === 'toggle' ? (
          <button
            type="button"
            id={field.key}
            role="switch"
            aria-checked={value === 'true'}
            onClick={() => onChange(field.key, value === 'true' ? 'false' : 'true')}
            className={clsx(
              'relative flex items-center shrink-0',
              'w-9 h-5 border',
              value === 'true'
                ? 'bg-[var(--rose-dim)] border-[var(--rose)]'
                : 'bg-[var(--bg-highlight)] border-[var(--text-ghost)]',
              'transition-colors duration-[80ms]',
              'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
            )}
          >
            <span
              className={clsx(
                'absolute h-3 w-3 bg-[var(--text-strong)]',
                'transition-[left] duration-[80ms]',
                value === 'true' ? 'left-[18px]' : 'left-1',
              )}
            />
          </button>
        ) : field.type === 'select' ? (
          <select
            id={field.key}
            value={value}
            onChange={(e) => onChange(field.key, e.target.value)}
            className={clsx(
              'font-[var(--font-mono)] text-[var(--text-sm)]',
              'bg-[var(--bg-highlight)] text-[var(--text-strong)]',
              'border border-[var(--text-ghost)]',
              'px-2 py-1 min-w-[140px]',
              'focus:border-[var(--border-active)] focus:bg-[var(--bg-highlight)]',
              'outline-none',
            )}
          >
            {field.options?.map((opt) => (
              <option key={opt} value={opt}>
                {opt}
              </option>
            ))}
          </select>
        ) : (
          <input
            id={field.key}
            type={field.type === 'number' ? 'number' : 'text'}
            value={value}
            placeholder={field.placeholder}
            onChange={(e) => onChange(field.key, e.target.value)}
            className={clsx(
              'font-[var(--font-mono)] text-[var(--text-sm)]',
              'bg-[var(--bg-highlight)] text-[var(--text-strong)]',
              'border border-[var(--text-ghost)]',
              'px-2 py-1 min-w-[200px]',
              'placeholder:text-[var(--text-ghost)]',
              'focus:border-[var(--border-active)] focus:bg-[var(--bg-highlight)]',
              'outline-none',
            )}
          />
        )}
      </div>
      <p className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] leading-relaxed">
        {field.description}
      </p>
    </div>
  );
}

// ---------------------------------------------------------------------------
// CategoryPanel
// ---------------------------------------------------------------------------

interface CategoryPanelProps {
  category: CategoryDef;
  active: boolean;
  values: Record<string, string>;
  onFieldChange: (key: string, value: string) => void;
}

function CategoryPanel({ category, active, values, onFieldChange }: CategoryPanelProps) {
  if (!active) return null;

  return (
    <div className="flex flex-col gap-5">
      {category.fields.map((field) => (
        <FieldRow
          key={field.key}
          field={field}
          value={values[field.key] ?? ''}
          onChange={onFieldChange}
        />
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Config page
// ---------------------------------------------------------------------------

export default function ConfigPage() {
  const { data, isLoading, isError } = useConfig();
  const updateConfig = useUpdateConfig();

  const [viewMode, setViewMode] = useState<ViewMode>('form');
  const [activeCategory, setActiveCategory] = useState<string>(CATEGORIES[0].id);
  const [formValues, setFormValues] = useState<Record<string, string>>({});
  const [rawToml, setRawToml] = useState<string>('');
  const [isDirty, setIsDirty] = useState(false);

  // Seed form values once config loads
  useEffect(() => {
    if (!data?.config) return;
    const flat: Record<string, string> = {};
    for (const cat of CATEGORIES) {
      for (const field of cat.fields) {
        const parts = field.key.split('.');
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        let cursor: any = data.config;
        for (const p of parts) cursor = cursor?.[p];
        flat[field.key] = cursor !== undefined ? String(cursor) : '';
      }
    }
    setFormValues(flat);
    setRawToml(JSON.stringify(data.config, null, 2));
  }, [data]);

  function handleFieldChange(key: string, value: string) {
    setFormValues((prev) => ({ ...prev, [key]: value }));
    setIsDirty(true);
  }

  function handleRawChange(text: string) {
    setRawToml(text);
    setIsDirty(true);
  }

  function handleSave() {
    if (viewMode === 'raw') {
      try {
        const parsed = JSON.parse(rawToml);
        updateConfig.mutate(
          { patch: parsed },
          { onSuccess: () => setIsDirty(false) },
        );
      } catch {
        // JSON parse error — show inline error (handled via mutation state)
      }
    } else {
      // Rebuild nested patch from flat form values
      const patch: Record<string, unknown> = {};
      for (const [key, val] of Object.entries(formValues)) {
        const parts = key.split('.');
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        let cursor: any = patch;
        for (let i = 0; i < parts.length - 1; i++) {
          cursor[parts[i]] ??= {};
          cursor = cursor[parts[i]];
        }
        cursor[parts[parts.length - 1]] = val;
      }
      updateConfig.mutate(
        { patch },
        { onSuccess: () => setIsDirty(false) },
      );
    }
  }

  return (
    <div className="flex flex-col h-full min-h-0">
      {/* ---- Toolbar ---- */}
      <div
        className={clsx(
          'flex items-center justify-between gap-4 px-5 py-3 shrink-0',
          'border-b border-b-[var(--text-ghost)]',
        )}
      >
        <div className="flex items-center gap-3">
          <h1 className="font-[var(--font-mono)] text-[var(--text-base)] text-[var(--text-strong)]">
            Config editor
          </h1>
          {isDirty && (
            <Badge variant="warning">unsaved</Badge>
          )}
        </div>

        {/* View toggle */}
        <div className="flex items-center gap-2 ml-auto">
          <button
            type="button"
            onClick={() => setViewMode('form')}
            className={clsx(
              'flex items-center gap-1.5 px-2.5 py-1.5',
              'font-[var(--font-mono)] text-[var(--text-xs)]',
              'border transition-colors duration-[80ms]',
              viewMode === 'form'
                ? 'border-[var(--rose-dim)] text-[var(--rose-glow)] bg-[var(--bg-highlight)]'
                : 'border-[var(--text-ghost)] text-[var(--text-muted)] hover:border-[var(--border-hover)] hover:text-[var(--text-strong)]',
              'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
            )}
          >
            <FormInput size={12} strokeWidth={1.5} aria-hidden />
            Form
          </button>
          <button
            type="button"
            onClick={() => setViewMode('raw')}
            className={clsx(
              'flex items-center gap-1.5 px-2.5 py-1.5',
              'font-[var(--font-mono)] text-[var(--text-xs)]',
              'border transition-colors duration-[80ms]',
              viewMode === 'raw'
                ? 'border-[var(--rose-dim)] text-[var(--rose-glow)] bg-[var(--bg-highlight)]'
                : 'border-[var(--text-ghost)] text-[var(--text-muted)] hover:border-[var(--border-hover)] hover:text-[var(--text-strong)]',
              'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
            )}
          >
            <Code size={12} strokeWidth={1.5} aria-hidden />
            Raw
          </button>
        </div>

        {/* Save button */}
        <Button
          variant="primary"
          size="sm"
          loading={updateConfig.isPending}
          disabled={!isDirty}
          onClick={handleSave}
        >
          <Save size={12} strokeWidth={1.5} aria-hidden />
          Save
        </Button>
      </div>

      {/* ---- Mutation error banner ---- */}
      {updateConfig.isError && (
        <div
          className={clsx(
            'flex items-center gap-2 px-5 py-2 shrink-0',
            'bg-[var(--accent-error)]/10 border-b border-b-[var(--accent-error)]',
          )}
        >
          <AlertCircle size={13} strokeWidth={1.5} className="text-[var(--accent-error)] shrink-0" aria-hidden />
          <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--accent-error)]">
            {updateConfig.error?.message ?? 'Save failed — check server logs'}
          </span>
        </div>
      )}

      {/* ---- Loading / error states ---- */}
      {isLoading && (
        <div className="flex-1 flex items-center justify-center">
          <Spinner size="md" />
        </div>
      )}

      {isError && !isLoading && (
        <div className="flex-1 flex items-center justify-center">
          <span className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-muted)]">
            Failed to load configuration
          </span>
        </div>
      )}

      {/* ---- Main editor area ---- */}
      {!isLoading && !isError && viewMode === 'form' && (
        <div className="flex flex-1 min-h-0 overflow-hidden">
          {/* Category tabs (left mini-nav) */}
          <nav
            className={clsx(
              'flex flex-col shrink-0 w-[140px] gap-0.5 p-2',
              'border-r border-r-[var(--text-ghost)] overflow-y-auto',
            )}
            aria-label="Config categories"
          >
            {CATEGORIES.map((cat) => (
              <button
                key={cat.id}
                type="button"
                onClick={() => setActiveCategory(cat.id)}
                className={clsx(
                  'flex items-center h-8 px-3 text-left',
                  'font-[var(--font-mono)] text-[var(--text-xs)] tracking-[var(--tracking-wide)]',
                  'border transition-colors duration-[80ms]',
                  activeCategory === cat.id
                    ? 'border-[var(--rose-dim)] bg-[var(--bg-highlight)] text-[var(--rose-glow)]'
                    : 'border-transparent text-[var(--text-muted)] hover:bg-[var(--bg-highlight)] hover:text-[var(--text-strong)]',
                  'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
                )}
              >
                {cat.label}
              </button>
            ))}
          </nav>

          {/* Fields area */}
          <div className="flex-1 overflow-y-auto p-5">
            {CATEGORIES.map((cat) => (
              <CategoryPanel
                key={cat.id}
                category={cat}
                active={activeCategory === cat.id}
                values={formValues}
                onFieldChange={handleFieldChange}
              />
            ))}
          </div>
        </div>
      )}

      {!isLoading && !isError && viewMode === 'raw' && (
        <div className="flex-1 flex flex-col min-h-0 p-4">
          <textarea
            value={rawToml}
            onChange={(e) => handleRawChange(e.target.value)}
            spellCheck={false}
            aria-label="Raw config JSON"
            className={clsx(
              'flex-1 resize-none font-[var(--font-mono)] text-[var(--text-xs)]',
              'bg-[var(--void)] text-[var(--text-strong)]',
              'border border-[var(--text-ghost)]',
              'p-4 leading-relaxed',
              'focus:border-[var(--border-active)]',
              'outline-none',
            )}
          />
          <p className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] mt-2">
            Edit the config as JSON. Changes are applied when you click Save.
          </p>
        </div>
      )}
    </div>
  );
}
