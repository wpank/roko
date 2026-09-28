'use client';

import { Fragment, useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { Transcript as TranscriptState } from '@/lib/runState';
import { toBlocks } from '@/lib/streamRecord';
import type { TranscriptBlock } from '@/lib/streamRecord';
import { compactDuration } from '@/lib/formatters';

// ── Transcript component ──────────────────────────────────────────────────────

/**
 * Renders a live agent transcript.
 *
 * - While `working` is set, a live "agent working · <elapsed>" line is appended
 *   and ticks every second (the server provides liveness via agent_spawned +
 *   5-second agent_heartbeat; this only makes elapsed visible).
 * - Auto-scrolls while the viewport is within 24px of the bottom; pauses on
 *   manual scroll-up and shows a "↓ follow" button to resume.
 */
export function Transcript({
  transcript,
  working,
}: {
  transcript: TranscriptState | undefined;
  working: { sinceMs: number } | null;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  // `following` starts true so the first content scrolls into view.
  const [following, setFollowing] = useState(true);
  const [elapsed, setElapsed] = useState(0);

  // ── Tick elapsed every second while the agent is working ──────────────────
  useEffect(() => {
    if (!working) return;
    setElapsed(Date.now() - working.sinceMs);
    const id = setInterval(() => {
      setElapsed(Date.now() - working.sinceMs);
    }, 1000);
    return () => clearInterval(id);
  }, [working]);

  // ── Auto-scroll after every render when following ─────────────────────────
  useLayoutEffect(() => {
    if (!following) return;
    const el = containerRef.current;
    if (!el) return;
    el.scrollTop = el.scrollHeight;
  });

  // ── Detect scroll position and update following state ─────────────────────
  const handleScroll = useCallback(() => {
    const el = containerRef.current;
    if (!el) return;
    const distFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
    setFollowing(distFromBottom <= 24);
  }, []);

  // ── Resume following ───────────────────────────────────────────────────────
  const resumeFollow = useCallback(() => {
    setFollowing(true);
    const el = containerRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, []);

  // ── Empty state ───────────────────────────────────────────────────────────
  if (transcript === undefined && !working) {
    return (
      <div data-region="transcript" className="transcript-empty">
        The agent has not produced output yet.
      </div>
    );
  }

  const blocks = transcript ? toBlocks(transcript.entries) : [];
  const dropped = transcript?.dropped ?? 0;

  // ── Render a single block (no key; key is managed by the outer wrapper) ───
  function renderBlockContent(block: TranscriptBlock): React.ReactNode {
    switch (block.kind) {
      case 'text':
        return (
          <pre className="transcript-text" style={{ whiteSpace: 'pre-wrap', margin: 0 }}>
            {block.text}
          </pre>
        );

      case 'reasoning':
        return (
          <details className="transcript-reasoning">
            <summary style={{ opacity: 0.5, cursor: 'pointer', userSelect: 'none' }}>
              <span className="transcript-gutter" aria-hidden="true">◦</span>{' '}
              reasoning
            </summary>
            <pre
              style={{
                whiteSpace: 'pre-wrap',
                margin: 0,
                opacity: 0.5,
                paddingLeft: '1.25em',
              }}
            >
              {block.text}
            </pre>
          </details>
        );

      case 'tool': {
        const firstOutputLine = block.output
          ? (block.output.split('\n')[0] ?? '')
          : '';
        return (
          <details data-tool={block.toolId} className="transcript-tool">
            <summary style={{ cursor: 'pointer', userSelect: 'none' }}>
              <span className="transcript-glyph" aria-hidden="true">
                {block.glyph}
              </span>{' '}
              <span className="transcript-tool-name">
                {block.tool || '(unknown)'}
              </span>
              {block.target && (
                <span
                  data-target=""
                  className="transcript-tool-target"
                  style={{ marginLeft: '0.5em', opacity: 0.7 }}
                >
                  {block.target}
                </span>
              )}
              {firstOutputLine && (
                <span
                  className="transcript-tool-preview"
                  style={{ opacity: 0.6 }}
                >
                  {' '}· {firstOutputLine}
                </span>
              )}
            </summary>
            {block.input && (
              <pre
                data-input=""
                style={{
                  whiteSpace: 'pre-wrap',
                  margin: 0,
                  fontFamily: 'monospace',
                  paddingLeft: '1.25em',
                }}
              >
                {block.input}
              </pre>
            )}
            {block.output !== null && (
              <div>
                {block.truncated && (
                  <div
                    className="transcript-truncated"
                    style={{ opacity: 0.6, fontSize: '0.85em', paddingLeft: '1.25em' }}
                  >
                    (server kept only the tail of this output)
                  </div>
                )}
                <pre
                  style={{
                    whiteSpace: 'pre-wrap',
                    margin: 0,
                    fontFamily: 'monospace',
                    paddingLeft: '1.25em',
                  }}
                >
                  {block.output}
                </pre>
              </div>
            )}
          </details>
        );
      }

      case 'step':
        return (
          <div data-step={block.toolId} className="transcript-step">
            <span className="transcript-glyph" aria-hidden="true">
              {block.glyph}
            </span>{' '}
            <span className="transcript-tool-name">{block.tool}</span>
            {block.target && (
              <span
                data-target=""
                className="transcript-tool-target"
                style={{ marginLeft: '0.5em', opacity: 0.7 }}
              >
                {block.target}
              </span>
            )}
            {working && (
              <span
                data-live=""
                className="transcript-live-marker"
                style={{
                  marginLeft: '0.5em',
                  fontSize: '0.75em',
                  textTransform: 'uppercase',
                  opacity: 0.5,
                }}
              >
                live
              </span>
            )}
          </div>
        );

      case 'raw':
        return (
          <pre
            className="transcript-raw"
            style={{
              whiteSpace: 'pre-wrap',
              margin: 0,
              fontFamily: 'monospace',
            }}
          >
            {block.malformed && (
              <span style={{ opacity: 0.6 }}>[malformed] </span>
            )}
            {block.text}
          </pre>
        );

      case 'divider':
        return (
          <div
            className="transcript-divider"
            style={{ opacity: 0.5, padding: '4px 0' }}
          >
            — attempt {block.attempt} —
          </div>
        );

      default:
        return null;
    }
  }

  return (
    <div style={{ position: 'relative', height: '100%' }}>
      <div
        ref={containerRef}
        data-region="transcript"
        onScroll={handleScroll}
        style={{
          overflowY: 'auto',
          height: '100%',
          fontFamily: 'inherit',
        }}
      >
        {/* Dropped record notice — always first when present */}
        {dropped > 0 && (
          <div className="transcript-dropped">
            {dropped} earlier {dropped === 1 ? 'record' : 'records'} not shown
          </div>
        )}

        {blocks.map((block, i) => {
          const isUnscreened = 'unscreened' in block && block.unscreened === true;
          const content = renderBlockContent(block);
          if (isUnscreened) {
            return (
              <div
                key={i}
                data-unscreened=""
                className="border-l-2 border-accent-warn pl-2"
              >
                <span
                  data-unscreened-label=""
                  className="text-accent-warn"
                >
                  live · unscreened
                </span>
                {content}
              </div>
            );
          }
          return <Fragment key={i}>{content}</Fragment>;
        })}

        {/* Live working indicator — ticks each second via elapsed state */}
        {working && (
          <div className="transcript-working" style={{ opacity: 0.7 }}>
            agent working · {compactDuration(elapsed)}
          </div>
        )}
      </div>

      {/* Follow button — appears when the operator has scrolled away from the bottom */}
      {!following && (
        <button
          onClick={resumeFollow}
          className="transcript-follow-btn"
          style={{
            position: 'absolute',
            bottom: '8px',
            right: '8px',
          }}
          aria-label="Scroll to latest output"
        >
          ↓ follow
        </button>
      )}
    </div>
  );
}
