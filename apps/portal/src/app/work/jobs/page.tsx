'use client';

import React, { useState, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { clsx } from 'clsx';
import { api } from '@/api/client';
import { Badge } from '@/components/atoms';
import { Spinner } from '@/components/atoms/Spinner';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type JobStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

interface JobEntry {
  id: string;
  title: string;
  description?: string;
  status: JobStatus;
  createdAt: string;
  updatedAt?: string;
  agentId?: string;
}

// ---------------------------------------------------------------------------
// Raw API shapes
// ---------------------------------------------------------------------------

interface RawJob {
  id?: string;
  title?: string;
  description?: string;
  status?: string;
  created_at?: string;
  updated_at?: string;
  agent_id?: string;
}

interface RawJobsResponse {
  jobs?: RawJob[];
}

// ---------------------------------------------------------------------------
// Mapper
// ---------------------------------------------------------------------------

function mapRawJob(raw: RawJob): JobEntry {
  const status = (raw.status ?? 'pending').toLowerCase() as JobStatus;
  return {
    id: raw.id ?? crypto.randomUUID(),
    title: raw.title ?? '(untitled)',
    description: raw.description,
    status,
    createdAt: raw.created_at ?? new Date().toISOString(),
    updatedAt: raw.updated_at,
    agentId: raw.agent_id,
  };
}

// ---------------------------------------------------------------------------
// Style maps
// ---------------------------------------------------------------------------

const STATUS_BADGE_VARIANT: Record<JobStatus, 'warning' | 'success' | 'error' | 'default' | 'info'> = {
  pending:   'default',
  running:   'warning',
  completed: 'success',
  failed:    'error',
  cancelled: 'default',
};

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

// ---------------------------------------------------------------------------
// JobRow
// ---------------------------------------------------------------------------

function JobRow({ job }: { job: JobEntry }) {
  return (
    <div
      className={clsx(
        'flex items-center gap-3 px-4 py-2.5',
        'border-b border-b-[var(--text-ghost)]',
        'hover:bg-[var(--bg-highlight)]',
        'transition-[background-color] duration-[80ms]',
      )}
    >
      <Badge variant={STATUS_BADGE_VARIANT[job.status]}>
        {job.status}
      </Badge>

      <div className="flex-1 min-w-0 flex flex-col gap-0.5">
        <span className="font-mono text-xs text-[var(--text-strong)] truncate">
          {job.title}
        </span>
        {job.description && (
          <span className="font-mono text-[10px] text-[var(--text-ghost)] truncate">
            {job.description}
          </span>
        )}
      </div>

      {job.agentId && (
        <span className="shrink-0 font-mono text-[10px] text-[var(--text-faint)] truncate max-w-[100px]">
          {job.agentId}
        </span>
      )}

      <span className="shrink-0 font-mono text-[10px] text-[var(--text-ghost)] tabular-nums">
        {formatRelativeTime(job.createdAt)}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

const STATUS_FILTER_OPTS: { label: string; value: JobStatus | 'all' }[] = [
  { label: 'All',       value: 'all'       },
  { label: 'Pending',   value: 'pending'   },
  { label: 'Running',   value: 'running'   },
  { label: 'Completed', value: 'completed' },
  { label: 'Failed',    value: 'failed'    },
];

export default function WorkJobsPage() {
  const [statusFilter, setStatusFilter] = useState<JobStatus | 'all'>('all');

  const { data, isLoading, isError } = useQuery<RawJobsResponse>({
    queryKey: ['jobs'],
    queryFn: () => api.get<RawJobsResponse>('/api/jobs'),
    staleTime: 30_000,
  });

  const jobs = useMemo<JobEntry[]>(() => {
    const raw = data?.jobs ?? [];
    return raw.map(mapRawJob);
  }, [data]);

  const visible = useMemo(
    () => statusFilter === 'all' ? jobs : jobs.filter((j) => j.status === statusFilter),
    [jobs, statusFilter],
  );

  return (
    <div className="flex flex-col h-full min-h-0">

      {/* Filter bar */}
      <div className="flex items-center gap-2 px-4 py-2 shrink-0 bg-[var(--bg-secondary)] border-b border-b-[var(--text-ghost)]">
        {STATUS_FILTER_OPTS.map((opt) => (
          <button
            key={opt.value}
            type="button"
            onClick={() => setStatusFilter(opt.value)}
            className={clsx(
              'font-mono text-[10px] px-2 py-0.5 border leading-none',
              'transition-[color,border-color] duration-[80ms]',
              statusFilter === opt.value
                ? 'text-[var(--text-strong)] border-[var(--rose-dim)]'
                : 'text-[var(--text-ghost)] border-[var(--text-ghost)] hover:text-[var(--text-muted)] hover:border-[var(--border-hover)]',
            )}
          >
            {opt.label}
          </button>
        ))}

        <span className="ml-auto font-mono text-[10px] text-[var(--text-ghost)] tabular-nums">
          {visible.length}/{jobs.length}
        </span>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto overflow-x-hidden min-h-0">
        {isLoading ? (
          <div className="flex items-center justify-center h-40 gap-2">
            <Spinner size="sm" />
            <span className="font-mono text-xs text-[var(--text-ghost)]">Loading jobs…</span>
          </div>
        ) : isError ? (
          <div className="flex flex-col items-center justify-center h-40 gap-2">
            <span className="font-mono text-xs text-[var(--accent-error)]">
              Failed to load jobs. Is roko serve running?
            </span>
          </div>
        ) : visible.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-40 gap-2 px-4 text-center">
            <span className="font-mono text-xs text-[var(--text-ghost)]">
              {jobs.length === 0
                ? 'No jobs yet. Jobs are created when agents claim marketplace work.'
                : 'No jobs match the current filter.'}
            </span>
          </div>
        ) : (
          visible.map((job) => <JobRow key={job.id} job={job} />)
        )}
      </div>
    </div>
  );
}
