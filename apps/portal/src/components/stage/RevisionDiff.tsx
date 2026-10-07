import type { WireKeyChange, WirePlanDiff, WireTaskChange } from '@/api/contracts';
import { Notice } from '@/components/primitives/Notice';

/**
 * RevisionDiff — what a revision changed in the plan (3229, G63): the tasks
 * it added and removed, and each changed task's fields with before and after,
 * verify-command and `files` changes first, since a dropped check or a
 * widened scope is what the operator must not miss. PlanView shows it under
 * the plan header after a revision lands, until dismissed. An empty or absent
 * diff renders nothing.
 */

/** Fields listed first in a changed task. */
const FIRST = ['verify', 'files'];

function rank(key: string): number {
  const index = FIRST.indexOf(key);
  return index === -1 ? FIRST.length : index;
}

/** Whether `diff` changes nothing. */
export function isEmptyDiff(diff: WirePlanDiff | null | undefined): boolean {
  return (
    !diff ||
    (diff.meta.length === 0 &&
      diff.added.length === 0 &&
      diff.removed.length === 0 &&
      diff.changed.length === 0)
  );
}

function Field({ change }: { change: WireKeyChange }) {
  return (
    <li data-field={change.key}>
      <code>{change.key}</code>: <del>{change.before ?? '(none)'}</del> →{' '}
      <ins>{change.after ?? '(none)'}</ins>
    </li>
  );
}

export function RevisionDiff({
  diff,
  onDismiss,
}: {
  diff: WirePlanDiff | null | undefined;
  onDismiss?: () => void;
}) {
  if (!diff || isEmptyDiff(diff)) return null;
  const changed: WireTaskChange[] =
    diff.meta.length > 0 ? [{ id: '[meta]', keys: diff.meta }] : [];
  changed.push(...diff.changed);
  return (
    <div data-revision-diff>
      <Notice kind="info" onClose={onDismiss}>
        <span>The revision changed the plan:</span>
        <ul className="mt-1 space-y-1">
          {diff.added.map((id) => (
            <li key={`added-${id}`} data-diff="added" data-task={id}>
              + {id} added
            </li>
          ))}
          {diff.removed.map((id) => (
            <li key={`removed-${id}`} data-diff="removed" data-task={id}>
              − {id} removed
            </li>
          ))}
          {changed.map((task) => (
            <li key={`changed-${task.id}`} data-diff="changed" data-task={task.id}>
              ~ {task.id}
              <ul className="ml-4">
                {[...task.keys]
                  .sort((a, b) => rank(a.key) - rank(b.key))
                  .map((change) => (
                    <Field key={change.key} change={change} />
                  ))}
              </ul>
            </li>
          ))}
        </ul>
      </Notice>
    </div>
  );
}
