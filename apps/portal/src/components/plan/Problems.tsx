'use client';

/**
 * Problems — why a plan can't run, one line per problem, in words (gap-6c4d24): `✗ Task 2 has no check, so roko could
 * never tell when it's done.` with a `show` key that opens the task. What roko's checker said, rule ids and all,
 * folds under the list.
 */

import React from 'react';
import { Fold } from '@/components/kit/Fold';
import { Key } from '@/components/kit/Key';
import type { Problem } from '@/lib/planProblems';

/** `text` with its backticked parts as code. */
function withCode(text: string): React.ReactNode[] {
  return text.split(/(`[^`]+`)/).map((part, i) =>
    part.startsWith('`') && part.endsWith('`') && part.length > 1 ? (
      <code key={i} className="pt-code">
        {part.slice(1, -1)}
      </code>
    ) : (
      part
    ),
  );
}

/** The line's lead (`Task 2 has no check,`) bold, the rest plain. */
function Line({ text }: { text: string }) {
  const comma = text.search(/[,:](\s|$)/);
  if (comma === -1 || comma > 60) return <>{withCode(text)}</>;
  return (
    <>
      <b>{withCode(text.slice(0, comma + 1))}</b>
      {withCode(text.slice(comma + 1))}
    </>
  );
}

export function Problems({ problems, onShow }: { problems: readonly Problem[]; onShow(taskId: string): void }) {
  return (
    <div className="pt-problems" data-region="problems">
      <ul>
        {problems.map((problem, i) => (
          <li key={i} data-problem-line data-severity={problem.severity}>
            <span className="pt-g" data-state={problem.severity === 'error' ? 'fail' : 'you'} aria-hidden="true">
              {problem.severity === 'error' ? '✗' : '!'}
            </span>
            <span>
              <Line text={problem.text} />
            </span>
            {problem.taskId && (
              <Key size="s" data-action="show-problem" onClick={() => onShow(problem.taskId!)}>
                show
              </Key>
            )}
          </li>
        ))}
      </ul>
      <Fold summary="what roko’s checker said" data-fold="problems">
        <ul className="pt-raw-list">
          {problems.flatMap((problem) => problem.raw).map((line, i) => (
            <li key={i}>
              <code className="pt-code">{line}</code>
            </li>
          ))}
        </ul>
      </Fold>
    </div>
  );
}
