/**
 * GET /api/git-info
 *
 * Returns real git state for the roko workspace: current branch, recent
 * commits, and active worktrees.
 */
import { NextResponse } from 'next/server';

export const dynamic = 'force-dynamic';
export const runtime = 'nodejs';

export async function GET() {
  // Use require() to avoid Turbopack static analysis treating this as edge code.
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const { spawnSync } = require('child_process') as typeof import('child_process');

  function gitSync(args: string[]): string {
    const result = spawnSync('git', args, {
      cwd: process.cwd(),
      encoding: 'utf8',
      timeout: 5000,
    });
    if (result.error || result.status !== 0) return '';
    return (result.stdout ?? '').trim();
  }

  const branch      = gitSync(['rev-parse', '--abbrev-ref', 'HEAD']) || 'unknown';
  const logRaw      = gitSync(['log', '--format=%H\t%h\t%s\t%an\t%ai', '-10']);
  const worktreeRaw = gitSync(['worktree', 'list', '--porcelain']);

  // Parse commits
  const commits = logRaw
    .split('\n')
    .filter(Boolean)
    .map((line) => {
      const [hash, shortHash, message, author, date] = line.split('\t');
      return { hash: hash ?? '', shortHash: shortHash ?? '', message: message ?? '', author: author ?? '', date: date ?? new Date().toISOString() };
    });

  // Parse worktree porcelain format
  type WorktreeEntry = { path: string; branch: string; head: string; locked: boolean; planId: string | null };
  const worktrees: WorktreeEntry[] = [];
  const cwd = process.cwd();

  for (const block of worktreeRaw.split('\n\n').filter(Boolean)) {
    const lines = block.split('\n').filter(Boolean);
    const pathLine   = lines.find((l) => l.startsWith('worktree '));
    const headLine   = lines.find((l) => l.startsWith('HEAD '));
    const branchLine = lines.find((l) => l.startsWith('branch '));
    const lockedLine = lines.find((l) => l.startsWith('locked'));
    const prunable   = lines.find((l) => l.startsWith('prunable'));
    if (!pathLine || prunable) continue;

    const wtPath    = pathLine.slice('worktree '.length);
    const wtHead    = (headLine?.slice('HEAD '.length) ?? '').slice(0, 7);
    const rawBranch = branchLine?.replace(/^branch refs\/heads\//, '') ?? 'detached';
    const planMatch = rawBranch.match(/^plan\/([^/]+)\//);

    worktrees.push({
      path:   wtPath.startsWith(cwd) ? '.' + wtPath.slice(cwd.length) : wtPath,
      branch: rawBranch,
      head:   wtHead,
      locked: Boolean(lockedLine),
      planId: planMatch?.[1] ?? null,
    });
  }

  return NextResponse.json({ branch, commits, worktrees, fetchedAt: new Date().toISOString() });
}
