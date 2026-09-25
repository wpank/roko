'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Terminal,
  Plus,
  X,
  Maximize2,
  Minimize2,
  ChevronDown,
  ChevronUp,
} from 'lucide-react';
import { clsx } from 'clsx';
import {
  useTerminalPanel,
  type TerminalSession,
  type SessionStatus,
} from './useTerminalPanel';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Quick-command entries shown in the dropdown. */
const QUICK_COMMANDS: { label: string; cmd: string }[] = [
  { label: 'roko status',      cmd: 'roko status' },
  { label: 'roko doctor',      cmd: 'roko doctor' },
  { label: 'roko plan run',    cmd: 'roko plan run plans/' },
  { label: 'roko agent list',  cmd: 'roko agent list' },
  { label: 'roko show',        cmd: 'roko show' },
  { label: 'roko learn all',   cmd: 'roko learn all' },
  { label: 'roko dashboard',   cmd: 'roko dashboard' },
];

/** Drag handle visible height in px. */
const DRAG_HANDLE_PX = 4;

// ---------------------------------------------------------------------------
// Status dot
// ---------------------------------------------------------------------------

const STATUS_COLORS: Record<SessionStatus, string> = {
  active: 'var(--sage)',
  idle:   'var(--text-muted)',
  dead:   'var(--accent-error)',
};

function StatusDot({ status }: { status: SessionStatus }) {
  return (
    <span
      aria-label={status}
      style={{
        display:         'inline-block',
        width:           6,
        height:          6,
        borderRadius:    '50%',
        backgroundColor: STATUS_COLORS[status],
        flexShrink:      0,
        boxShadow:
          status === 'active'
            ? `0 0 4px 1px ${STATUS_COLORS.active}`
            : undefined,
      }}
    />
  );
}

// ---------------------------------------------------------------------------
// Session tab
// ---------------------------------------------------------------------------

interface SessionTabProps {
  session:  TerminalSession;
  isActive: boolean;
  onSelect: () => void;
  onClose:  (e: React.MouseEvent) => void;
}

function SessionTab({ session, isActive, onSelect, onClose }: SessionTabProps) {
  return (
    <button
      type="button"
      role="tab"
      aria-selected={isActive}
      onClick={onSelect}
      className={clsx(
        'relative group flex items-center gap-1.5 shrink-0',
        'h-full px-3',
        'font-[var(--font-mono)] text-[11px] leading-none',
        'transition-colors duration-[80ms]',
        'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
        'select-none',
        isActive
          ? [
              'bg-[var(--bg-raised)]',
              'text-[var(--text-strong)]',
              // Active tab underline accent
              'after:absolute after:bottom-0 after:left-0 after:right-0',
              'after:h-[1px] after:bg-[var(--rose-dim)]',
            ]
          : [
              'bg-transparent',
              'text-[var(--text-faint)]',
              'hover:bg-[var(--bg-highlight)]',
              'hover:text-[var(--text-muted)]',
            ],
      )}
    >
      <StatusDot status={session.status} />
      <span className="truncate max-w-[120px]">{session.name}</span>

      {/* Close button — visible on hover/active */}
      <span
        role="button"
        aria-label={`Close ${session.name}`}
        tabIndex={0}
        onClick={onClose}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') onClose(e as unknown as React.MouseEvent);
        }}
        className={clsx(
          'flex items-center justify-center',
          'w-3.5 h-3.5 ml-0.5 -mr-1',
          'text-[var(--text-ghost)] hover:text-[var(--text-muted)]',
          'opacity-0 group-hover:opacity-100',
          isActive && 'opacity-100',
          'transition-opacity duration-[80ms]',
          'rounded-[0]',
        )}
      >
        <X size={10} strokeWidth={1.5} aria-hidden />
      </span>
    </button>
  );
}

// ---------------------------------------------------------------------------
// Quick commands dropdown
// ---------------------------------------------------------------------------

