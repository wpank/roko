'use client';

/**
 * Stage — the central content area of the Workspace.
 *
 * Renders one of four states:
 *   1. PromptPanel (generate, firstRun=true) when loaded and no plans yet.
 *   2. PromptPanel (generate, firstRun=false) when promptOpen is true.
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

  // ── Layout style ────────────────────────────────────────────────────────────

  // Empty state centres its sentence; other modes use top-aligned column flow.
  const sectionStyle: React.CSSProperties = showEmpty
    ? {
        flex: 1,
        overflow: 'auto',
        minHeight: 0,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: '1rem',
        textAlign: 'center',
      }
    : {
        flex: 1,
        overflow: 'auto',
        minHeight: 0,
        padding: '1rem',
      };

  // ── Content ──────────────────────────────────────────────────────────────────

  return (
    <section data-region="stage" style={sectionStyle}>
      {showPrompt && (
        <PromptPanel
          mode="generate"
          workspace={workspace}
          firstRun={noPlans}
          onDone={handleGenerateDone}
          // Cancel only available when there are plans to return to.
          onCancel={!noPlans ? onClosePrompt : undefined}
        />
      )}

      {showPlan && (
        <PlanView
          plan={selectedPlan!}
          selectedTaskId={selection.task}
          onSelectTask={(id) => onSelect({ task: id })}
          onRequestError={onRequestError}
        />
      )}

      {showEmpty && (
        <p
          style={{
            color: 'var(--text-faint)',
            fontSize: 'var(--text-sm)',
            fontFamily: 'var(--font-mono)',
          }}
        >
          {describeEmpty({
            workspace,
            connection,
            planCount: plans?.length ?? 0,
            plan: undefined,
          })}
        </p>
      )}
    </section>
  );
}
