'use client';

/**
 * PlanPage — a plan as roko's run card at full width (the design system's §4.8), the same shape as the plan's card
 * in Hermes:
 *
 * 1. its state in a pill, and its title;
 * 2. the glass: TASKS, TIME, SPENT and CHECKS, in dot-matrix;
 * 3. the plan drawn as a sequencer; a click on a pad opens that task under it;
 * 4. one sentence, the state's own (or a line per task that runs now), and its keys, at most one of them orange;
 * 5. `▸ details`: each task's tries, models, times and spend, the plan's own check, tokens, and the plan's file.
 *
 * Revise and the editor take the place of the glass and the sequencer while open. The resume question
 * (gap-0eb763) and a revision's diff sit under the title.
 */

import React, { useCallback, useMemo, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import type { WirePlanDiff, WirePlanSummary } from '@/api/contracts';
import { queryKeys, useGateRungs, usePlanCap, usePlanTasks, useValidation } from '@/api/queries';
import { Fold } from '@/components/kit/Fold';
import { Key } from '@/components/kit/Key';
import { Pill } from '@/components/kit/Pill';
import { Readout } from '@/components/plan/Readout';
import { Sequencer } from '@/components/plan/Sequencer';
import { TaskDetail } from '@/components/plan/TaskDetail';
import { usePrimaryAction } from '@/components/plan/usePrimaryAction';
import { PromptPanel } from '@/components/stage/PromptPanel';
import { RevisionDiff } from '@/components/stage/RevisionDiff';
import { RunChoice } from '@/components/stage/RunChoice';
import { SourceEditor } from '@/components/stage/SourceEditor';
import { ValidationBadge, validationCounts } from '@/components/stage/ValidationBadge';
import { waitReason } from '@/lib/planSet';
import { AWAITING_APPROVAL, becauseOf, buildCard } from '@/lib/runCard';
import type { Card, NowLine } from '@/lib/runCard';
import type { RunState } from '@/lib/runState';
import { taskKey } from '@/lib/runState';
import { useNow } from '@/lib/useNow';
import { clock, count, duration, money } from '@/lib/words';
import { confirmDiscard, useDashboardStore } from '@/stores/dashboard';

export interface PlanPageProps {
  plan: WirePlanSummary;
  selectedTaskId: string | null;
  onSelectTask(id: string | null): void;
  onRequestError(message: string): void;
}

export function PlanPage({ plan, selectedTaskId, onSelectTask, onRequestError }: PlanPageProps) {
  const [editing, setEditing] = useState(false);
  const [revising, setRevising] = useState(false);
  // What the last revision changed, shown until dismissed (3229).
  const [revisionDiff, setRevisionDiff] = useState<WirePlanDiff | null>(null);
  // The editor's unsaved text is marked in the store, where the run key, the `r` key, run all and the selection
  // all see it.
  const dirty = useDashboardStore((s) => s.unsavedPlan === plan.id);
  const setUnsaved = useDashboardStore((s) => s.setUnsaved);
  const markDirty = useCallback((d: boolean) => setUnsaved(plan.id, d), [setUnsaved, plan.id]);

  const queryClient = useQueryClient();
  const run = useDashboardStore((s) => s.run);
  const { data: tasksData } = usePlanTasks(plan.id);
  const { data: rungs } = useGateRungs();
  const { data: capUsd } = usePlanCap(plan.id);
  const { data: validation } = useValidation(plan.id);
  const problems = validationCounts(validation);

  const running = run.plans[plan.id]?.phase === 'running';
  const nowMs = useNow(running);
  const card = useMemo(
    () =>
      buildCard({
        plan,
        tasks: tasksData?.tasks ?? [],
        run,
        rungs: rungs ?? [],
        capUsd: capUsd ?? null,
        nowMs,
      }),
    [plan, tasksData, run, rungs, capUsd, nowMs],
  );
  const primary = usePrimaryAction(plan.id, { onError: onRequestError });

  const selectedRow = card.rows.find((r) => r.id === selectedTaskId) ?? null;
  const selectedPad = selectedRow ? card.steps.flatMap((s) => s.pads).find((p) => p.id === selectedRow.id) : undefined;
  const heldIds = card.rows.filter((r) => run.tasks[taskKey(plan.id, r.id)]?.phase === AWAITING_APPROVAL).map((r) => r.id);

  const toggleTask = useCallback(
    (id: string) => onSelectTask(id === selectedTaskId ? null : id),
    [onSelectTask, selectedTaskId],
  );

  const handleRevisionDone = useCallback(
    async (_slug: string, diff?: WirePlanDiff) => {
      // A revision lands only when its operation completes; refresh everything it may have changed before the
      // prompt closes, so the new tasks are on screen when it does.
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: queryKeys.plans }),
        queryClient.invalidateQueries({ queryKey: queryKeys.planTasks(plan.id) }),
        queryClient.invalidateQueries({ queryKey: queryKeys.planSource(plan.id) }),
        queryClient.invalidateQueries({ queryKey: queryKeys.validation(plan.id) }),
      ]);
      setRevisionDiff(diff ?? null);
      setRevising(false);
    },
    [queryClient, plan.id],
  );

  const reviseKey = (
    <Key
      data-action="revise"
      aria-pressed={revising}
      disabled={running}
      title={running ? 'Cannot revise while the plan is running' : 'Ask roko to change the plan, in words'}
      onClick={() => {
        if (!confirmDiscard(dirty)) return;
        setRevising((r) => !r);
        setEditing(false);
      }}
    >
      revise
    </Key>
  );

  return (
    // While editing, the page grows to fill the stage and the editor takes what is left (design §2, §4a).
    <article className={editing ? 'pt-planpage flex flex-col flex-1' : 'pt-planpage'} data-region="plan" data-state={card.state}>
      <div className="pt-head">
        <Pill signal={card.pill.signal} word={card.pill.word} />
        <h1>{plan.title || plan.id}</h1>
        {(problems.errors > 0 || problems.warnings > 0) && (
          <ValidationBadge planId={plan.id} onSelectTask={(id) => onSelectTask(id)} />
        )}
        <span className="rk-card-when">{whenText(run, plan.id)}</span>
      </div>

      <RunChoice planId={plan.id} onError={onRequestError} />
      <RevisionDiff diff={revisionDiff} onDismiss={() => setRevisionDiff(null)} />

      {editing ? (
        <SourceEditor planId={plan.id} running={running} onClose={() => setEditing(false)} onDirtyChange={markDirty} />
      ) : revising ? (
        <PromptPanel
          mode="revise"
          planId={plan.id}
          workspace={plan.title || plan.id}
          firstRun={false}
          onDone={handleRevisionDone}
          onCancel={() => setRevising(false)}
        />
      ) : (
        <>
          <Readout readings={card.readings} />
          {card.steps.length > 0 && (
            <Sequencer steps={card.steps} edges={card.edges} selectedId={selectedTaskId} onSelect={toggleTask} />
          )}
          {selectedRow && selectedPad && (
            <TaskDetail
              planId={plan.id}
              pad={selectedPad}
              row={selectedRow}
              run={run}
              rungs={rungs ?? []}
              held={heldIds.includes(selectedRow.id)}
              onClose={() => onSelectTask(null)}
            />
          )}
        </>
      )}

      <div className="pt-line" data-region="status-line">
        {card.state === 'running' && card.now.length > 0 ? (
          <NowLines now={card.now} />
        ) : (
          <p className="pt-sentence" data-region="run-summary">
            {sentence(card, run, plan.id, problems.errors)}
          </p>
        )}
        <div className="rk-keys">
          {primary.disabled && primary.reason !== null && (
            <span data-reason className="pt-reason">
              {primary.reason}
            </span>
          )}
          {card.state === 'needs-you' && heldIds.length > 0 ? (
            <Key you data-action="review" onClick={() => onSelectTask(heldIds[0]!)}>
              review
            </Key>
          ) : primary.kind === 'cancel' ? (
            <Key data-action="cancel" disabled={primary.disabled} onClick={primary.perform}>
              {primary.label}
            </Key>
          ) : (
            <>
              {reviseKey}
              {primary.kind !== 'none' && (
                <Key
                  you={primary.kind !== 'run-again'}
                  kbd={primary.kind === 'run-again' ? undefined : 'r'}
                  data-action={primary.kind}
                  disabled={primary.disabled}
                  onClick={primary.perform}
                >
                  {primary.label}
                </Key>
              )}
            </>
          )}
        </div>
      </div>

      <Fold summary="details" data-fold="plan">
        <PlanFacts card={card} run={run} planId={plan.id} />
        <div className="pt-line" style={{ marginTop: 14 }}>
          <span className="pt-hint">The plan lives in its tasks.toml; edit it by hand here.</span>
          <Key
            size="s"
            data-action="edit"
            aria-pressed={editing}
            disabled={running}
            title={running ? 'Cannot edit while the plan is running' : undefined}
            onClick={() => {
              if (!confirmDiscard(dirty)) return;
              setEditing((e) => !e);
              setRevising(false);
            }}
          >
            edit tasks.toml
          </Key>
        </div>
      </Fold>
    </article>
  );
}

