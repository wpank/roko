'use client';

import React, { useState, useEffect } from 'react';
import { clsx } from 'clsx';
import { Badge } from '@/components/atoms';
import { GitBranch, GitCommit, FolderOpen, Loader } from 'lucide-react';

// ---------------------------------------------------------------------------
// Types matching the /api/git-info response
// ---------------------------------------------------------------------------

interface Commit {
  hash: string;
  shortHash: string;
  message: string;
  author: string;
  date: string;
}

interface Worktree {
  path: string;
  branch: string;
  head: string;
  locked: boolean;
  planId: string | null;
}

interface GitInfo {
  branch: string;
  commits: Commit[];
  worktrees: Worktree[];
  fetchedAt: string;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatRelativeTime(iso: string): string {
  const ms = Date.now() - new Date(iso).getTime();
  if (ms < 60_000)     return `${Math.floor(ms / 1000)}s ago`;
  if (ms < 3_600_000)  return `${Math.floor(ms / 60_000)}m ago`;
  if (ms < 86_400_000) return `${Math.floor(ms / 3_600_000)}h ago`;
  return `${Math.floor(ms / 86_400_000)}d ago`;
}

function truncatePath(path: string, maxLen = 52): string {
  if (path.length <= maxLen) return path;
  return '…' + path.slice(-(maxLen - 1));
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <div
      className={clsx(
        'flex items-center gap-2 px-4 py-1.5',
        'bg-[var(--bg-secondary)]',
        'border-b border-b-[var(--text-ghost)]',
        'sticky top-0',
      )}
    >
      <span className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest">
        {children}
      </span>
    </div>
  );
}

function CommitRow({ commit }: { commit: Commit }) {
  return (
    <div
      className={clsx(
        'flex items-start gap-3 px-4 py-2',
        'border-b border-b-[var(--text-ghost)]',
        'hover:bg-[var(--bg-highlight)]',
        'transition-[background-color] duration-[80ms]',
      )}
    >
      {/* Short hash */}
      <span className="shrink-0 font-mono text-xs tabular-nums text-[var(--rose)] w-[68px]">
        {commit.shortHash}
      </span>

      {/* Message */}
      <span className="flex-1 font-mono text-xs text-[var(--text-strong)] leading-snug truncate min-w-0">
        {commit.message}
      </span>

      {/* Author */}
      <span className="shrink-0 font-mono text-xs text-[var(--text-muted)] ml-2 truncate max-w-[100px]">
        {commit.author}
      </span>

      {/* Age */}
      <span className="shrink-0 font-mono text-xs text-[var(--text-ghost)] tabular-nums ml-2 w-[60px] text-right">
        {formatRelativeTime(commit.date)}
      </span>
    </div>
  );
}

