'use client';

import { useState, useEffect, useCallback } from 'react';
import { clsx } from 'clsx';
import {
  Plus,
  Trash2,
  Edit2,
  Wifi,
  WifiOff,
  Search,
  CheckCircle,
  XCircle,
} from 'lucide-react';
import { Button, Badge, StatusLED, Spinner } from '@/components/atoms';
import { api } from '@/api/client';
import { setRokoServeUrl } from '@/lib/env';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type ConnStatus = 'connected' | 'connecting' | 'disconnected' | 'error';

interface ConnectionProfile {
  id: string;
  name: string;
  url: string;
  apiKey?: string;
  isLocal: boolean;
  isActive: boolean;
  latencyMs: number | null;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function connStatusToLed(s: ConnStatus): 'active' | 'idle' | 'error' | 'offline' {
  if (s === 'connected') return 'active';
  if (s === 'connecting') return 'idle';
  if (s === 'error') return 'error';
  return 'offline';
}

/** Ping a roko-serve URL and return latency in ms, or null on failure. */
async function pingServer(url: string): Promise<number | null> {
  const healthUrl = `${url.replace(/\/+$/, '')}/api/health`;
  const start = performance.now();
  try {
    const resp = await fetch(healthUrl, {
      method: 'GET',
      signal: AbortSignal.timeout(5000),
    });
    if (!resp.ok) return null;
    return Math.round(performance.now() - start);
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// CurrentConnectionCard
// ---------------------------------------------------------------------------

interface CurrentConnectionCardProps {
  profile: ConnectionProfile | null;
  status: ConnStatus;
  onDetect: () => void;
  detecting: boolean;
}

function CurrentConnectionCard({ profile, status, onDetect, detecting }: CurrentConnectionCardProps) {
  const StatusIcon = status === 'connected' ? CheckCircle : status === 'error' ? XCircle : WifiOff;

  return (
    <div className="border border-[var(--text-ghost)]">
      {/* Header */}
      <div className="flex items-center gap-2 px-4 py-2 border-b border-b-[var(--text-ghost)]">
        <Wifi size={13} strokeWidth={1.5} className="text-[var(--text-muted)]" aria-hidden />
        <h2 className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tracking-[var(--tracking-widest)] uppercase">
          Current connection
        </h2>
      </div>

      <div className="px-4 py-4 flex flex-col gap-3">
        {profile ? (
          <>
            <div className="flex items-center justify-between gap-4">
              <div className="flex items-center gap-3">
                <StatusLED status={connStatusToLed(status)} pulse={status === 'connecting'} />
                <div className="flex flex-col gap-0.5">
                  <span className="font-[var(--font-mono)] text-[var(--text-base)] text-[var(--text-strong)]">
                    {profile.name}
                  </span>
                  <code className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)]">
                    {profile.url}
                  </code>
                </div>
              </div>

              <div className="flex items-center gap-2 shrink-0">
                {profile.latencyMs !== null && (
                  <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tabular-nums">
                    {profile.latencyMs} ms
                  </span>
                )}
                <Badge
                  variant={
                    status === 'connected' ? 'success'
                    : status === 'error' ? 'error'
                    : 'warning'
                  }
                >
                  <StatusIcon size={9} strokeWidth={2} className="mr-1" aria-hidden />
                  {status}
                </Badge>
              </div>
            </div>
          </>
        ) : (
          <div className="flex items-center gap-2">
            <WifiOff size={13} strokeWidth={1.5} className="text-[var(--text-ghost)]" aria-hidden />
            <span className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-faint)]">
              Not connected to any roko-serve instance
            </span>
          </div>
        )}

        {/* Auto-detect button */}
        <Button
          variant="ghost"
          size="sm"
          loading={detecting}
          onClick={onDetect}
          className="self-start"
        >
          <Search size={12} strokeWidth={1.5} aria-hidden />
          {detecting ? 'Detecting…' : 'Test connection'}
        </Button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// ProfileRow
// ---------------------------------------------------------------------------

interface ProfileRowProps {
  profile: ConnectionProfile;
  onConnect: (id: string) => void;
  onEdit: (id: string) => void;
  onDelete: (id: string) => void;
}

function ProfileRow({ profile, onConnect, onEdit, onDelete }: ProfileRowProps) {
  return (
    <div
      className={clsx(
        'flex items-center gap-3 px-4 py-3',
        'border-b border-b-[var(--text-ghost)] last:border-b-0',
        'hover:bg-[var(--bg-highlight)]',
        'transition-colors duration-[80ms]',
      )}
    >
      {/* Status LED */}
      <StatusLED
        status={profile.isActive ? 'active' : 'offline'}
        pulse={profile.isActive}
      />

      {/* Profile info */}
      <div className="flex flex-col gap-0.5 flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-strong)]">
            {profile.name}
          </span>
          {profile.isLocal && <Badge variant="info">local</Badge>}
          {profile.isActive && <Badge variant="success">active</Badge>}
          {profile.latencyMs !== null && profile.isActive && (
            <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tabular-nums">
              {profile.latencyMs} ms
            </span>
          )}
        </div>
        <code className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-muted)] truncate">
          {profile.url}
        </code>
      </div>

