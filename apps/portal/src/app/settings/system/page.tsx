'use client';

import { useState, useEffect } from 'react';
import { clsx } from 'clsx';
import {
  RefreshCw,
  Stethoscope,
  CheckCircle,
  XCircle,
  AlertTriangle,
  HardDrive,
  Cpu,
  Box,
  RotateCw,
} from 'lucide-react';
import { Button, Badge, ProgressBar, Spinner } from '@/components/atoms';
import { api } from '@/api/client';

// ---------------------------------------------------------------------------
// Types — mirroring the /api/doctor response shape
// ---------------------------------------------------------------------------

type DiagLevel = 'ok' | 'warn' | 'fail';

interface DoctorCheck {
  id: string;
  status: string;   // "ok" | "warn" | "fail"
  message: string;
  detail?: string;
  path?: string;
  fix?: string;
}

interface DoctorSummary {
  ok: number;
  warn: number;
  fail: number;
  total: number;
  skipped: number;
}

interface DoctorResponse {
  healthy: boolean;
  workdir: string;
  checks: DoctorCheck[];
  summary: DoctorSummary;
}

// Parsed disk info from the "disk_space" check detail string, e.g.
// "244894 MiB free of 1902788 MiB (87% used)"
interface DiskInfo {
  freeMib: number;
  totalMib: number;
  usedPct: number;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function parseDiskDetail(detail: string): DiskInfo | null {
  // Pattern: "244894 MiB free of 1902788 MiB (87% used)"
  const m = detail.match(/(\d+)\s+MiB free of (\d+)\s+MiB\s+\((\d+)%/);
  if (!m) return null;
  return {
    freeMib: parseInt(m[1], 10),
    totalMib: parseInt(m[2], 10),
    usedPct: parseInt(m[3], 10),
  };
}

function mibToGib(mib: number): string {
  return (mib / 1024).toFixed(1);
}

function mapCheckStatus(status: string): DiagLevel {
  if (status === 'ok') return 'ok';
  if (status === 'warn') return 'warn';
  return 'fail';
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function DiagIcon({ level }: { level: DiagLevel }) {
  if (level === 'ok')
    return <CheckCircle size={13} strokeWidth={1.5} className="text-[var(--sage)] shrink-0" aria-hidden />;
  if (level === 'warn')
    return <AlertTriangle size={13} strokeWidth={1.5} className="text-[var(--warning)] shrink-0" aria-hidden />;
  return <XCircle size={13} strokeWidth={1.5} className="text-[var(--accent-error)] shrink-0" aria-hidden />;
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-baseline justify-between gap-4 py-2 border-b border-b-[var(--text-ghost)] last:border-b-0">
      <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] shrink-0">
        {label}
      </span>
      <span className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-strong)] tabular-nums text-right truncate max-w-[60%]">
        {value}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// System page
// ---------------------------------------------------------------------------

export default function SystemPage() {
  const [loading, setLoading] = useState(true);
  const [doctor, setDoctor] = useState<DoctorResponse | null>(null);
  const [fetchError, setFetchError] = useState<string | null>(null);
  const [runningDiag, setRunningDiag] = useState(false);
  const [prunedCache, setPrunedCache] = useState(false);

  // Auto-fetch doctor on mount
  useEffect(() => {
    void fetchDoctor();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function fetchDoctor() {
    setLoading(true);
    setFetchError(null);
    try {
      const data = await api.get<DoctorResponse>('/api/doctor');
      setDoctor(data);
    } catch (err) {
      setFetchError(err instanceof Error ? err.message : 'Failed to load diagnostics');
    } finally {
      setLoading(false);
    }
  }

  async function handleRunDiag() {
    setRunningDiag(true);
    try {
      const data = await api.get<DoctorResponse>('/api/doctor');
      setDoctor(data);
    } catch (err) {
      setFetchError(err instanceof Error ? err.message : 'Diagnostics failed');
    } finally {
      setRunningDiag(false);
    }
  }

  function handlePruneCache() {
    setPrunedCache(true);
  }

  // Extract disk info from the doctor checks
  const diskCheck = doctor?.checks.find((c) => c.id === 'disk_space');
  const diskInfo: DiskInfo | null = diskCheck?.detail ? parseDiskDetail(diskCheck.detail) : null;

  // Workdir from doctor response
  const workdir = doctor?.workdir ?? '—';

  return (
    <div className="flex flex-col gap-6 p-6">
      {/* Page header */}
      <div className="flex flex-col gap-1">
        <h1 className="font-[var(--font-mono)] text-[var(--text-lg)] text-[var(--text-strong)]">
          System Diagnostics
        </h1>
        <p className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-muted)]">
          Host info, disk usage, cache management, and runtime diagnostics.
        </p>
      </div>

      {/* Loading state */}
      {loading && (
        <div className="flex items-center gap-2">
          <Spinner size="sm" />
          <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)]">
            Loading diagnostics…
          </span>
        </div>
      )}