function WorktreeRow({ wt, mainBranch }: { wt: Worktree; mainBranch: string }) {
  const isMain = wt.branch === mainBranch;

  return (
    <div
      className={clsx(
        'flex items-start gap-3 px-4 py-2.5',
        'border-b border-b-[var(--text-ghost)]',
        'hover:bg-[var(--bg-highlight)]',
        'transition-[background-color] duration-[80ms]',
      )}
    >
      {/* Path */}
      <span className="flex-1 font-mono text-xs text-[var(--text-faint)] leading-snug truncate min-w-0">
        {truncatePath(wt.path)}
      </span>

      {/* Branch */}
      <div className="shrink-0 flex items-center gap-1 ml-2">
        <GitBranch size={11} className="text-[var(--rose-dim)]" aria-hidden />
        <span className="font-mono text-xs text-[var(--rose)] truncate max-w-[180px]">{wt.branch}</span>
      </div>

      {/* Head hash */}
      <span className="shrink-0 font-mono text-xs text-[var(--text-ghost)] tabular-nums ml-3">
        {wt.head}
      </span>

      {/* Plan link */}
      {wt.planId && (
        <span className="shrink-0 font-mono text-xs text-[var(--accent-cyan)] ml-2">
          {wt.planId}
        </span>
      )}

      {/* Status badge */}
      <div className="shrink-0 ml-2">
        {isMain ? (
          <Badge variant="default">main</Badge>
        ) : wt.locked ? (
          <Badge variant="info">locked</Badge>
        ) : (
          <Badge variant="default">free</Badge>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function ObserveGitPage() {
  const [gitInfo, setGitInfo] = useState<GitInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error,   setError]   = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);

    fetch('/api/git-info')
      .then((r) => {
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return r.json() as Promise<GitInfo>;
      })
      .then((data) => {
        if (!cancelled) {
          setGitInfo(data);
          setLoading(false);
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
          setLoading(false);
        }
      });

    return () => { cancelled = true; };
  }, []);

  const branch    = gitInfo?.branch ?? '—';
  const commits   = gitInfo?.commits ?? [];
  const worktrees = gitInfo?.worktrees ?? [];

  return (
    <div className="flex flex-col min-h-0">

      {/* ---------------------------------------------------------------- */}
      {/* Current branch banner                                             */}
      {/* ---------------------------------------------------------------- */}
      <div
        className={clsx(
          'flex items-center gap-4 px-4 py-3 shrink-0',
          'border-b border-b-[var(--text-ghost)]',
          'bg-[var(--bg-secondary)]',
        )}
      >
        {loading ? (
          <div className="flex items-center gap-2 text-[var(--text-ghost)]">
            <Loader size={13} strokeWidth={1.5} className="animate-spin" aria-hidden />
            <span className="font-mono text-xs">Loading git info…</span>
          </div>
        ) : error ? (
          <span className="font-mono text-xs text-[var(--accent-error)]">
            Failed to load git info: {error}
          </span>
        ) : (
          <>
            <div className="flex items-center gap-2">
              <GitBranch
                size={14}
                strokeWidth={1.5}
                className="shrink-0"
                style={{ color: 'var(--rose)' }}
                aria-hidden
              />
              <span className="font-mono text-sm text-[var(--rose)]">
                {branch}
              </span>
            </div>

            <span className="w-px h-5 bg-[var(--text-ghost)]" aria-hidden />

            {commits[0] && (
              <div className="flex items-center gap-2">
                <GitCommit
                  size={13}
                  strokeWidth={1.5}
                  className="shrink-0"
                  style={{ color: 'var(--text-faint)' }}
                  aria-hidden
                />
                <span className="font-mono text-xs text-[var(--text-faint)] tabular-nums">
                  {commits[0].shortHash}
                </span>
                <span className="font-mono text-xs text-[var(--text-muted)] truncate max-w-xs">
                  {commits[0].message}
                </span>
              </div>
            )}

            <div className="ml-auto flex items-center gap-2">
              <span className="font-mono text-xs text-[var(--text-ghost)]">
                {worktrees.filter((w) => w.locked).length} locked worktrees
              </span>
              {gitInfo && (
                <span className="font-mono text-xs text-[var(--text-ghost)] tabular-nums">
                  {formatRelativeTime(gitInfo.fetchedAt)}
                </span>
              )}
            </div>
          </>
        )}
      </div>

      {/* ---------------------------------------------------------------- */}
      {/* Recent commits                                                    */}
      {/* ---------------------------------------------------------------- */}
      {!loading && !error && (
        <>
          <SectionLabel>Recent commits</SectionLabel>

          {commits.length === 0 ? (
            <div className="flex items-center justify-center h-16">
              <span className="font-mono text-xs text-[var(--text-ghost)]">No commits found.</span>
            </div>
          ) : (
            commits.map((commit) => (
              <CommitRow key={commit.hash} commit={commit} />
            ))
          )}

          {/* ---------------------------------------------------------------- */}
          {/* Worktrees                                                         */}
          {/* ---------------------------------------------------------------- */}
          <SectionLabel>
            <FolderOpen size={10} className="inline mr-1.5 relative -top-px" aria-hidden />
            Worktrees
          </SectionLabel>

          {worktrees.length === 0 ? (
            <div className="flex items-center justify-center h-16">
              <span className="font-mono text-xs text-[var(--text-ghost)]">
                No worktrees found.
              </span>
            </div>
          ) : (
            worktrees.map((wt) => (
              <WorktreeRow key={wt.path} wt={wt} mainBranch={branch} />
            ))
          )}
        </>
      )}

      {/* ---------------------------------------------------------------- */}
      {/* Footer note                                                       */}
      {/* ---------------------------------------------------------------- */}
      <div
        className={clsx(
          'px-4 py-3 mt-auto',
          'border-t border-t-[var(--text-ghost)]',
          'bg-[var(--bg-secondary)]',
        )}
      >
        <p className="font-mono text-xs text-[var(--text-ghost)] leading-relaxed">
          Data from local git. Worktrees are created per-task under
          <span className="text-[var(--text-faint)]"> .roko/state/worktrees/</span> and locked
          while a plan task is executing. Clean up via <span className="text-[var(--text-faint)]">roko doctor disk</span>.
        </p>
      </div>
    </div>
  );
}