/** One line per task that runs now: "Task 2 · writing cli.py · glm-4.7 · 0:04". */
function NowLines({ now }: { now: readonly NowLine[] }) {
  return (
    <ul className="pt-now" data-region="now">
      {now.map((line) => (
        <li key={line.taskId} data-now-task={line.taskId}>
          <b>Task {line.n.replace(/^0/, '')}</b>
          <span>
            {line.doing}
            {line.model && ` · ${line.model}`}
            {line.tryN > 1 && ` · try ${line.tryN}${line.because ? `, after ${line.because}` : ''}`}
          </span>
          <span className="pt-t">{line.elapsedMs !== null ? clock(line.elapsedMs) : ''}</span>
        </li>
      ))}
    </ul>
  );
}

/** When the plan last ran: "started 10:37", or the day and time it finished. */
function whenText(run: RunState, planId: string): string {
  const live = run.plans[planId];
  if (!live?.startedAtMs) return '';
  const at = (ms: number) => {
    const d = new Date(ms);
    const hm = `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
    return new Date().toDateString() === d.toDateString() ? `today ${hm}` : `${d.getDate()} ${d.toLocaleString('en', { month: 'short' })} ${hm}`;
  };
  return live.phase === 'running' || !live.finishedAtMs ? `started ${at(live.startedAtMs).replace(/^today /, '')}` : at(live.finishedAtMs);
}

/** The state's one sentence. */
function sentence(card: Card, run: RunState, planId: string, errors: number): string {
  const total = card.rows.length;
  const n = (id: string) => card.steps.flatMap((s) => s.pads).find((p) => p.id === id)?.n.replace(/^0/, '') ?? id;
  const passed = card.rows.filter((r) => ['passed', 'already_satisfied', 'passed_with_preexisting_failures'].includes(r.status)).length;
  switch (card.state) {
    case 'ready':
      return errors > 0
        ? `This plan can’t run yet: ${count(errors, 'problem')} to fix first.`
        : 'Each task has a check your project runs. roko works on its own branch, so your files change only when you merge.';
    case 'queued': {
      const reason = waitReason(run, planId);
      if (reason?.startsWith('after ')) return `Queued: it starts ${reason}.`;
      if (reason?.startsWith('blocked: ')) return `Queued, and blocked: ${reason.slice('blocked: '.length)}, so it won’t start.`;
      return 'Queued: it starts when a slot is free.';
    }
    case 'running':
      return 'Finishing: checking the plan’s work and delivering it.';
    case 'passed': {
      const check = run.plans[planId]?.planCheck;
      return `All ${count(total, 'task')} passed their checks${check?.passed ? ', and so did the plan’s own check' : ''}.`;
    }
    case 'not-checked': {
      const unchecked = card.rows.filter((r) => r.status === 'unverified').length;
      return `${unchecked} of ${count(total, 'task')} finished without a check that judged them, so this is not a pass.`;
    }
    case 'needs-you': {
      const held = card.rows.find((r) => run.tasks[taskKey(planId, r.id)]?.phase === AWAITING_APPROVAL);
      if (held) return `Task ${n(held.id)} passed its checks and waits for your review.`;
      const forced = card.rows.find((r) => r.status === 'accepted_with_failures');
      return forced
        ? `Task ${n(forced.id)} went through with a failing check: review it before you merge.`
        : 'roko needs you to decide something.';
    }
    case 'failed': {
      const failed = card.rows.find((r) => r.status === 'failed' || r.status === 'interrupted');
      if (!failed) return 'The run failed.';
      const tries = run.tasks[taskKey(planId, failed.id)]?.tries ?? [];
      const last = tries[tries.length - 1];
      const after = tries.length > 1 ? ` after ${count(tries.length, 'try', 'tries')}` : '';
      return `Task ${n(failed.id)} failed${after}${last ? `: ${becauseOf(last)}` : ''}. Resume runs it again; what passed stays.`;
    }
    case 'interrupted':
      return `roko stopped before it finished, with ${passed} of ${count(total, 'task')} passed. Resume runs only what is left.`;
    case 'stopped':
      return `Stopped, with ${passed} of ${count(total, 'task')} passed. Resume carries on from where it stopped.`;
    default:
      return '';
  }
}

/** The fold's facts: each task's tries, model, time and spend; the plan's own check; tokens. */
function PlanFacts({ card, run, planId }: { card: Card; run: RunState; planId: string }) {
  const pads = new Map(card.steps.flatMap((s) => s.pads).map((p) => [p.id, p]));
  let tokensIn = 0;
  let tokensOut = 0;
  const lines = card.rows.map((row) => {
    const task = run.tasks[taskKey(planId, row.id)];
    tokensIn += task?.inputTokens ?? 0;
    tokensOut += task?.outputTokens ?? 0;
    const tries = task?.tries ?? [];
    return {
      id: row.id,
      label: `${pads.get(row.id)?.n ?? ''} ${row.title}`,
      tries: tries.length || (task ? 1 : 0),
      models: [...new Set(tries.map((t) => t.model).filter(Boolean))].join(', ') || row.model || '—',
      time: row.time.kind === 'actual' && row.time.ms !== null ? duration(row.time.ms) : '—',
      spent: row.costUsd ? money(row.costUsd) : '—',
    };
  });
  const check = run.plans[planId]?.planCheck;
  const kilo = (n: number) => (n >= 1_000_000 ? `${(n / 1_000_000).toFixed(1)}M` : `${Math.round(n / 1000)}k`);
  return (
    <>
      <table className="pt-table">
        <thead>
          <tr>
            <th>task</th>
            <th>tries</th>
            <th>model</th>
            <th className="n">time</th>
            <th className="n">spent</th>
          </tr>
        </thead>
        <tbody>
          {lines.map((l) => (
            <tr key={l.id}>
              <td>{l.label}</td>
              <td>{l.tries || '—'}</td>
              <td>{l.models}</td>
              <td className="n">{l.time}</td>
              <td className="n">{l.spent}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <dl className="pt-kv">
        {check && (
          <>
            <dt>plan check</dt>
            <dd>{check.passed ? 'passed' : 'failed'}</dd>
          </>
        )}
        {tokensIn + tokensOut > 0 && (
          <>
            <dt>tokens</dt>
            <dd>
              {kilo(tokensIn)} in, {kilo(tokensOut)} out
            </dd>
          </>
        )}
      </dl>
    </>
  );
}

