// @vitest-environment jsdom
/**
 * Acceptance: "this server cannot do that" has one treatment — the Notice and
 * the alert row's info severity share a glyph and an accent — and no alert
 * glyph looks like a close button.
 * Copied verbatim from plans/portal-programme/08d-portal-legibility/accept/.
 */
import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render } from '@testing-library/react';
import { Notice } from '@/components/primitives/Notice';
import { AlertBand } from '@/components/shell/AlertBand';
import { pickAlert } from '@/lib/alerts';
import type { AlertInput } from '@/lib/alerts';
import { initialRunState } from '@/lib/runState';
import { textOf } from '@/test/dom';

afterEach(() => cleanup());

const INPUT: AlertInput = {
  run: initialRunState(),
  connection: 'connected',
  selectedPlanId: null,
  validationErrors: 0,
  requestError: null,
  dismissedKey: null,
};

const band = () => document.querySelector('[data-region="alert"]');
const glyph = () => textOf(band()?.querySelector('.rd-alert__glyph') ?? null);

describe('Notice', () => {
  it('renders the unsupported notice with its glyph and text', () => {
    render(<Notice>This roko serve does not support editing plans yet.</Notice>);
    const note = document.querySelector('[data-notice="unsupported"]');
    expect(note?.getAttribute('role')).toBe('note');
    expect(note?.classList.contains('rd-notice')).toBe(true);
    expect(textOf(note)).toBe('ⓘThis roko serve does not support editing plans yet.');
  });
});

describe('info alerts', () => {
  it('raises a request the server cannot do as information', () => {
    const alert = pickAlert({ ...INPUT, requestNotice: 'This roko serve does not support running all plans yet.' });
    expect(alert).toEqual({
      key: 'request-notice:This roko serve does not support running all plans yet.',
      severity: 'info',
      text: 'This roko serve does not support running all plans yet.',
      actions: [],
    });
  });

  it('can be dismissed like any other alert', () => {
    const text = 'This roko serve does not support running all plans yet.';
    expect(pickAlert({ ...INPUT, requestNotice: text, dismissedKey: `request-notice:${text}` })).toBeNull();
  });

  it('keeps a real request error an error', () => {
    expect(pickAlert({ ...INPUT, requestError: 'plan hello is already running' })?.severity).toBe('error');
  });

  it('draws the info alert with the notice glyph', () => {
    render(
      <AlertBand
        alert={{ key: 'k', severity: 'info', text: 'This roko serve does not support running all plans yet.', actions: [] }}
        onAction={() => {}}
        onDismiss={() => {}}
      />,
    );
    expect(band()?.getAttribute('data-severity')).toBe('info');
    expect(glyph()).toBe('ⓘ');
  });

  it('never draws a severity glyph that looks like the close button', () => {
    for (const severity of ['error', 'warning', 'info'] as const) {
      render(<AlertBand alert={{ key: 'k', severity, text: 't', actions: [] }} onAction={() => {}} onDismiss={() => {}} />);
      expect(['✗', '×', 'x', 'X']).not.toContain(glyph());
      cleanup();
    }
  });
});
