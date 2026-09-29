/**
 * The page names its icon inline, so loading it never asks the server for
 * /favicon.ico, which the static export does not have: that request was the
 * one 404 in the console on every load.
 */
import { describe, expect, it } from 'vitest';
import { metadata } from '@/app/layout';

describe('page metadata', () => {
  it('names an inline icon, so the browser requests no favicon', () => {
    expect(metadata.icons).toMatch(/^data:/);
  });
});
