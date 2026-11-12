'use client';

/**
 * Stage — the central content area of the Workspace.
 *
 * Renders one of five states:
 *   1. A plan being written, when its row is selected: the writing view, with its stop key (gap-f8c57a).
 *   2. A project with no plans: what roko is (Intro), the request box, and how it works.
 *   3. New plan: the request box under its own heading.
 *   4. The plan's page (PlanPage) when a plan is selected.
 *   5. A describeEmpty sentence otherwise (plans exist, nothing selected).
 */

import React from 'react';
import { useQueryClient } from '@tanstack/react-query';
import type { WirePlanSummary, WirePlanWriting } from '@/api/contracts';
import type { Selection } from '@/lib/selection';
import { queryKeys } from '@/api/queries';
import { useDashboardStore } from '@/stores/dashboard';
import { describeEmpty } from '@/lib/emptyState';
import { Key } from '@/components/kit/Key';
import { HowItWorks, Intro } from '@/components/shell/Intro';
import { PromptPanel } from '@/components/stage/PromptPanel';
import { Writing } from '@/components/stage/Writing';
import { PlanPage } from '@/components/plan/PlanPage';
import { stoppedSentence } from '@/lib/writing';
import { writeFailure } from '@/lib/writeFailure';

// ── Types ───────────────────────────────────────────────────────────────────────

/** What the request box opens with: a stopped write's request, and what it spent. */
export interface PromptSeed {
  prompt: string;
  note: string;
  /** When it was given: each seed opens a fresh box. */
  at: number;
}

export interface StageProps {
  plans: WirePlanSummary[] | undefined;
  /** The plans being written (gap-f8c57a). */
  writes?: readonly (WirePlanSummary & { writing: WirePlanWriting })[];
  loaded: boolean;
  selection: Selection;
  promptOpen: boolean;
  /** What the request box opens with, when a stopped write gave it back. */
  promptSeed?: PromptSeed | null;
  workspace: string;
  onSelect(patch: Partial<Selection>): void;
  onClosePrompt(): void;
  /** Open the request box with `seed`: a write stopped from its own view. */
  onReopenPrompt?(seed: PromptSeed): void;
  onRequestError(message: string): void;
}

// ── Stage ───────────────────────────────────────────────────────────────────────

export function Stage({
  plans,
  writes = [],
  loaded,
  selection,
  promptOpen,
  promptSeed = null,
  workspace,
  onSelect,
  onClosePrompt,
  onReopenPrompt,
  onRequestError,
}: StageProps) {
  const queryClient = useQueryClient();
  const connection = useDashboardStore((s) => s.connection);

  // True once the list has loaded and has no plans.
  const noPlans = loaded && (!plans || plans.length === 0);
  // The first-run box keeps its place while its write runs: the list shows the plan a moment before the write ends
  // (gap-f8c57a), and the box must stay to open it.
  const [firstWriting, setFirstWriting] = React.useState(false);
  const firstRun = noPlans || firstWriting;

  // The currently selected plan object (null if id unknown or list not ready).
  const selectedPlan = selection.plan
    ? (plans?.find((p) => p.id === selection.plan) ?? null)
    : null;

  // After a successful generate: await the list refresh BEFORE navigating to
  // the new plan. resolveSelection (in Workspace) clears a selected plan id
  // that the loaded list does not contain — so if we called onSelect while the
  // refetch was still in flight the first plan would be deselected and the
  // operator left at "Select a plan". Awaiting invalidateQueries guarantees
  // the fresh list (which now includes the new plan) is in the cache before
  // we update the selection.
  const handleGenerateDone = React.useCallback(
    async (slug: string) => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.plans });
      onSelect({ plan: slug, task: null });
      onClosePrompt();
    },
    [queryClient, onSelect, onClosePrompt],
  );

  // A plan being written, opened from its row: the request box, when open, comes first.
  const selectedWrite = !promptOpen && selection.plan ? (writes.find((w) => w.id === selection.plan) ?? null) : null;

  // ── Determine content mode ───────────────────────────────────────────────────

  const showWrite = selectedWrite !== null;
  const showPrompt = !showWrite && (firstRun || promptOpen);
  const showPlan = !showWrite && !showPrompt && selectedPlan !== null;
  const showEmpty = !showWrite && !showPrompt && !showPlan;

  // ── Content ──────────────────────────────────────────────────────────────────

  return (
    <section data-region="stage" className="pt-stage">
      <div className="pt-page min-h-full flex flex-col">
        {showWrite && (
          <>
            <div className="pt-head">
              <h1>Writing the plan</h1>
            </div>
            {/* Keyed by write: another write gets a fresh view. */}
            <section key={selectedWrite.id} className="rk-module pt-ask" data-region="writing">
              <Writing
                mode="generate"
                workspace={workspace}
                request={selectedWrite.title || selectedWrite.id}
                accepted={{
                  id: selectedWrite.writing.run_id,
                  plan_id: selectedWrite.id,
                  max_usd: selectedWrite.writing.max_usd ?? null,
                }}
                sinceMs={(selectedWrite.writing.elapsed_secs ?? 0) * 1000}
                onDone={() => void queryClient.invalidateQueries({ queryKey: queryKeys.plans })}
                onStopped={(costUsd) =>
                  onReopenPrompt?.({ prompt: selectedWrite.title, note: stoppedSentence(costUsd), at: Date.now() })
                }
                onFailed={(err) =>
                  onRequestError(
                    err instanceof Error ? writeFailure(err.message, 'generate').sentence : String(err),
                  )
                }
              />
            </section>
          </>
        )}

        {showPrompt && firstRun && (
          <>
            <Intro />
            <PromptPanel
              key={promptSeed ? `seed-${promptSeed.at}` : 'ask'}
              mode="generate"
              workspace={workspace}
              firstRun
              onDone={handleGenerateDone}
              onWaitingChange={setFirstWriting}
              initialPrompt={promptSeed?.prompt}
              initialNote={promptSeed?.note}
            />
            <HowItWorks />
          </>
        )}

        {showPrompt && !firstRun && (
          <>
            <div className="pt-head">
              <h1>New plan</h1>
              <Key size="s" onClick={onClosePrompt}>
                close
              </Key>
            </div>
            <PromptPanel
              key={promptSeed ? `seed-${promptSeed.at}` : 'ask'}
              mode="generate"
              workspace={workspace}
              firstRun={false}
              onDone={handleGenerateDone}
              onCancel={onClosePrompt}
              initialPrompt={promptSeed?.prompt}
              initialNote={promptSeed?.note}
            />
          </>
        )}

        {/* Keyed by plan: another plan gets a fresh view, never this one's editor text. */}
        {showPlan && (
          <PlanPage
            key={selectedPlan!.id}
            plan={selectedPlan!}
            selectedTaskId={selection.task}
            onSelectTask={(id) => onSelect({ task: id })}
            onRequestError={onRequestError}
          />
        )}

        {showEmpty && (
          <p className="pt-hint" data-empty>
            {describeEmpty({
              workspace,
              connection,
              planCount: plans?.length ?? 0,
                })}
          </p>
        )}
      </div>
    </section>
  );
}
