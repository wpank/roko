'use client';

import { useState } from 'react';
import { clsx } from 'clsx';
import { Plus, Trash2, Play, Pause, Rss } from 'lucide-react';
import { Button, Badge, StatusLED } from '@/components/atoms';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type FeedStatus = 'running' | 'paused' | 'error' | 'stopped';
type FeedSource = 'github' | 'webhook' | 'cron' | 'signal' | 'http_poll';

interface Feed {
  id: string;
  name: string;
  source: FeedSource;
  status: FeedStatus;
  description: string;
  lastEventAt: string | null;
  eventCount: number;
}

// ---------------------------------------------------------------------------
// Placeholder data
// ---------------------------------------------------------------------------

const SAMPLE_FEEDS: Feed[] = [];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function statusToLed(s: FeedStatus): 'active' | 'idle' | 'error' | 'offline' {
  if (s === 'running') return 'active';
  if (s === 'paused') return 'idle';
  if (s === 'error') return 'error';
  return 'offline';
}

const sourceLabel: Record<FeedSource, string> = {
  github:     'GitHub',
  webhook:    'Webhook',
  cron:       'Cron',
  signal:     'Signal',
  http_poll:  'HTTP Poll',
};

// ---------------------------------------------------------------------------
// FeedRow
// ---------------------------------------------------------------------------

interface FeedRowProps {
  feed: Feed;
  onToggle: (id: string) => void;
  onRemove: (id: string) => void;
}

function FeedRow({ feed, onToggle, onRemove }: FeedRowProps) {
  const isRunning = feed.status === 'running';

  return (
    <div
      className={clsx(
        'flex items-start gap-3 px-4 py-3',
        'border-b border-b-[var(--text-ghost)] last:border-b-0',
        'hover:bg-[var(--bg-highlight)]',
        'transition-colors duration-[80ms]',
      )}
    >
      <div className="mt-1 shrink-0">
        <StatusLED status={statusToLed(feed.status)} pulse={isRunning} />
      </div>

      <div className="flex flex-col gap-1 flex-1 min-w-0">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-strong)]">
            {feed.name}
          </span>
          <Badge variant="default">{sourceLabel[feed.source]}</Badge>
        </div>
        <p className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] leading-relaxed">
          {feed.description}
        </p>
        <div className="flex items-center gap-3 mt-0.5">
          <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)]">
            {feed.eventCount} events
          </span>
          {feed.lastEventAt && (
            <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)]">
              Last: {new Date(feed.lastEventAt).toLocaleTimeString()}
            </span>
          )}
        </div>
      </div>

      <div className="flex items-center gap-1 shrink-0">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onToggle(feed.id)}
          title={isRunning ? 'Pause feed' : 'Start feed'}
        >
          {isRunning ? (
            <Pause size={12} strokeWidth={1.5} aria-hidden />
          ) : (
            <Play size={12} strokeWidth={1.5} aria-hidden />
          )}
          {isRunning ? 'Pause' : 'Start'}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onRemove(feed.id)}
          className="hover:text-[var(--accent-error)] hover:border-[var(--accent-error)]"
        >
          <Trash2 size={12} strokeWidth={1.5} aria-hidden />
        </Button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Add feed form
// ---------------------------------------------------------------------------

interface AddFeedFormProps {
  onAdd: (name: string, source: FeedSource, description: string) => void;
}