      {/* Actions */}
      <div className="flex items-center gap-1 shrink-0">
        {!profile.isActive && (
          <Button variant="secondary" size="sm" onClick={() => onConnect(profile.id)}>
            Connect
          </Button>
        )}
        <Button variant="ghost" size="sm" onClick={() => onEdit(profile.id)}>
          <Edit2 size={12} strokeWidth={1.5} aria-hidden />
        </Button>
        <Button
          variant="ghost"
          size="sm"
          disabled={profile.isActive}
          onClick={() => onDelete(profile.id)}
          className="hover:text-[var(--accent-error)] hover:border-[var(--accent-error)]"
        >
          <Trash2 size={12} strokeWidth={1.5} aria-hidden />
        </Button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Add-profile form
// ---------------------------------------------------------------------------

interface AddProfileFormProps {
  onAdd: (name: string, url: string, apiKey: string) => void;
}

function AddProfileForm({ onAdd }: AddProfileFormProps) {
  const [name, setName] = useState('');
  const [url, setUrl] = useState('');
  const [apiKey, setApiKey] = useState('');

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!name.trim() || !url.trim()) return;
    onAdd(name.trim(), url.trim(), apiKey.trim());
    setName('');
    setUrl('');
    setApiKey('');
  }

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4">
      <div className="grid grid-cols-2 gap-4">
        {/* Name */}
        <div className="flex flex-col gap-1">
          <label
            htmlFor="conn-name"
            className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]"
          >
            Profile name
          </label>
          <input
            id="conn-name"
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Production"
            className={clsx(
              'font-[var(--font-mono)] text-[var(--text-sm)]',
              'bg-[var(--bg-highlight)] text-[var(--text-strong)]',
              'border border-[var(--text-ghost)]',
              'px-2 py-1.5',
              'placeholder:text-[var(--text-ghost)]',
              'focus:border-[var(--border-active)]',
              'outline-none',
            )}
          />
        </div>

        {/* URL */}
        <div className="flex flex-col gap-1">
          <label
            htmlFor="conn-url"
            className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]"
          >
            Server URL
          </label>
          <input
            id="conn-url"
            type="url"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="https://roko.example.com"
            className={clsx(
              'font-[var(--font-mono)] text-[var(--text-sm)]',
              'bg-[var(--bg-highlight)] text-[var(--text-strong)]',
              'border border-[var(--text-ghost)]',
              'px-2 py-1.5',
              'placeholder:text-[var(--text-ghost)]',
              'focus:border-[var(--border-active)]',
              'outline-none',
            )}
          />
        </div>
      </div>

      {/* API key */}
      <div className="flex flex-col gap-1">
        <label
          htmlFor="conn-apikey"
          className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]"
        >
          API key <span className="text-[var(--text-ghost)]">(optional for local)</span>
        </label>
        <input
          id="conn-apikey"
          type="password"
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          placeholder="roko_key_…"
          autoComplete="off"
          className={clsx(
            'font-[var(--font-mono)] text-[var(--text-sm)]',
            'bg-[var(--bg-highlight)] text-[var(--text-strong)]',
            'border border-[var(--text-ghost)]',
            'px-2 py-1.5',
            'placeholder:text-[var(--text-ghost)]',
            'focus:border-[var(--border-active)]',
            'outline-none',
          )}
        />
      </div>

      <div className="flex justify-end">
        <Button
          type="submit"
          variant="primary"
          size="sm"
          disabled={!name.trim() || !url.trim()}
        >
          <Plus size={12} strokeWidth={1.5} aria-hidden />
          Add profile
        </Button>
      </div>
    </form>
  );
}

// ---------------------------------------------------------------------------
// Connection page
// ---------------------------------------------------------------------------

const LOCAL_URL = 'http://localhost:6677';

const INITIAL_PROFILES: ConnectionProfile[] = [
  {
    id: 'local',
    name: 'Local dev',
    url: LOCAL_URL,
    isLocal: true,
    isActive: true,
    latencyMs: null,
  },
];

