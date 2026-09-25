'use client';

import { Keyboard } from 'lucide-react';
import { clsx } from 'clsx';

// ---------------------------------------------------------------------------
// Types — kept local so Header has no hard store dependency.
// Callers pass in pre-selected slices.
// ---------------------------------------------------------------------------

interface ActivePlan {
  name: string;
  /** Completed task count, 0–total. */
  completed: number;
  total: number;
}

interface HeaderProps {
  /**
   * Name of the active workspace shown in the brand slot.
   * Defaults to "roko".
   */
  workspaceName?: string;
  /**
   * The currently-running plan, if any.  Pass `null` or omit to hide the
   * progress strip entirely.
   */
  activePlan?: ActivePlan | null;
  /**
   * Accumulated USD cost for today, pre-formatted as a number.
   * e.g. 3.47 → "$3.47"
   */
  costToday?: number;
  /**
   * Called when the Cmd+K button is pressed.
   * Should open the command palette.
   */
  onCommandPalette?: () => void;
}

// ---------------------------------------------------------------------------
// Mini progress bar
// ---------------------------------------------------------------------------

interface MiniProgressProps {
  plan: ActivePlan;
}

function MiniProgress({ plan }: MiniProgressProps) {
  const pct = plan.total > 0 ? (plan.completed / plan.total) * 100 : 0;

  return (
    <div className="flex items-center gap-2 min-w-0 max-w-xs">
      {/* Plan name */}
      <span
        className="truncate font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] leading-none shrink-0 max-w-[140px]"
        title={plan.name}
      >
        {plan.name}
      </span>

      {/* Track */}
      <div
        className="relative h-[3px] flex-1 min-w-[80px] rounded-full overflow-hidden bg-[var(--bg-highlight)]"
        role="progressbar"
        aria-valuenow={pct}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={`${plan.name}: ${plan.completed} of ${plan.total} tasks`}
      >
        <div
          className="absolute inset-y-0 left-0 rounded-full bg-[var(--rose-glow)] transition-[width] duration-300 ease-[var(--ease-out)]"
          style={{ width: `${pct}%` }}
        />
      </div>

      {/* Counter */}
      <span className="shrink-0 font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tabular-nums leading-none">
        {plan.completed}/{plan.total}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Cost display
// ---------------------------------------------------------------------------

function formatCost(usd: number): string {
  return `$${usd.toFixed(2)}`;
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

export default function Header({
  workspaceName = 'roko',
  activePlan = null,
  costToday = 0,
  onCommandPalette,
}: HeaderProps) {
  return (
    <header
      className={clsx(
        'flex items-center justify-between',
        'h-12 px-4 shrink-0',
        'bg-[var(--bg-raised)]',
        'border-b border-b-[var(--text-ghost)]',
        // Sit above the sidebar in the stacking context
        'relative z-[var(--z-sticky)]',
      )}
    >
      {/* ---- Left: Workspace brand ---- */}
      <div className="flex items-center gap-0 min-w-0 shrink-0">
        <span
          className="font-[var(--font-mono)] text-[var(--text-md)] font-semibold text-[var(--rose-glow)] tracking-[var(--tracking-wide)] leading-none select-none"
        >
          {workspaceName}
        </span>
      </div>

      {/* ---- Center: Active plan progress ---- */}
      <div className="flex-1 flex items-center justify-center px-4 min-w-0">
        {activePlan ? (
          <MiniProgress plan={activePlan} />
        ) : null}
      </div>

      {/* ---- Right: Cost + Cmd+K ---- */}
      <div className="flex items-center gap-3 shrink-0">
        {/* Cost today */}
        <span
          className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)] tabular-nums leading-none"
          title="Cost today (USD)"
        >
          {formatCost(costToday)}
        </span>

        {/* Cmd+K trigger */}
        <button
          type="button"
          onClick={onCommandPalette}
          className={clsx(
            'flex items-center gap-1.5 px-2 h-6 rounded-sm',
            'border border-[var(--text-ghost)]',
            'text-[var(--text-faint)] hover:text-[var(--text-muted)] hover:border-[var(--rose-dim)]',
            'font-[var(--font-mono)] text-[var(--text-xs)] leading-none',
            'transition-colors duration-[80ms] ease-[var(--ease-out)]',
            'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
          )}
          aria-label="Open command palette (⌘K)"
        >
          <Keyboard size={11} strokeWidth={1.5} aria-hidden />
          <span className="hidden sm:inline">⌘K</span>
        </button>
      </div>
    </header>
  );
}
