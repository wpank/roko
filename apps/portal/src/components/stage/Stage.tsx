'use client';

/**
 * Stage — the central content area of the Workspace.
 *
 * Renders one of four states:
 *   1. A project with no plans: what roko is (Intro), the request box, and how it works.
 *   2. New plan: the request box under its own heading.
 *   3. PlanView when a plan is selected.
 *   4. A describeEmpty sentence otherwise (plans exist, nothing selected).
 */

import React from 'react';
import { useQueryClient } from '@tanstack/react-query';
import type { WirePlanSummary } from '@/api/contracts';
import type { Selection } from '@/lib/selection';
import { queryKeys } from '@/api/queries';
import { useDashboardStore } from '@/stores/dashboard';
import { describeEmpty } from '@/lib/emptyState';
import { Key } from '@/components/kit/Key';
import { HowItWorks, Intro } from '@/components/shell/Intro';
import { PromptPanel } from '@/components/stage/PromptPanel';
import { PlanView } from '@/components/stage/PlanView';

// ── Types ───────────────────────────────────────────────────────────────────────

export interface StageProps {
  plans: WirePlanSummary[] | undefined;
  loaded: boolean;
  selection: Selection;
  promptOpen: boolean;
  workspace: string;
  onSelect(patch: Partial<Selection>): void;
  onClosePrompt(): void;
  onRequestError(message: string): void;
}

// ── Stage ───────────────────────────────────────────────────────────────────────

export function Stage({
  plans,
  loaded,
  selection,
  promptOpen,
  workspace,
  onSelect,
  onClosePrompt,
  onRequestError,
}: StageProps) {
  const queryClient = useQueryClient();
  const connection = useDashboardStore((s) => s.connection);

  // True once the list has loaded and has no plans.
  const noPlans = loaded && (!plans || plans.length === 0);

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

  // ── Determine content mode ───────────────────────────────────────────────────

  const showPrompt = noPlans || promptOpen;
  const showPlan = !showPrompt && selectedPlan !== null;
  const showEmpty = !showPrompt && !showPlan;

  // ── Content ──────────────────────────────────────────────────────────────────

  return (
    <section data-region="stage" className="pt-stage">
      <div className="pt-page min-h-full flex flex-col">
        {showPrompt && noPlans && (
          <>
            <Intro />
            <PromptPanel mode="generate" workspace={workspace} firstRun onDone={handleGenerateDone} />
            <HowItWorks />
          </>
        )}

        {showPrompt && !noPlans && (
          <>
            <div className="pt-head">
              <h1>New plan</h1>
              <Key size="s" onClick={onClosePrompt}>
                close
              </Key>
            </div>
            <PromptPanel
              mode="generate"
              workspace={workspace}
              firstRun={false}
              onDone={handleGenerateDone}
              onCancel={onClosePrompt}
            />
          </>
        )}

        {/* Keyed by plan: another plan gets a fresh view, never this one's editor text. */}
        {showPlan && (
          <PlanView
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
              plan: undefined,
            })}
          </p>
        )}
      </div>
    </section>
  );
}
