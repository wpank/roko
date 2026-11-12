'use client';

/**
 * Result — where a passed run's work went (gap-6d2dd2), from the run summary's `result`: the sentence ("Ready to
 * merge: 3 files changed, +14 lines, on its own branch. Your checkout hasn't changed."), the command that takes the
 * work with a copy key, and, behind the orange review key, every file the run changed with its patch folded.
 */

import { useState } from 'react';
import type { WireRunResult } from '@/api/contracts';
import { useRunDiff } from '@/api/queries';
import { Key } from '@/components/kit/Key';
import { ChangeList } from '@/components/plan/Changes';
import { count } from '@/lib/words';

/** Whether the run left work to look at and take: it changed files the checkout doesn't have yet. */
export function hasWorkToTake(result: WireRunResult | null | undefined): result is WireRunResult {
  return !!result && !result.merged && (result.diffstat?.files ?? 0) > 0;
}

/** `3 files changed, +14 lines` (with `−2` when it removed some). */
function changeWords(result: WireRunResult): string {
  const stat = result.diffstat;
  if (!stat) return 'its changes';
  const lines = [stat.insertions > 0 ? `+${stat.insertions}` : null, stat.deletions > 0 ? `−${stat.deletions}` : null]
    .filter(Boolean)
    .join(' ');
  return `${count(stat.files, 'file')} changed${lines ? `, ${lines} lines` : ''}`;
}

/** The sentence for a passed run whose `result` is known. `branch` is the checkout's branch, when known. */
export function ResultSentence({ result, branch }: { result: WireRunResult; branch: string | null }) {
  if (result.merged) {
    const files = result.diffstat ? `: ${count(result.diffstat.files, 'file')} changed` : '';
    return (
      <>
        <span className="pt-g" data-state="pass" aria-hidden="true">
          ✓
        </span>{' '}
        <b>Merged into {branch ?? 'your checkout'}</b>
        {files}.
      </>
    );
  }
  if (!hasWorkToTake(result)) return <>The run passed, but it changed no files.</>;
  return (
    <>
      <span className="pt-g" data-state="pass" aria-hidden="true">
        ✓
      </span>{' '}
      <b>{result.partial ? 'Part of the work is ready to merge:' : 'Ready to merge:'}</b> {changeWords(result)}, on its
      own branch. Your checkout hasn’t changed.
    </>
  );
}

/** The command that takes the work, and a key that copies it. */
export function MergeRow({ command }: { command: string }) {
  const [copied, setCopied] = useState(false);
  const copy = () => {
    void navigator.clipboard?.writeText(command).then(
      () => {
        setCopied(true);
        setTimeout(() => setCopied(false), 2_000);
      },
      () => undefined,
    );
  };
  return (
    <div className="pt-merge" data-region="merge">
      <span className="rk-label">merge</span>
      <code className="pt-code" data-merge-command>
        {command}
      </code>
      <Key size="s" data-action="copy-merge" onClick={copy}>
        {copied ? 'copied' : 'copy'}
      </Key>
    </div>
  );
}

/**
 * Every file run `runId` changed, each with its patch folded: what the review key opens. `banner` is why the work
 * needs a closer look (unchecked tasks, part of the work), shown above the files.
 */
export function RunChanges({ runId, banner, onClose }: { runId: string; banner?: string | null; onClose(): void }) {
  const { data: diff, isError } = useRunDiff(runId);
  const totals = diff?.totals;
  const totalsWords = totals
    ? [
        count(totals.files, 'file'),
        totals.additions > 0 ? `+${totals.additions}` : null,
        totals.deletions > 0 ? `−${totals.deletions}` : null,
      ]
        .filter(Boolean)
        .join(' · ')
    : null;
  return (
    <section className="rk-module pt-task" data-region="changes" aria-label="What the run changed">
      <div className="pt-task-head">
        <h2>What the run changed</h2>
        {totalsWords && <span className="rk-card-when">{totalsWords}</span>}
        <Key size="s" aria-label="Close the changes" onClick={onClose}>
          close
        </Key>
      </div>
      {banner && (
        <p className="pt-banner" data-banner>
          {banner}
        </p>
      )}
      {diff ? (
        <>
          <p className="pt-task-what">
            On <code className="pt-code">{diff.branch}</code>. Open a file to see its change.
          </p>
          <ChangeList files={diff.files} />
          {diff.truncated && <p className="pt-dim">Only the first {count(diff.files.length, 'file')} are shown.</p>}
        </>
      ) : (
        <p className="pt-dim">{isError ? 'roko serve could not read the run’s changes.' : 'Reading the run’s changes…'}</p>
      )}
    </section>
  );
}
