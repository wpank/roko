'use client';

import React, { useState, useRef, useEffect, useCallback } from 'react';
import { clsx } from 'clsx';
import { useAgents } from '@/api/hooks';
import { api } from '@/api/client';
import { Button } from '@/components/atoms';
import { Spinner } from '@/components/atoms/Spinner';
import { showToast } from '@/components/atoms/Toast';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: string;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

let _msgId = 0;
function nextId(): string {
  _msgId += 1;
  return `msg-${_msgId}`;
}

function formatTs(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleTimeString(undefined, { hour12: false, hour: '2-digit', minute: '2-digit' });
  } catch {
    return '';
  }
}

// ---------------------------------------------------------------------------
// MessageBubble
// ---------------------------------------------------------------------------

function MessageBubble({ msg }: { msg: ChatMessage }) {
  const isUser = msg.role === 'user';
  return (
    <div
      className={clsx(
        'flex gap-3 px-4 py-3',
        'border-b border-b-[var(--text-ghost)] last:border-b-0',
      )}
    >
      {/* Role glyph */}
      <span
        className={clsx(
          'shrink-0 w-5 h-5 flex items-center justify-center border mt-[1px]',
          'font-mono text-[10px] font-medium leading-none select-none',
          isUser
            ? 'border-[var(--rose-dim)] text-[var(--rose-dim)]'
            : 'border-[var(--dream)] text-[var(--dream)]',
        )}
        aria-hidden
      >
        {isUser ? 'U' : 'A'}
      </span>

      {/* Content */}
      <div className="flex-1 min-w-0 flex flex-col gap-1">
        <p
          className={clsx(
            'font-mono text-xs leading-relaxed whitespace-pre-wrap break-words',
            isUser ? 'text-[var(--text-strong)]' : 'text-[var(--text-muted)]',
          )}
        >
          {msg.content}
        </p>
        <span className="font-mono text-[10px] text-[var(--text-ghost)] tabular-nums">
          {formatTs(msg.timestamp)}
        </span>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function AgentsChatPage() {
  const { data: agents, isLoading: agentsLoading, isError: agentsError } = useAgents();

  const [selectedAgentId, setSelectedAgentId] = useState<string>('');
  const [messages, setMessages]   = useState<ChatMessage[]>([]);
  const [input, setInput]         = useState('');
  const [sending, setSending]     = useState(false);

  const bottomRef = useRef<HTMLDivElement>(null);

  // Auto-select the first agent once loaded
  useEffect(() => {
    if (!selectedAgentId && agents && agents.length > 0) {
      setSelectedAgentId(agents[0].id);
    }
  }, [agents, selectedAgentId]);

  // Scroll to bottom when messages change
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ block: 'end', behavior: 'smooth' });
  }, [messages]);

  const handleSend = useCallback(async () => {
    const trimmed = input.trim();
    if (!trimmed || !selectedAgentId || sending) return;

    setInput('');
    setSending(true);

    const userMsg: ChatMessage = {
      id: nextId(),
      role: 'user',
      content: trimmed,
      timestamp: new Date().toISOString(),
    };
    setMessages((prev) => [...prev, userMsg]);

    try {
      const resp = await api.post<{ reply?: string; response?: string; content?: string }>(
        `/api/agents/${selectedAgentId}/message`,
        { content: trimmed },
      );
      const reply =
        resp.reply ?? resp.response ?? resp.content ?? '(no response)';
      setMessages((prev) => [
        ...prev,
        { id: nextId(), role: 'assistant', content: reply, timestamp: new Date().toISOString() },
      ]);
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to send message';
      showToast(msg, 'error');
      setMessages((prev) => [
        ...prev,
        {
          id: nextId(),
          role: 'assistant',
          content: `Error: ${msg}`,
          timestamp: new Date().toISOString(),
        },
      ]);
    } finally {
      setSending(false);
    }
  }, [input, selectedAgentId, sending]);

  // ---------------------------------------------------------------------------
  // Loading / error guards
  // ---------------------------------------------------------------------------

  if (agentsLoading) {
    return (
      <div className="flex items-center justify-center h-64 gap-2">
        <Spinner size="sm" />
        <span className="font-mono text-xs text-[var(--text-ghost)]">Loading agents…</span>
      </div>
    );
  }

  if (agentsError) {
    return (
      <div className="flex flex-col items-center justify-center h-64 gap-2">
        <span className="font-mono text-xs text-[var(--accent-error)]">
          Failed to load agents. Is roko serve running?
        </span>
      </div>
    );
  }

  if (!agents || agents.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-64 gap-3 px-4 text-center">
        <span className="font-mono text-xs text-[var(--text-ghost)]">
          No agents registered.
        </span>
        <span className="font-mono text-[10px] text-[var(--text-faint)]">
          Create an agent from the Roster tab to start chatting.
        </span>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full min-h-0">

      {/* Agent selector bar */}
      <div className="shrink-0 flex items-center gap-3 px-4 py-2 border-b border-b-[var(--text-ghost)] bg-[var(--bg-raised)]">
        <label
          htmlFor="chat-agent-select"
          className="font-mono text-[10px] text-[var(--text-ghost)] uppercase tracking-widest shrink-0"
        >
          Agent
        </label>
        <select
          id="chat-agent-select"
          value={selectedAgentId}
          onChange={(e) => {
            setSelectedAgentId(e.target.value);
            setMessages([]);
          }}
          className={clsx(
            'flex-1 max-w-xs',
            'bg-[var(--bg-secondary)] border border-[var(--text-ghost)]',
            'font-mono text-xs text-[var(--text-muted)]',
            'px-2 py-1 leading-none',
            'outline-none focus:border-[var(--rose-dim)]',
            'transition-[border-color] duration-[80ms]',
            'cursor-pointer',
          )}
        >
          {agents.map((a) => (
            <option key={a.id} value={a.id}>
              {a.name} ({a.role})
            </option>
          ))}
        </select>

        {messages.length > 0 && (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setMessages([])}
          >
            Clear
          </Button>
        )}
      </div>

      {/* Message history */}
      <div className="flex-1 overflow-y-auto overflow-x-hidden min-h-0">
        {messages.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-48 gap-2 px-4 text-center">
            <span className="font-mono text-xs text-[var(--text-ghost)]">
              No messages yet — type below to start the conversation.
            </span>
          </div>
        ) : (
          messages.map((msg) => <MessageBubble key={msg.id} msg={msg} />)
        )}
        <div ref={bottomRef} aria-hidden />
      </div>

      {/* Input area */}
      <div className="shrink-0 flex items-end gap-2 px-4 py-3 border-t border-t-[var(--text-ghost)] bg-[var(--bg-raised)]">
        <textarea
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
              e.preventDefault();
              void handleSend();
            }
          }}
          placeholder="Message agent… (⌘↵ to send)"
          rows={2}
          disabled={sending}
          className={clsx(
            'flex-1 bg-[var(--bg-secondary)] border border-[var(--text-ghost)]',
            'font-mono text-xs text-[var(--text-strong)]',
            'px-2 py-2 resize-none leading-relaxed',
            'placeholder:text-[var(--text-ghost)]',
            'focus:border-[var(--rose-dim)] focus:outline-none',
            'transition-[border-color] duration-[80ms]',
            'disabled:opacity-50',
          )}
        />
        <Button
          variant="primary"
          size="sm"
          onClick={() => void handleSend()}
          disabled={!input.trim() || !selectedAgentId || sending}
          loading={sending}
        >
          Send
        </Button>
      </div>
    </div>
  );
}