function AddFeedForm({ onAdd }: AddFeedFormProps) {
  const [name, setName] = useState('');
  const [source, setSource] = useState<FeedSource>('webhook');
  const [description, setDescription] = useState('');

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!name.trim()) return;
    onAdd(name.trim(), source, description.trim());
    setName('');
    setDescription('');
  }

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4">
      <div className="grid grid-cols-2 gap-4">
        <div className="flex flex-col gap-1">
          <label
            htmlFor="feed-name"
            className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]"
          >
            Feed name
          </label>
          <input
            id="feed-name"
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="my-feed"
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

        <div className="flex flex-col gap-1">
          <label
            htmlFor="feed-source"
            className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]"
          >
            Source type
          </label>
          <select
            id="feed-source"
            value={source}
            onChange={(e) => setSource(e.target.value as FeedSource)}
            className={clsx(
              'font-[var(--font-mono)] text-[var(--text-sm)]',
              'bg-[var(--bg-highlight)] text-[var(--text-strong)]',
              'border border-[var(--text-ghost)]',
              'px-2 py-1.5',
              'focus:border-[var(--border-active)]',
              'outline-none',
            )}
          >
            {(Object.keys(sourceLabel) as FeedSource[]).map((s) => (
              <option key={s} value={s}>{sourceLabel[s]}</option>
            ))}
          </select>
        </div>
      </div>

      <div className="flex flex-col gap-1">
        <label
          htmlFor="feed-description"
          className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)]"
        >
          Description (optional)
        </label>
        <input
          id="feed-description"
          type="text"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder="What does this feed do?"
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
          disabled={!name.trim()}
        >
          <Plus size={12} strokeWidth={1.5} aria-hidden />
          Add feed
        </Button>
      </div>
    </form>
  );
}

// ---------------------------------------------------------------------------
// Feeds page
// ---------------------------------------------------------------------------

export default function FeedsPage() {
  const [feeds, setFeeds] = useState<Feed[]>(SAMPLE_FEEDS);

  function handleToggle(id: string) {
    setFeeds((prev) =>
      prev.map((f) =>
        f.id === id
          ? { ...f, status: f.status === 'running' ? 'paused' : 'running' }
          : f,
      ),
    );
  }

  function handleRemove(id: string) {
    setFeeds((prev) => prev.filter((f) => f.id !== id));
  }

  function handleAdd(name: string, source: FeedSource, description: string) {
    setFeeds((prev) => [
      ...prev,
      {
        id: `feed-${Date.now()}`,
        name,
        source,
        description: description || `${sourceLabel[source]} feed`,
        status: 'stopped',
        lastEventAt: null,
        eventCount: 0,
      },
    ]);
  }

  const runningCount = feeds.filter((f) => f.status === 'running').length;

  return (
    <div className="flex flex-col gap-6 p-6">
      {/* Page header */}
      <div className="flex items-start justify-between">
        <div className="flex flex-col gap-1">
          <h1 className="font-[var(--font-mono)] text-[var(--text-lg)] text-[var(--text-strong)]">
            Feeds
          </h1>
          <p className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-muted)]">
            Runtime data feeds — webhooks, crons, GitHub events, and HTTP polls.
          </p>
        </div>
        <Badge variant={runningCount > 0 ? 'success' : 'default'}>
          {runningCount} running
        </Badge>
      </div>

      {/* Feed list */}
      <section>
        <h2 className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tracking-[var(--tracking-widest)] uppercase mb-3">
          Active feeds
        </h2>

        {feeds.length === 0 ? (
          <div
            className={clsx(
              'flex flex-col items-center gap-3 py-12',
              'border border-[var(--text-ghost)]',
              'text-center',
            )}
          >
            <Rss size={24} strokeWidth={1} className="text-[var(--text-ghost)]" aria-hidden />
            <span className="font-[var(--font-mono)] text-[var(--text-sm)] text-[var(--text-faint)]">
              No feeds configured
            </span>
            <span className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-ghost)]">
              Add a feed below to start receiving events
            </span>
          </div>
        ) : (
          <div className="border border-[var(--text-ghost)]">
            {feeds.map((feed) => (
              <FeedRow
                key={feed.id}
                feed={feed}
                onToggle={handleToggle}
                onRemove={handleRemove}
              />
            ))}
          </div>
        )}
      </section>

      {/* Add feed form */}
      <section>
        <h2 className="font-[var(--font-mono)] text-[var(--text-xs)] text-[var(--text-faint)] tracking-[var(--tracking-widest)] uppercase mb-3">
          Add feed
        </h2>
        <div className="border border-[var(--text-ghost)] p-4">
          <AddFeedForm onAdd={handleAdd} />
        </div>
      </section>
    </div>
  );
}