function QuickCommandsMenu({ onClose }: { onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);

  // Close on outside click
  useEffect(() => {
    function handler(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose();
      }
    }
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [onClose]);

  return (
    <div
      ref={ref}
      role="menu"
      className={clsx(
        'absolute right-0 bottom-full mb-1',
        'w-52',
        'bg-[var(--bg-secondary)]',
        'border border-[var(--text-ghost)]',
        'shadow-[var(--shadow-md)]',
        'z-[var(--z-dropdown)]',
        'py-1',
      )}
    >
      {QUICK_COMMANDS.map(({ label, cmd }) => (
        <button
          key={cmd}
          type="button"
          role="menuitem"
          onClick={() => {
            // Phase 3: pipe cmd into the active xterm session.
            // For now, just copy to clipboard as a convenience.
            void navigator.clipboard?.writeText(cmd).catch(() => {});
            onClose();
          }}
          className={clsx(
            'w-full flex items-center px-3 py-1.5',
            'font-[var(--font-mono)] text-[11px] text-left',
            'text-[var(--text-muted)] hover:text-[var(--text-strong)]',
            'hover:bg-[var(--bg-highlight)]',
            'transition-colors duration-[80ms]',
            'border-0',
            'outline-none focus-visible:ring-inset focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
          )}
        >
          <span className="text-[var(--text-ghost)] mr-2 select-none">$</span>
          {label}
        </button>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Drag-resize logic
// ---------------------------------------------------------------------------

function useDragResize(enabled: boolean) {
  const store = useTerminalPanel();
  const dragRef  = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);

  useEffect(() => {
    if (!enabled) return;

    function onMouseDown(e: MouseEvent) {
      if (!dragRef.current?.contains(e.target as Node)) return;
      e.preventDefault();
      dragging.current = true;
      document.body.style.cursor = 'ns-resize';
      document.body.style.userSelect = 'none';
    }

    function onMouseMove(e: MouseEvent) {
      if (!dragging.current) return;
      // Panel grows upward: distance from bottom of viewport.
      const pct = ((window.innerHeight - e.clientY) / window.innerHeight) * 100;
      store.setHeight(pct);
    }

    function onMouseUp() {
      if (!dragging.current) return;
      dragging.current = false;
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    }

    document.addEventListener('mousedown', onMouseDown);
    document.addEventListener('mousemove', onMouseMove);
    document.addEventListener('mouseup', onMouseUp);

    return () => {
      document.removeEventListener('mousedown', onMouseDown);
      document.removeEventListener('mousemove', onMouseMove);
      document.removeEventListener('mouseup', onMouseUp);
    };
  }, [enabled, store]);

  return dragRef;
}

// ---------------------------------------------------------------------------
// Keyboard shortcut hook
// ---------------------------------------------------------------------------

function useKeyboardShortcuts() {
  const store = useTerminalPanel();

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      const ctrl = e.ctrlKey;
      const shift = e.shiftKey;
      const key = e.key;

      // Ctrl+` — toggle panel
      if (ctrl && !shift && key === '`') {
        e.preventDefault();
        store.toggle();
        return;
      }

      // Ctrl+Shift+T — new session tab
      if (ctrl && shift && key === 'T') {
        e.preventDefault();
        store.addSession();
        return;
      }

      // Ctrl+Shift+W — close current tab
      if (ctrl && shift && key === 'W') {
        e.preventDefault();
        if (store.activeSessionId) store.removeSession(store.activeSessionId);
        return;
      }

      // Ctrl+1-9 — switch to tab N
      if (ctrl && !shift && key >= '1' && key <= '9') {
        const idx = parseInt(key, 10) - 1;
        const session = store.sessions[idx];
        if (session) {
          e.preventDefault();
          store.setActiveSession(session.id);
        }
      }
    }

    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
  }, [store]);
}

// ---------------------------------------------------------------------------
// Animation variants
// ---------------------------------------------------------------------------

// Panel slides up from the bottom edge.
const PANEL_VARIANTS = {
  hidden:  { y: '100%', opacity: 0 },
  visible: { y: 0,      opacity: 1 },
  exit:    { y: '100%', opacity: 0 },
} as const;

const EXPO_OUT = [0.16, 1, 0.3, 1] as const;

// ---------------------------------------------------------------------------
// TerminalPanel
// ---------------------------------------------------------------------------

export function TerminalPanel() {
  const store = useTerminalPanel();
  const { panelState, sessions, activeSessionId, panelHeight } = store;

  const [quickMenuOpen, setQuickMenuOpen] = useState(false);

  const dragRef = useDragResize(panelState === 'open');
  useKeyboardShortcuts();

  const isVisible   = panelState !== 'hidden';
  const isMinimized = panelState === 'minimized';
  const isOpen      = panelState === 'open';
  const isMaximized = panelState === 'maximized';

  const handleTabClose = useCallback(
    (e: React.MouseEvent, id: string) => {
      e.stopPropagation();
      store.removeSession(id);
    },
    [store],
  );

  // Derive the panel's pixel height for the open state.
  const openHeightStyle =
    isOpen      ? `${panelHeight}vh`
    : isMaximized ? '100%'
    : undefined;

  // ---- Minimized bar -------------------------------------------------------

  const minimizedBar = (
    <div
      role="region"
      aria-label="Terminal panel (minimized)"
      className={clsx(
        'flex items-center justify-between shrink-0',
        'h-7 px-3',
        'bg-[var(--bg-secondary)]',
        'border-t border-t-[var(--text-ghost)]',
        'cursor-pointer select-none',
      )}
      onClick={() => store.restore()}
    >
      {/* Left: icon + label + session count */}
      <div className="flex items-center gap-2">
        <Terminal
          size={12}
          strokeWidth={1.5}
          className="text-[var(--text-faint)]"
          aria-hidden
        />
        <span className="font-[var(--font-mono)] text-[11px] text-[var(--text-faint)] leading-none tracking-[var(--tracking-wide)]">
          Terminal
        </span>
        {sessions.length > 0 && (
          <span className="font-[var(--font-mono)] text-[10px] text-[var(--text-ghost)] leading-none">
            ({sessions.length})
          </span>
        )}
      </div>

      {/* Right: chevron */}
      <ChevronUp
        size={12}
        strokeWidth={1.5}
        className="text-[var(--text-faint)]"
        aria-hidden
      />
    </div>
  );

  // ---- Full panel ----------------------------------------------------------

  const fullPanel = (
    <div
      role="region"
      aria-label="Terminal panel"
      style={{ height: openHeightStyle }}
      className={clsx(
        'flex flex-col',
        'bg-[var(--bg-raised)]',
        'border-t border-t-[var(--text-ghost)]',
        isMaximized && 'absolute inset-0 z-[var(--z-overlay)]',
      )}
    >
      {/* ---- Drag handle ---- */}
      {isOpen && (
        <div
          ref={dragRef}
          aria-hidden
          style={{ height: DRAG_HANDLE_PX }}
          className={clsx(
            'w-full shrink-0',
            'cursor-ns-resize',
            'bg-transparent',
            'hover:bg-[var(--rose-dim)]',
            'transition-colors duration-[80ms]',
          )}
          title="Drag to resize terminal"
        />
      )}

      {/* ---- Tab bar ---- */}
      <div
        role="tablist"
        aria-label="Terminal sessions"
        className={clsx(
          'flex items-stretch shrink-0',
          isOpen ? `h-[${28 - DRAG_HANDLE_PX}px]` : 'h-7',
          'bg-[var(--bg-secondary)]',
          'border-b border-b-[var(--text-ghost)]',
          'overflow-x-auto overflow-y-hidden',
        )}
        style={{ height: isOpen ? 28 - DRAG_HANDLE_PX : 28 }}
      >
        {/* Session tabs */}
        {sessions.map((session) => (
          <SessionTab
            key={session.id}
            session={session}
            isActive={session.id === activeSessionId}
            onSelect={() => store.setActiveSession(session.id)}
            onClose={(e) => handleTabClose(e, session.id)}
          />
        ))}

        {/* New session button */}
        <button
          type="button"
          aria-label="New terminal session (Ctrl+Shift+T)"
          onClick={store.addSession}
          className={clsx(
            'flex items-center justify-center shrink-0',
            'h-full w-7',
            'text-[var(--text-ghost)] hover:text-[var(--text-muted)]',
            'hover:bg-[var(--bg-highlight)]',
            'transition-colors duration-[80ms]',
            'border-0 outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
          )}
        >
          <Plus size={12} strokeWidth={1.5} aria-hidden />
        </button>

        {/* Spacer pushes panel controls to the right */}
        <div className="flex-1" aria-hidden />

        {/* ---- Panel controls (right side) ---- */}
        <div className="flex items-center shrink-0 gap-0.5 px-1.5">
          {/* Quick commands */}
          <div className="relative">
            <button
              type="button"
              aria-label="Quick commands"
              aria-haspopup="menu"
              aria-expanded={quickMenuOpen}
              onClick={() => setQuickMenuOpen((o) => !o)}
              className={clsx(
                'flex items-center gap-1 shrink-0',
                'h-5 px-2',
                'font-[var(--font-mono)] text-[10px]',
                'text-[var(--text-ghost)] hover:text-[var(--text-muted)]',
                'border border-[var(--text-ghost)] hover:border-[var(--rose-dim)]',
                'hover:bg-[var(--bg-highlight)]',
                'transition-colors duration-[80ms]',
                'outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
              )}
            >
              $ <ChevronDown size={9} strokeWidth={1.5} aria-hidden />
            </button>

            <AnimatePresence>
              {quickMenuOpen && (
                <motion.div
                  initial={{ opacity: 0, y: 4 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: 4 }}
                  transition={{ duration: 0.1, ease: 'easeOut' }}
                >
                  <QuickCommandsMenu onClose={() => setQuickMenuOpen(false)} />
                </motion.div>
              )}
            </AnimatePresence>
          </div>

          {/* Maximize / restore */}
          <button
            type="button"
            aria-label={isMaximized ? 'Restore terminal panel' : 'Maximize terminal panel'}
            onClick={() => (isMaximized ? store.restore() : store.maximize())}
            className={clsx(
              'flex items-center justify-center',
              'h-5 w-5',
              'text-[var(--text-ghost)] hover:text-[var(--text-muted)]',
              'hover:bg-[var(--bg-highlight)]',
              'transition-colors duration-[80ms]',
              'border-0 outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
            )}
          >
            {isMaximized
              ? <Minimize2 size={11} strokeWidth={1.5} aria-hidden />
              : <Maximize2 size={11} strokeWidth={1.5} aria-hidden />}
          </button>

          {/* Minimize */}
          <button
            type="button"
            aria-label="Minimize terminal panel"
            onClick={store.minimize}
            className={clsx(
              'flex items-center justify-center',
              'h-5 w-5',
              'text-[var(--text-ghost)] hover:text-[var(--text-muted)]',
              'hover:bg-[var(--bg-highlight)]',
              'transition-colors duration-[80ms]',
              'border-0 outline-none focus-visible:ring-1 focus-visible:ring-[var(--rose-glow)]',
            )}
          >
            <ChevronDown size={11} strokeWidth={1.5} aria-hidden />
          </button>
        </div>
      </div>

      {/* ---- Content area ---- */}
      <div className="flex-1 min-h-0 relative overflow-hidden">
        {/* Placeholder content — replaced by xterm.js in Phase 3 (PB-041) */}
        <div
          className={clsx(
            'absolute inset-0 flex flex-col items-center justify-center gap-3',
            'select-none pointer-events-none',
          )}
        >
          {/* Decorative terminal glyph */}
          <Terminal
            size={28}
            strokeWidth={1}
            className="text-[var(--text-ghost)]"
            aria-hidden
          />

          <div className="flex flex-col items-center gap-1.5 text-center">
            <p className="font-[var(--font-mono)] text-[11px] text-[var(--text-faint)] leading-snug">
              Terminal requires WebSocket PTY bridge
              <span className="text-[var(--text-ghost)]"> (coming soon)</span>
            </p>
            <p className="font-[var(--font-mono)] text-[11px] text-[var(--text-ghost)] leading-snug">
              Use{' '}
              <kbd
                className={clsx(
                  'inline-block px-1 py-0.5',
                  'font-[var(--font-mono)] text-[10px]',
                  'bg-[var(--bg-secondary)]',
                  'border border-[var(--text-ghost)]',
                  'text-[var(--text-faint)]',
                  'leading-none',
                )}
              >
                roko dashboard
              </kbd>{' '}
              for TUI access
            </p>
          </div>

          {/* Keyboard shortcut hints */}
          <div className="flex items-center gap-3 mt-1">
            {[
              { keys: 'Ctrl+`',         label: 'Toggle' },
              { keys: 'Ctrl+Shift+T',   label: 'New tab' },
              { keys: 'Ctrl+Shift+W',   label: 'Close tab' },
            ].map(({ keys, label }) => (
              <div key={keys} className="flex items-center gap-1">
                <kbd
                  className={clsx(
                    'inline-block px-1 py-0.5',
                    'font-[var(--font-mono)] text-[9px]',
                    'bg-[var(--bg-secondary)]',
                    'border border-[var(--text-ghost)]',
                    'text-[var(--text-ghost)]',
                    'leading-none',
                  )}
                >
                  {keys}
                </kbd>
                <span className="font-[var(--font-mono)] text-[9px] text-[var(--text-ghost)]">
                  {label}
                </span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );

  // ---- Render --------------------------------------------------------------

  return (
    <AnimatePresence>
      {isVisible && (
        <motion.div
          key="terminal-panel"
          className={clsx(
            'shrink-0 w-full',
            // Clip drag-resize overflow
            'overflow-hidden',
          )}
          variants={PANEL_VARIANTS}
          initial="hidden"
          animate="visible"
          exit="exit"
          transition={{ duration: 0.25, ease: EXPO_OUT }}
          // Suppress the animated height so inline style controls it
          style={{}}
        >
          {isMinimized ? minimizedBar : fullPanel}
        </motion.div>
      )}
    </AnimatePresence>
  );
}
