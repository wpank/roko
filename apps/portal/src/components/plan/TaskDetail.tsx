'use client';

/**
 * TaskDetail — one task, opened from its pad: what it does, its checks with their commands, each try with the model
 * that made it and how it ended, a review it waits for, and, folded, what the agent did and what its checks printed.
 * While the task runs, what the agent does is open, so a person watches it work.
 */

import { Key } from '@/components/kit/Key';
import { Fold } from '@/components/kit/Fold';
import { Pill } from '@/components/kit/Pill';
import type { Signal } from '@/components/kit/Led';
import { ReviewPane } from '@/components/stage/ReviewPane';
import { Checks } from '@/components/stream/Checks';
import { Transcript } from '@/components/stream/Transcript';
import type { CheckRun, RunState, TaskRun, TryRun } from '@/lib/runState';
import { taskKey } from '@/lib/runState';
import type { Pad } from '@/lib/runCard';
import { becauseOf, checkWord } from '@/lib/runCard';
import type { TaskRowModel } from '@/lib/taskRows';
import { count, duration, money } from '@/lib/words';

const PAD_PILLS: Record<Pad['state'], { signal: Signal; word: string }> = {
  waiting: { signal: 'waiting', word: 'waiting' },
  live: { signal: 'live', word: 'running' },
  pass: { signal: 'pass', word: 'passed' },
  fail: { signal: 'fail', word: 'failed' },
  you: { signal: 'you', word: 'needs you' },
  still: { signal: 'still', word: 'not checked' },
  skipped: { signal: 'still', word: 'skipped' },
};

const MARK: Record<CheckRun['status'] | 'waiting', { glyph: string; signal: Signal }> = {
  passed: { glyph: '✓', signal: 'pass' },
  failed: { glyph: '✗', signal: 'fail' },
  running: { glyph: '●', signal: 'live' },
  waiting: { glyph: '○', signal: 'waiting' },
};

export function TaskDetail({
  planId,
  pad,
  row,
  run,
  rungs,
  held,
  onClose,
}: {
  planId: string;
  pad: Pad;
  row: TaskRowModel;
  run: RunState;
  /** The project's own checks, run on every task. */
  rungs: readonly string[];
  /** Whether its verified try waits for the person's review. */
  held: boolean;
  onClose(): void;
}) {
  const key = taskKey(planId, row.id);
  const task: TaskRun | undefined = run.tasks[key];
  const tries = task?.tries ?? [];
  const checks = task?.checks ?? [];
  const pill = held ? PAD_PILLS.you : PAD_PILLS[pad.state];
  const running = pad.state === 'live';
  const agent = task?.agentId ? run.agents[task.agentId] : undefined;
  const meta = [
    tries.length > 1 ? count(tries.length, 'try', 'tries') : null,
    row.time.kind === 'actual' && row.time.ms !== null ? duration(row.time.ms) : null,
    row.costUsd ? money(row.costUsd) : null,
  ].filter(Boolean);

  const find = (word: string) => checks.find((c) => checkWord(c.phase || c.name) === word || checkWord(c.name) === word);
  const own = row.verify.map((v) => ({ word: v.phase, command: v.command, check: find(v.phase) }));
  const project = rungs.map((word) => ({ word, check: find(word) }));

  return (
    <section className="rk-module pt-task" data-region="task-detail" aria-label={`Task ${pad.n.replace(/^0/, '')}`}>
      <div className="pt-task-head">
        <span className="rk-pad-n">{pad.n}</span>
        <h2>{pad.title}</h2>
        <Pill signal={pill.signal} word={pill.word} />
        {meta.length > 0 && <span className="rk-card-when">{meta.join(' · ')}</span>}
        <Key size="s" aria-label="Close this task" onClick={onClose}>
          close
        </Key>
      </div>

      {row.description && <p className="pt-task-what">{row.description.trim()}</p>}

      <div className="pt-rows">
        <div className="pt-row">
          <span>checks</span>
          <ul>
            {own.length === 0 && project.length === 0 && <li className="pt-dim">No check judges this task.</li>}
            {own.map(({ word, command, check }) => (
              <li key={`own-${word}`}>
                <Mark status={check?.status ?? 'waiting'} />
                <b>{checkWord(word)}</b>
                {command && (
                  <>
                    {' '}
                    <span className="pt-dim">·</span> <code className="pt-code">{command}</code>
                  </>
                )}
              </li>
            ))}
            {project.map(({ word, check }) => (
              <li key={`project-${word}`}>
                <Mark status={check?.status ?? 'waiting'} />
                <b>{word}</b> <span className="pt-dim">· your project’s check, run on every task</span>
              </li>
            ))}
          </ul>
        </div>

        {tries.length > 0 && (
          <div className="pt-row">
            <span>tries</span>
            <ul>
              {tries.map((t) => (
                <TryLine key={t.attempt} t={t} />
              ))}
            </ul>
          </div>
        )}

        {pad.waits !== null && (
          <div className="pt-row">
            <span>skipped</span>
            <ul>
              <li>{pad.waits}</li>
            </ul>
          </div>
        )}
      </div>

      {held && <ReviewPane planId={planId} taskId={row.id} />}

      <Fold summary="what the agent did" open={running} data-fold="transcript">
        <Transcript
          transcript={run.transcripts[key]}
          working={running && agent?.active ? { sinceMs: agent.spawnedAtMs ?? task?.startedAtMs ?? Date.now() } : null}
          taskStatus={task === undefined ? 'pending' : task.status === 'active' ? 'active' : 'finished'}
        />
      </Fold>

      {checks.length > 0 && (
        <Fold summary="what the checks printed" open={pad.state === 'fail'} data-fold="checks">
          <Checks checks={checks} declared={row.verify} />
        </Fold>
      )}
    </section>
  );
}

function Mark({ status }: { status: CheckRun['status'] | 'waiting' }) {
  const mark = MARK[status];
  return (
    <span className="pt-g" data-state={mark.signal} aria-label={status}>
      {mark.glyph}
    </span>
  );
}

/** One try: its number, its model, what it cost and took, and how it ended. */
function TryLine({ t }: { t: TryRun }) {
  const facts = [t.model, t.costUsd !== null ? money(t.costUsd) : null, t.durationMs !== null ? duration(t.durationMs) : null]
    .filter(Boolean)
    .join(' · ');
  const ending =
    t.state === 'passed' ? 'passed' : t.state === 'running' ? 'running now' : becauseOf(t);
  return (
    <li data-try={t.attempt}>
      <Mark status={t.state === 'passed' ? 'passed' : t.state === 'failed' ? 'failed' : 'running'} />
      <b>Try {t.attempt}</b>
      {facts && <span className="pt-dim"> · {facts}</span>} · {ending}
      {t.climbed && <span className="pt-dim">; roko moved up to a stronger model</span>}
    </li>
  );
}