export default function ConnectionPage() {
  const [profiles, setProfiles] = useState<ConnectionProfile[]>(INITIAL_PROFILES);
  const [activeId, setActiveId] = useState<string>('local');
  const [connStatus, setConnStatus] = useState<ConnStatus>('connecting');
  const [detecting, setDetecting] = useState(false);
  const [detectResult, setDetectResult] = useState<'found' | 'not_found' | null>(null);

  const activeProfile = profiles.find((p) => p.id === activeId) ?? null;

  /** Test the active connection and update status + latency. */
  const testActiveConnection = useCallback(async () => {
    const profile = profiles.find((p) => p.id === activeId);
    if (!profile) {
      setConnStatus('disconnected');
      return;
    }
    setConnStatus('connecting');
    const latencyMs = await pingServer(profile.url);
    if (latencyMs !== null) {
      setConnStatus('connected');
      setProfiles((prev) =>
        prev.map((p) => (p.id === activeId ? { ...p, latencyMs } : p)),
      );
    } else {
      setConnStatus('error');
      setProfiles((prev) =>
        prev.map((p) => (p.id === activeId ? { ...p, latencyMs: null } : p)),
      );
    }
  }, [profiles, activeId]);

  // Test connection on mount
  useEffect(() => {
    void testActiveConnection();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function handleDetect() {
    setDetecting(true);
    setDetectResult(null);
    const latencyMs = await pingServer(LOCAL_URL);
    setDetecting(false);
    if (latencyMs !== null) {
      setDetectResult('found');
      // If not already in profiles, add it; otherwise update latency
      setProfiles((prev) => {
        const existing = prev.find((p) => p.url === LOCAL_URL);
        if (existing) {
          return prev.map((p) => p.url === LOCAL_URL ? { ...p, latencyMs } : p);
        }
        return [
          ...prev,
          {
            id: 'local-detected',
            name: 'Local dev',
            url: LOCAL_URL,
            isLocal: true,
            isActive: false,
            latencyMs,
          },
        ];
      });
    } else {
      setDetectResult('not_found');
    }
  }

  function handleConnect(id: string) {
    const profile = profiles.find((p) => p.id === id);
    if (!profile) return;
    setProfiles((prev) =>
      prev.map((p) => ({ ...p, isActive: p.id === id })),
    );
    setActiveId(id);
    // Persist the URL so the API client picks it up
    setRokoServeUrl(profile.url);
    api.updateConnection(profile.url, profile.apiKey);
    // Re-test
    void testActiveConnection();
  }

  function handleDelete(id: string) {
    setProfiles((prev) => prev.filter((p) => p.id !== id));
  }

  function handleAdd(name: string, url: string, apiKey: string) {
    const id = `profile-${Date.now()}`;
    setProfiles((prev) => [
      ...prev,
      {
        id,
        name,
        url,
        apiKey: apiKey || undefined,
        isLocal: url.includes('localhost') || url.includes('127.0.0.1'),
        isActive: false,
        latencyMs: null,
      },
    ]);
  }

  return (
    <div className="flex flex-col gap-6 p-6">
      {/* Page header */}
      <div className="flex flex-col gap-1">
        <h1 className="font-[var(--font-mono)] text-[var(--text-lg)] text-[var(--text-strong)]">
          Connection
        </h1>
        <p className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-muted)]">
          Manage roko-serve endpoints and connection profiles.
        </p>
      </div>

      {/* Current connection status */}
      <CurrentConnectionCard
        profile={activeProfile}
        status={connStatus}
        onDetect={handleDetect}
        detecting={detecting}
      />

      {/* Detect result feedback */}
      {detectResult && (
        <div
          className={clsx(
            'flex items-center gap-2 px-3 py-2',
            'border',
            detectResult === 'found'
              ? 'border-[var(--sage)] bg-[var(--sage)]/5'
              : 'border-[var(--accent-error)] bg-[var(--accent-error)]/5',
          )}
        >
          {detecting ? (
            <Spinner size="sm" />
          ) : detectResult === 'found' ? (
            <CheckCircle size={12} strokeWidth={1.5} className="text-[var(--sage)]" aria-hidden />
          ) : (
            <XCircle size={12} strokeWidth={1.5} className="text-[var(--accent-error)]" aria-hidden />
          )}
          <span
            className={clsx(
              'font-[var(--font-mono)] text-[var(--text-xs)]',
              detectResult === 'found' ? 'text-[var(--sage)]' : 'text-[var(--accent-error)]',
            )}
          >
            {detectResult === 'found'
              ? 'roko-serve detected at localhost:6677'
              : 'No roko-serve found at localhost:6677'}
          </span>
        </div>
      )}

      {/* Saved profiles */}
      <section>
        <h2 className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tracking-[var(--tracking-widest)] uppercase mb-3">
          Saved profiles
        </h2>

        {profiles.length === 0 ? (
          <div className="border border-[var(--text-ghost)] px-4 py-6 text-center">
            <span className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-faint)]">
              No profiles saved
            </span>
          </div>
        ) : (
          <div className="border border-[var(--text-ghost)]">
            {profiles.map((profile) => (
              <ProfileRow
                key={profile.id}
                profile={profile}
                onConnect={handleConnect}
                onEdit={() => {}}
                onDelete={handleDelete}
              />
            ))}
          </div>
        )}
      </section>

      {/* Add profile form */}
      <section>
        <h2 className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tracking-[var(--tracking-widest)] uppercase mb-3">
          Add profile
        </h2>
        <div className="border border-[var(--text-ghost)] p-4">
          <AddProfileForm onAdd={handleAdd} />
        </div>
      </section>
    </div>
  );
}
