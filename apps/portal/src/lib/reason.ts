/**
 * reason.ts — why a failed check failed, from its output (gap-6d2dd2): the line a person would look for, not the
 * output's first line. A Python traceback ends on its error, at its last frame inside the project; a test runner
 * names the test that failed; a compiler names a file and a line; anything else ends on its last line. Paths inside
 * the project are shown as the project names them, without the try's worktree.
 */

import { digestOutput } from '@/lib/checks';

export interface Reason {
  /** Where it stopped (`settings.py, line 2`), or null when the output doesn't say. */
  at: string | null;
  /** What went wrong (`'(' was never closed`). */
  message: string;
}

/** A path inside a try's worktree, or inside the project at `root`, as the project names it. */
export function projectPath(path: string, root?: string | null): string {
  const inWorktree = path.match(/\/\.roko\/worktrees\/[^/]+\/(.+)$/);
  if (inWorktree) return inWorktree[1]!;
  const base = root?.replace(/\/+$/, '');
  return base && path.startsWith(`${base}/`) ? path.slice(base.length + 1) : path;
}

/** `text` with every path inside a try's worktree, or inside `root`, written as the project names it. */
function projectPaths(text: string, root?: string | null): string {
  const base = root?.replace(/\/+$/, '');
  let out = text.replace(/(?:\/[^\s"'/]+)*\/\.roko\/worktrees\/[^/\s"']+\//g, '');
  if (base) out = out.split(`${base}/`).join('');
  return out;
}

/** Python's frame line: `  File "settings.py", line 2, in load`. */
const PY_FRAME = /^\s*File "(.+)", line (\d+)/;
/** Python's last line: `SyntaxError: invalid syntax`, `ModuleNotFoundError: No module named 'x'`, `AssertionError`. */
const PY_ERROR = /^([A-Za-z_][\w.]*(?:Error|Exception|Exit|Interrupt|Warning)|KeyboardInterrupt|StopIteration)(?::\s*(.*))?$/;
/** pytest's short summary: `FAILED tests/test_x.py::test_y - AssertionError: assert 1 == 2`. */
const PYTEST_FAILED = /^FAILED (\S+)(?: - (.+))?$/;
/** The runner's own lines: the command, its closing line, dropped lines and the stderr separator. */
const RUNNER_LINE = /^(\$ |✗ |… |---stderr---$|(exit status|timed out after) )/;

/** Why the check whose output is `output` failed, or null when it printed nothing to go on. */
export function failureReason(output: string, root?: string | null): Reason | null {
  const lines = output.split('\n').map((line) => line.replace(/\s+$/, ''));

  // A Python traceback: its last error line, at the last frame that is a file.
  const errorAt = findLast(lines, (line) => PY_ERROR.test(line));
  if (errorAt !== -1 && lines.slice(0, errorAt).some((line) => PY_FRAME.test(line))) {
    const [, name, message] = lines[errorAt]!.match(PY_ERROR)!;
    const frame = lines
      .slice(0, errorAt)
      .map((line) => line.match(PY_FRAME))
      .filter((m): m is RegExpMatchArray => m !== null && !m[1]!.startsWith('<'))
      .pop();
    return {
      at: frame ? `${projectPath(frame[1]!, root)}, line ${frame[2]}` : null,
      message: projectPaths(message?.trim() || name!, root),
    };
  }

  // pytest names the test that failed.
  const failedAt = findLast(lines, (line) => PYTEST_FAILED.test(line));
  if (failedAt !== -1) {
    const [, test, message] = lines[failedAt]!.match(PYTEST_FAILED)!;
    return { at: projectPath(test!, root), message: projectPaths(message?.trim() || 'the test failed', root) };
  }

  // A compiler or a panic names a file and a line.
  const digest = digestOutput(output);
  const entry = digest.groups[0]?.entries.find((e) => e.severity === 'error');
  if (digest.groups[0] && entry) {
    const line = entry.line === null ? '' : `, line ${entry.line}`;
    return { at: `${projectPath(digest.groups[0].file, root)}${line}`, message: projectPaths(entry.message, root) };
  }

  // Anything else ends on its last line.
  const last = findLast(lines, (line) => line.trim() !== '' && !RUNNER_LINE.test(line.trim()));
  return last === -1 ? null : { at: null, message: projectPaths(lines[last]!.trim(), root) };
}

function findLast(lines: readonly string[], test: (line: string) => boolean): number {
  for (let i = lines.length - 1; i >= 0; i -= 1) if (test(lines[i]!)) return i;
  return -1;
}