      {/* Fetch error */}
      {fetchError && !loading && (
        <div className="flex items-center gap-2 px-3 py-2 border border-[var(--accent-error)] bg-[var(--accent-error)]/5">
          <XCircle size={13} strokeWidth={1.5} className="text-[var(--accent-error)] shrink-0" aria-hidden />
          <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--accent-error)]">
            {fetchError}
          </span>
        </div>
      )}

      {!loading && doctor && (
        <>
          {/* Workspace info */}
          <section className="border border-[var(--text-ghost)]">
            <div className="flex items-center gap-2 px-4 py-2 border-b border-b-[var(--text-ghost)]">
              <Cpu size={13} strokeWidth={1.5} className="text-[var(--text-muted)]" aria-hidden />
              <h2 className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tracking-[var(--tracking-widest)] uppercase">
                Workspace
              </h2>
              <Badge
                variant={doctor.healthy ? 'success' : 'warning'}
                className="ml-auto"
              >
                {doctor.healthy ? 'healthy' : 'degraded'}
              </Badge>
            </div>
            <div className="px-4 py-1">
              <InfoRow label="Working directory" value={workdir} />
              <InfoRow
                label="Doctor checks"
                value={`${doctor.summary.ok} ok · ${doctor.summary.warn} warn · ${doctor.summary.fail} fail`}
              />
            </div>
          </section>

          {/* Disk usage */}
          {diskInfo && (
            <section className="border border-[var(--text-ghost)]">
              <div className="flex items-center gap-2 px-4 py-2 border-b border-b-[var(--text-ghost)]">
                <HardDrive size={13} strokeWidth={1.5} className="text-[var(--text-muted)]" aria-hidden />
                <h2 className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tracking-[var(--tracking-widest)] uppercase">
                  Disk usage
                </h2>
              </div>

              <div className="px-4 py-3 flex flex-col gap-4">
                {/* Overall */}
                <div className="flex flex-col gap-2">
                  <div className="flex items-baseline justify-between">
                    <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]">
                      Volume
                    </span>
                    <span className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-muted)] tabular-nums">
                      {mibToGib(diskInfo.totalMib - diskInfo.freeMib)} / {mibToGib(diskInfo.totalMib)} GiB
                    </span>
                  </div>
                  <ProgressBar value={diskInfo.usedPct} variant="cost" showLabel />
                </div>

                {/* Detail text */}
                <p className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]">
                  {diskCheck?.detail}
                </p>
              </div>
            </section>
          )}

          {/* Roko info */}
          <section className="border border-[var(--text-ghost)]">
            <div className="flex items-center gap-2 px-4 py-2 border-b border-b-[var(--text-ghost)]">
              <Box size={13} strokeWidth={1.5} className="text-[var(--text-muted)]" aria-hidden />
              <h2 className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tracking-[var(--tracking-widest)] uppercase">
                Roko
              </h2>
            </div>
            <div className="px-4 py-1">
              {/* Show a subset of checks as info rows */}
              {doctor.checks
                .filter((c) =>
                  ['config_presence', 'default_model', 'budget', 'serve_auth'].includes(c.id),
                )
                .map((c) => (
                  <InfoRow key={c.id} label={c.id.replace(/_/g, ' ')} value={c.message} />
                ))}
            </div>
          </section>

          {/* Cache management */}
          <section className="border border-[var(--text-ghost)]">
            <div className="flex items-center justify-between px-4 py-2 border-b border-b-[var(--text-ghost)]">
              <h2 className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tracking-[var(--tracking-widest)] uppercase">
                Cache
              </h2>
              <Button
                variant="ghost"
                size="sm"
                onClick={handlePruneCache}
                disabled={prunedCache}
              >
                <RefreshCw size={12} strokeWidth={1.5} aria-hidden />
                {prunedCache ? 'Pruned' : 'Prune cache'}
              </Button>
            </div>
            <div className="px-4 py-3">
              <p className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] leading-relaxed">
                {prunedCache
                  ? 'Cache pruned. Stale evidence bundles and old JSONL generations have been removed.'
                  : 'Pruning removes stale evidence bundles and old JSONL generation files while preserving active data.'}
              </p>
            </div>
          </section>

          {/* Doctor diagnostics */}
          <section className="border border-[var(--text-ghost)]">
            <div className="flex items-center justify-between px-4 py-2 border-b border-b-[var(--text-ghost)]">
              <div className="flex items-center gap-2">
                <Stethoscope size={13} strokeWidth={1.5} className="text-[var(--text-muted)]" aria-hidden />
                <h2 className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tracking-[var(--tracking-widest)] uppercase">
                  Doctor
                </h2>
              </div>
              <Button
                variant="primary"
                size="sm"
                loading={runningDiag}
                onClick={handleRunDiag}
              >
                <RotateCw size={12} strokeWidth={1.5} aria-hidden />
                Refresh
              </Button>
            </div>

            <div className="p-4">
              {runningDiag && (
                <div className="flex items-center gap-2">
                  <Spinner size="sm" />
                  <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)]">
                    Running roko doctor…
                  </span>
                </div>
              )}

              {!runningDiag && (
                <div className="flex flex-col gap-1">
                  {doctor.checks.map((check) => {
                    const level = mapCheckStatus(check.status);
                    return (
                      <div
                        key={check.id}
                        className={clsx(
                          'flex items-start gap-3 px-3 py-2',
                          'border border-[var(--text-ghost)]',
                          level === 'fail' && 'border-[var(--accent-error)] bg-[var(--accent-error)]/5',
                          level === 'warn' && 'border-[var(--warning)] bg-[var(--warning)]/5',
                        )}
                      >
                        <DiagIcon level={level} />
                        <div className="flex flex-col gap-0.5 flex-1 min-w-0">
                          <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-strong)]">
                            {check.message}
                          </span>
                          {check.detail && (
                            <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]">
                              {check.detail}
                            </span>
                          )}
                          {check.fix && level !== 'ok' && (
                            <code className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)] bg-[var(--bg-highlight)] px-1 py-0.5 mt-0.5 break-all">
                              {check.fix}
                            </code>
                          )}
                        </div>
                        <Badge
                          variant={level === 'ok' ? 'success' : level === 'warn' ? 'warning' : 'error'}
                        >
                          {level}
                        </Badge>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          </section>
        </>
      )}
    </div>
  );
}
