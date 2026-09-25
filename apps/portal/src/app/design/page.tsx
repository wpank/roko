'use client';

import React, { useState } from 'react';
import {
  Badge,
  Button,
  Pill,
  ProgressBar,
  Sparkline,
  Spinner,
  StatusLED,
  Tooltip,
} from '@/components/atoms';

// ---------------------------------------------------------------------------
// Layout helpers
// ---------------------------------------------------------------------------

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section style={{ marginBottom: 'var(--space-16)' }}>
      <h2
        style={{
          fontFamily: 'var(--font-mono)',
          fontSize: 'var(--text-xs)',
          fontWeight: 600,
          letterSpacing: 'var(--tracking-widest)',
          textTransform: 'uppercase',
          color: 'var(--text-faint)',
          marginBottom: 'var(--space-6)',
          paddingBottom: 'var(--space-2)',
          borderBottom: '1px solid var(--border-default)',
        }}
      >
        {title}
      </h2>
      {children}
    </section>
  );
}

function SubSection({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div style={{ marginBottom: 'var(--space-8)' }}>
      <h3
        style={{
          fontFamily: 'var(--font-mono)',
          fontSize: 'var(--text-xs)',
          fontWeight: 500,
          letterSpacing: 'var(--tracking-wide)',
          color: 'var(--text-muted)',
          marginBottom: 'var(--space-4)',
        }}
      >
        {title}
      </h3>
      {children}
    </div>
  );
}

function Row({
  children,
  gap = 'var(--space-3)',
  wrap = true,
}: {
  children: React.ReactNode;
  gap?: string;
  wrap?: boolean;
}) {
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        flexWrap: wrap ? 'wrap' : 'nowrap',
        gap,
      }}
    >
      {children}
    </div>
  );
}

// ---------------------------------------------------------------------------
// 1. Color Swatches
// ---------------------------------------------------------------------------

interface SwatchDef {
  name: string;
  cssValue: string;
  hex: string;
}

function Swatch({ name, cssValue, hex }: SwatchDef) {
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 'var(--space-2)',
        minWidth: 96,
      }}
    >
      <div
        style={{
          width: 40,
          height: 40,
          background: cssValue,
          border: '1px solid var(--border-default)',
          flexShrink: 0,
        }}
      />
      <span
        style={{
          fontFamily: 'var(--font-mono)',
          fontSize: 'var(--text-xs)',
          color: 'var(--text-muted)',
          lineHeight: 'var(--leading-snug)',
        }}
      >
        --{name}
      </span>
      <span
        style={{
          fontFamily: 'var(--font-mono)',
          fontSize: 10,
          color: 'var(--text-faint)',
          lineHeight: 'var(--leading-tight)',
        }}
      >
        {hex}
      </span>
    </div>
  );
}

function SwatchGrid({ swatches }: { swatches: SwatchDef[] }) {
  return (
    <div style={{ display: 'flex', flexWrap: 'wrap', gap: 'var(--space-6)' }}>
      {swatches.map((s) => (
        <Swatch key={s.name} {...s} />
      ))}
    </div>
  );
}

const BACKGROUNDS: SwatchDef[] = [
  { name: 'void',            cssValue: 'var(--void)',            hex: '#000000' },
  { name: 'bg-raised',       cssValue: 'var(--bg-raised)',       hex: '#0e0c12' },
  { name: 'bg-secondary',    cssValue: 'var(--bg-secondary)',    hex: '#0e0c10' },
  { name: 'bg-highlight',    cssValue: 'var(--bg-highlight)',    hex: '#221c24' },
  { name: 'bg-glass',        cssValue: 'rgba(14,12,18,0.85)',    hex: 'rgba(14,12,18,.85)' },
  { name: 'bg-glass-active', cssValue: 'rgba(34,28,36,0.70)',    hex: 'rgba(34,28,36,.70)' },
];

const TEXT_COLORS: SwatchDef[] = [
  { name: 'text-strong', cssValue: 'var(--text-strong)', hex: '#d8c8d0' },
  { name: 'text-muted',  cssValue: 'var(--text-muted)',  hex: '#7a6f7f' },
  { name: 'text-faint',  cssValue: 'var(--text-faint)',  hex: '#504a54' },
  { name: 'text-ghost',  cssValue: 'var(--text-ghost)',  hex: '#3a3440' },
];

const ACCENTS: SwatchDef[] = [
  { name: 'rose',         cssValue: 'var(--rose)',         hex: '#c06070' },
  { name: 'rose-bright',  cssValue: 'var(--rose-bright)',  hex: '#e87070' },
  { name: 'rose-dim',     cssValue: 'var(--rose-dim)',     hex: '#9b6a7c' },
  { name: 'rose-glow',    cssValue: 'var(--rose-glow)',    hex: '#ff8090' },
  { name: 'bone',         cssValue: 'var(--bone)',         hex: '#d0c0b0' },
  { name: 'bone-bright',  cssValue: 'var(--bone-bright)',  hex: '#e8d8c8' },
  { name: 'dream',        cssValue: 'var(--dream)',        hex: '#8060c0' },
  { name: 'dream-bright', cssValue: 'var(--dream-bright)', hex: '#a080e0' },
  { name: 'sage',         cssValue: 'var(--sage)',         hex: '#60a060' },
  { name: 'ember',        cssValue: 'var(--ember)',        hex: '#c08040' },
  { name: 'warning',      cssValue: 'var(--warning)',      hex: '#c0a040' },
];

const SEMANTIC: SwatchDef[] = [
  { name: 'accent-rose',    cssValue: 'var(--accent-rose)',    hex: '→ --rose' },
  { name: 'accent-cyan',    cssValue: 'var(--accent-cyan)',    hex: '#60a0c0' },
  { name: 'accent-amber',   cssValue: 'var(--accent-amber)',   hex: '→ --warning' },
  { name: 'accent-error',   cssValue: 'var(--accent-error)',   hex: '#c41f1f' },
  { name: 'accent-success', cssValue: 'var(--accent-success)', hex: '→ --sage' },
];

function ColorsSection() {
  return (
    <Section title="Color Swatches">
      <SubSection title="Backgrounds">
        <SwatchGrid swatches={BACKGROUNDS} />
      </SubSection>
      <SubSection title="Text">
        <SwatchGrid swatches={TEXT_COLORS} />
      </SubSection>
      <SubSection title="Accents">
        <SwatchGrid swatches={ACCENTS} />
      </SubSection>
      <SubSection title="Semantic">
        <SwatchGrid swatches={SEMANTIC} />
      </SubSection>
    </Section>
  );
}

// ---------------------------------------------------------------------------
// 2. Typography
// ---------------------------------------------------------------------------

const TYPE_SCALE: Array<{ varName: string; px: string; sample: string }> = [
  { varName: '10px (raw)',      px: '10px',               sample: 'The quick brown fox jumps over the lazy dog' },
  { varName: '--text-xs (11)',  px: 'var(--text-xs)',     sample: 'The quick brown fox jumps over the lazy dog' },
  { varName: '--text-sm (12)',  px: 'var(--text-sm)',     sample: 'The quick brown fox jumps over the lazy dog' },
  { varName: '--text-base (13)',px: 'var(--text-base)',   sample: 'The quick brown fox jumps over the lazy dog' },
  { varName: '--text-md (14)',  px: 'var(--text-md)',     sample: 'The quick brown fox jumps over the lazy dog' },
  { varName: '--text-lg (16)',  px: 'var(--text-lg)',     sample: 'The quick brown fox' },
  { varName: '--text-xl (18)',  px: 'var(--text-xl)',     sample: 'The quick brown fox' },
  { varName: '--text-2xl (22)', px: 'var(--text-2xl)',    sample: 'Quick brown fox' },
  { varName: '--text-3xl (28)', px: 'var(--text-3xl)',    sample: 'Quick brown fox' },
];

function TypographySection() {
  return (
    <Section title="Typography">
      <SubSection title="Font Stacks">
        <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-4)' }}>
          {[
            {
              token: '--font-mono',
              family: 'var(--font-mono)',
              preview: 'JetBrains Mono · Fira Code · Cascadia Code · SF Mono · ui-monospace',
            },
            {
              token: '--font-sans',
              family: 'var(--font-sans)',
              preview: '-apple-system · BlinkMacSystemFont · Segoe UI · Helvetica Neue · Arial',
            },
          ].map(({ token, family, preview }) => (
            <div key={token}>
              <span
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 10,
                  color: 'var(--text-ghost)',
                  letterSpacing: 'var(--tracking-wide)',
                  display: 'block',
                  marginBottom: 'var(--space-1)',
                }}
              >
                {token}
              </span>
              <span
                style={{
                  fontFamily: family,
                  fontSize: 'var(--text-base)',
                  color: 'var(--text-strong)',
                }}
              >
                {preview}
              </span>
            </div>
          ))}
        </div>
      </SubSection>

      <SubSection title="Size Scale">
        <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-3)' }}>
          {TYPE_SCALE.map(({ varName, px, sample }) => (
            <div
              key={varName}
              style={{
                display: 'flex',
                alignItems: 'baseline',
                gap: 'var(--space-4)',
                paddingBottom: 'var(--space-2)',
                borderBottom: '1px solid var(--border-default)',
              }}
            >
              <span
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 10,
                  color: 'var(--text-ghost)',
                  width: 148,
                  flexShrink: 0,
                  letterSpacing: 'var(--tracking-wide)',
                }}
              >
                {varName}
              </span>
              <span
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: px,
                  color: 'var(--text-strong)',
                  lineHeight: 'var(--leading-tight)',
                }}
              >
                {sample}
              </span>
            </div>
          ))}
        </div>
      </SubSection>

      <SubSection title="Letter Spacing">
        <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-3)' }}>
          {[
            { token: '--tracking-tight',   value: '-0.02em', label: 'TIGHT  −0.02EM' },
            { token: '--tracking-normal',  value: '0',       label: 'NORMAL  0' },
            { token: '--tracking-wide',    value: '0.04em',  label: 'WIDE  +0.04EM' },
            { token: '--tracking-wider',   value: '0.08em',  label: 'WIDER  +0.08EM' },
            { token: '--tracking-widest',  value: '0.16em',  label: 'WIDEST  +0.16EM' },
          ].map(({ token, value, label }) => (
            <div
              key={token}
              style={{ display: 'flex', alignItems: 'baseline', gap: 'var(--space-4)' }}
            >
              <span
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 10,
                  color: 'var(--text-ghost)',
                  width: 148,
                  flexShrink: 0,
                }}
              >
                {token}
              </span>
              <span
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 'var(--text-sm)',
                  color: 'var(--text-muted)',
                  letterSpacing: value,
                }}
              >
                {label}
              </span>
            </div>
          ))}
        </div>
      </SubSection>
    </Section>
  );
}

// ---------------------------------------------------------------------------
// 3. Components
// ---------------------------------------------------------------------------

function ComponentsSection() {
  const [activePill, setActivePill] = useState<string | null>('all');

  return (
    <Section title="Components">

      {/* --- Button --- */}
      <SubSection title="Button">
        {(['primary', 'secondary', 'ghost', 'danger'] as const).map((variant) => (
          <div key={variant} style={{ marginBottom: 'var(--space-4)' }}>
            <div
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: 10,
                color: 'var(--text-faint)',
                marginBottom: 'var(--space-2)',
                letterSpacing: 'var(--tracking-wide)',
              }}
            >
              {variant}
            </div>
            <Row>
              <Button variant={variant} size="sm">{variant} sm</Button>
              <Button variant={variant} size="md">{variant} md</Button>
              <Button variant={variant} size="lg">{variant} lg</Button>
              <Button variant={variant} size="md" loading>loading</Button>
              <Button variant={variant} size="md" disabled>disabled</Button>
            </Row>
          </div>
        ))}
      </SubSection>

      {/* --- Badge --- */}
      <SubSection title="Badge">
        <Row>
          {(['default', 'success', 'warning', 'error', 'info', 'dream'] as const).map((v) => (
            <Badge key={v} variant={v}>{v}</Badge>
          ))}
        </Row>
      </SubSection>

      {/* --- StatusLED --- */}
      <SubSection title="StatusLED">
        <Row gap="var(--space-6)">
          {(['active', 'idle', 'success', 'warning', 'error', 'offline'] as const).map((status) => (
            <div
              key={status}
              style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}
            >
              <StatusLED status={status} size="sm" />
              <span
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 'var(--text-xs)',
                  color: 'var(--text-muted)',
                }}
              >
                {status}
              </span>
            </div>
          ))}
        </Row>
        <div style={{ marginTop: 'var(--space-3)' }}>
          <Row gap="var(--space-6)">
            {(['active', 'warning', 'error'] as const).map((status) => (
              <div
                key={`${status}-pulse`}
                style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}
              >
                <StatusLED status={status} size="md" pulse />
                <span
                  style={{
                    fontFamily: 'var(--font-mono)',
                    fontSize: 'var(--text-xs)',
                    color: 'var(--text-muted)',
                  }}
                >
                  {status} md pulse
                </span>
              </div>
            ))}
          </Row>
        </div>
      </SubSection>

      {/* --- ProgressBar --- */}
      <SubSection title="ProgressBar">
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 'var(--space-4)',
            maxWidth: 400,
          }}
        >
          <div>
            <div
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: 10,
                color: 'var(--text-faint)',
                marginBottom: 'var(--space-2)',
              }}
            >
              default variant
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2)' }}>
              {[0, 25, 50, 75, 100].map((v) => (
                <ProgressBar key={v} value={v} showLabel />
              ))}
            </div>
          </div>
          <div>
            <div
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: 10,
                color: 'var(--text-faint)',
                marginBottom: 'var(--space-2)',
              }}
            >
              cost variant (green → amber → red)
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2)' }}>
              {[20, 50, 80, 95].map((v) => (
                <ProgressBar key={v} value={v} variant="cost" showLabel />
              ))}
            </div>
          </div>
          <div>
            <div
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: 10,
                color: 'var(--text-faint)',
                marginBottom: 'var(--space-2)',
              }}
            >
              height variants (2px / 4px / 8px)
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2)' }}>
              <ProgressBar value={60} height={2} />
              <ProgressBar value={60} height={4} />
              <ProgressBar value={60} height={8} />
            </div>
          </div>
        </div>
      </SubSection>

      {/* --- Spinner --- */}
      <SubSection title="Spinner">
        <Row gap="var(--space-6)">
          {(['sm', 'md', 'lg'] as const).map((size) => (
            <div
              key={size}
              style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}
            >
              <Spinner size={size} />
              <span
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 'var(--text-xs)',
                  color: 'var(--text-muted)',
                }}
              >
                {size}
              </span>
            </div>
          ))}
        </Row>
      </SubSection>

      {/* --- Pill --- */}
      <SubSection title="Pill">
        <Row>
          {['all', 'running', 'idle', 'error', 'complete'].map((label) => (
            <Pill
              key={label}
              active={activePill === label}
              onClick={() => setActivePill(activePill === label ? null : label)}
            >
              {label}
            </Pill>
          ))}
        </Row>
        <div
          style={{
            marginTop: 'var(--space-2)',
            fontFamily: 'var(--font-mono)',
            fontSize: 10,
            color: 'var(--text-faint)',
          }}
        >
          active: {activePill ?? 'none'} — click to toggle
        </div>
      </SubSection>

      {/* --- Sparkline --- */}
      <SubSection title="Sparkline">
        <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-4)' }}>
          {[
            {
              label: 'default — rose, no fill',
              data: [4, 7, 2, 9, 3, 11, 6, 14, 8, 13, 10, 15, 9, 12],
              color: undefined,
              fill: false,
            },
            {
              label: 'sage + fill',
              data: [10, 12, 8, 15, 11, 9, 14, 13, 16, 11, 12],
              color: 'var(--sage)',
              fill: true,
            },
            {
              label: 'dream-bright',
              data: [2, 4, 3, 8, 6, 5, 9, 7, 11, 10, 13, 12],
              color: 'var(--dream-bright)',
              fill: false,
            },
            {
              label: 'ember cost trend + fill',
              data: [1, 2, 1, 3, 4, 6, 5, 8, 9, 11, 13, 15],
              color: 'var(--ember)',
              fill: true,
            },
          ].map(({ label, data, color, fill }) => (
            <div
              key={label}
              style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-4)' }}
            >
              <Sparkline data={data} width={120} height={28} color={color} fill={fill} />
              <span
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 'var(--text-xs)',
                  color: 'var(--text-faint)',
                }}
              >
                {label}
              </span>
            </div>
          ))}
        </div>
      </SubSection>

      {/* --- Tooltip --- */}
      <SubSection title="Tooltip">
        <Row gap="var(--space-6)">
          {(['top', 'bottom', 'left', 'right'] as const).map((side) => (
            <Tooltip key={side} content={`Tooltip ${side}`} side={side}>
              <Button variant="secondary" size="sm">hover ({side})</Button>
            </Tooltip>
          ))}
        </Row>
        <div style={{ marginTop: 'var(--space-3)' }}>
          <Tooltip
            content={
              <span>
                Rich: <span style={{ color: 'var(--rose)' }}>rose</span> accent
              </span>
            }
          >
            <Button variant="ghost" size="sm">hover (rich content)</Button>
          </Tooltip>
        </div>
      </SubSection>
    </Section>
  );
}

// ---------------------------------------------------------------------------
// 4. Spacing
// ---------------------------------------------------------------------------

const SPACING_TOKENS: Array<{ token: string; px: number }> = [
  { token: '--space-1',  px: 4  },
  { token: '--space-2',  px: 8  },
  { token: '--space-3',  px: 12 },
  { token: '--space-4',  px: 16 },
  { token: '--space-5',  px: 20 },
  { token: '--space-6',  px: 24 },
  { token: '--space-8',  px: 32 },
  { token: '--space-10', px: 40 },
  { token: '--space-12', px: 48 },
  { token: '--space-16', px: 64 },
];

function SpacingSection() {
  return (
    <Section title="Spacing">
      <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2)' }}>
        {SPACING_TOKENS.map(({ token, px }) => (
          <div
            key={token}
            style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-4)' }}
          >
            <span
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: 10,
                color: 'var(--text-ghost)',
                width: 88,
                flexShrink: 0,
                letterSpacing: 'var(--tracking-wide)',
              }}
            >
              {token}
            </span>
            <div
              style={{
                height: 16,
                width: px * 2,
                background: 'var(--rose-dim)',
                opacity: 0.55,
                flexShrink: 0,
              }}
            />
            <span
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: 10,
                color: 'var(--text-faint)',
              }}
            >
              {px}px
            </span>
          </div>
        ))}
      </div>
    </Section>
  );
}

// ---------------------------------------------------------------------------
// 5. Motion
// ---------------------------------------------------------------------------

function MotionDemo() {
  const [hovered, setHovered] = useState<string | null>(null);

  return (
    <Row gap="var(--space-4)">
      {([
        { key: 'fast',   label: 'fast (80ms)',    dur: 'var(--duration-fast)' },
        { key: 'normal', label: 'normal (150ms)', dur: 'var(--duration-normal)' },
        { key: 'slow',   label: 'slow (300ms)',   dur: 'var(--duration-slow)' },
      ] as const).map(({ key, label, dur }) => (
        <div
          key={key}
          onMouseEnter={() => setHovered(key)}
          onMouseLeave={() => setHovered(null)}
          style={{
            padding: 'var(--space-3) var(--space-4)',
            border: `1px solid ${hovered === key ? 'var(--rose)' : 'var(--border-default)'}`,
            background: hovered === key ? 'var(--bg-highlight)' : 'transparent',
            color: hovered === key ? 'var(--text-strong)' : 'var(--text-muted)',
            fontFamily: 'var(--font-mono)',
            fontSize: 'var(--text-xs)',
            cursor: 'default',
            transition: `all ${dur} var(--ease-out)`,
          }}
        >
          {label}
        </div>
      ))}
    </Row>
  );
}

function MotionSection() {
  return (
    <Section title="Motion">
      <SubSection title="Duration">
        <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2)' }}>
          {[
            { token: '--duration-fast',   value: '80ms',   desc: 'Micro-interactions: opacity, colour, border' },
            { token: '--duration-normal', value: '150ms',  desc: 'Standard interaction feedback' },
            { token: '--duration-slow',   value: '300ms',  desc: 'Panels, drawers, content reveals' },
          ].map(({ token, value, desc }) => (
            <div
              key={token}
              style={{ display: 'flex', alignItems: 'baseline', gap: 'var(--space-4)' }}
            >
              <span
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 10,
                  color: 'var(--text-ghost)',
                  width: 148,
                  flexShrink: 0,
                }}
              >
                {token}
              </span>
              <span
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 'var(--text-xs)',
                  color: 'var(--rose)',
                  width: 60,
                  flexShrink: 0,
                }}
              >
                {value}
              </span>
              <span
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 'var(--text-xs)',
                  color: 'var(--text-faint)',
                }}
              >
                {desc}
              </span>
            </div>
          ))}
        </div>
      </SubSection>

      <SubSection title="Easing">
        <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2)' }}>
          {[
            { token: '--ease-out',   value: 'cubic-bezier(0.0, 0.0, 0.2, 1)', desc: 'Entrances' },
            { token: '--ease-in',    value: 'cubic-bezier(0.4, 0.0, 1, 1)',   desc: 'Exits' },
            { token: '--ease-inout', value: 'cubic-bezier(0.4, 0.0, 0.2, 1)', desc: 'State changes (in + out)' },
          ].map(({ token, value, desc }) => (
            <div
              key={token}
              style={{ display: 'flex', alignItems: 'baseline', gap: 'var(--space-4)' }}
            >
              <span
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 10,
                  color: 'var(--text-ghost)',
                  width: 148,
                  flexShrink: 0,
                }}
              >
                {token}
              </span>
              <span
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 'var(--text-xs)',
                  color: 'var(--rose)',
                  width: 272,
                  flexShrink: 0,
                }}
              >
                {value}
              </span>
              <span
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 'var(--text-xs)',
                  color: 'var(--text-faint)',
                }}
              >
                {desc}
              </span>
            </div>
          ))}
        </div>
      </SubSection>

      <SubSection title="Live Demo — hover to compare durations">
        <MotionDemo />
      </SubSection>
    </Section>
  );
}

// ---------------------------------------------------------------------------
// 6. Shadows & Elevation
// ---------------------------------------------------------------------------

function ShadowsSection() {
  return (
    <Section title="Shadows & Elevation">
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 'var(--space-6)' }}>
        {[
          { token: '--shadow-sm',    desc: 'Cards, inline dropdowns' },
          { token: '--shadow-md',    desc: 'Floating panels, popovers' },
          { token: '--shadow-lg',    desc: 'Modals, drawers' },
          { token: '--shadow-rose',  desc: 'Focused elements, active indicators' },
          { token: '--shadow-dream', desc: 'Speculative / AI highlights' },
        ].map(({ token, desc }) => (
          <div
            key={token}
            style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2)' }}
          >
            <div
              style={{
                width: 80,
                height: 60,
                background: 'var(--bg-raised)',
                border: '1px solid var(--border-default)',
                boxShadow: `var(${token})`,
              }}
            />
            <span
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: 10,
                color: 'var(--text-faint)',
                letterSpacing: 'var(--tracking-wide)',
              }}
            >
              {token}
            </span>
            <span
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: 10,
                color: 'var(--text-ghost)',
              }}
            >
              {desc}
            </span>
          </div>
        ))}
      </div>
    </Section>
  );
}

// ---------------------------------------------------------------------------
// 7. Borders
// ---------------------------------------------------------------------------

function BordersSection() {
  return (
    <Section title="Borders">
      <Row gap="var(--space-8)">
        {[
          { token: '--border-default', label: 'default', desc: 'Resting — ghost-tone' },
          { token: '--border-hover',   label: 'hover',   desc: 'Hover — rose-dim' },
          { token: '--border-active',  label: 'active',  desc: 'Active / focus — rose' },
        ].map(({ token, label, desc }) => (
          <div
            key={token}
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: 'var(--space-2)',
              alignItems: 'flex-start',
            }}
          >
            <div
              style={{
                width: 80,
                height: 40,
                border: `1px solid var(${token})`,
                background: 'transparent',
              }}
            />
            <span
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: 10,
                color: 'var(--text-faint)',
              }}
            >
              {token}
            </span>
            <span
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: 10,
                color: 'var(--text-ghost)',
              }}
            >
              {label} — {desc}
            </span>
          </div>
        ))}
      </Row>
    </Section>
  );
}

// ---------------------------------------------------------------------------
// 8. Z-index scale
// ---------------------------------------------------------------------------

function ZIndexSection() {
  const layers = [
    { token: '--z-base',     value: 0,   label: 'base' },
    { token: '--z-raised',   value: 10,  label: 'raised' },
    { token: '--z-dropdown', value: 100, label: 'dropdown' },
    { token: '--z-sticky',   value: 200, label: 'sticky' },
    { token: '--z-overlay',  value: 300, label: 'overlay' },
    { token: '--z-modal',    value: 400, label: 'modal' },
    { token: '--z-toast',    value: 500, label: 'toast' },
    { token: '--z-tooltip',  value: 600, label: 'tooltip' },
  ];

  return (
    <Section title="Z-index Scale">
      <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2)' }}>
        {layers.map(({ token, value, label }) => (
          <div
            key={token}
            style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-4)' }}
          >
            <span
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: 10,
                color: 'var(--text-ghost)',
                width: 110,
                flexShrink: 0,
              }}
            >
              {token}
            </span>
            <div
              style={{
                height: 10,
                width: Math.max(4, (value / 600) * 200),
                background: 'var(--rose-dim)',
                opacity: 0.4 + (value / 600) * 0.55,
                flexShrink: 0,
              }}
            />
            <span
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: 10,
                color: 'var(--text-faint)',
                width: 36,
                textAlign: 'right',
                fontVariantNumeric: 'tabular-nums',
              }}
            >
              {value}
            </span>
            <span
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: 10,
                color: 'var(--text-ghost)',
              }}
            >
              {label}
            </span>
          </div>
        ))}
      </div>
    </Section>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function DesignPage() {
  return (
    <div
      style={{
        background: 'var(--void)',
        minHeight: '100vh',
        color: 'var(--text-strong)',
        fontFamily: 'var(--font-mono)',
      }}
    >
      {/* Page header */}
      <div
        style={{
          padding: 'var(--space-8)',
          borderBottom: '1px solid var(--border-default)',
          marginBottom: 'var(--space-8)',
          background: 'var(--bg-raised)',
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'baseline',
            gap: 'var(--space-4)',
            flexWrap: 'wrap',
          }}
        >
          <h1
            style={{
              fontSize: 'var(--text-lg)',
              fontWeight: 600,
              letterSpacing: 'var(--tracking-tight)',
              color: 'var(--text-strong)',
              margin: 0,
            }}
          >
            ROSEDUST
          </h1>
          <span
            style={{
              fontSize: 'var(--text-xs)',
              color: 'var(--text-faint)',
              letterSpacing: 'var(--tracking-wide)',
            }}
          >
            Design System · Token Reference
          </span>
          <Badge variant="dream">dev only</Badge>
        </div>
        <p
          style={{
            marginTop: 'var(--space-2)',
            fontSize: 'var(--text-xs)',
            color: 'var(--text-faint)',
            lineHeight: 'var(--leading-relaxed)',
          }}
        >
          Living style guide — every ROSEDUST design token, component variant,
          and visual primitive. Hard-edged. Mono. Zero border-radius.
        </p>
      </div>

      {/* Content */}
      <div
        style={{
          maxWidth: 960,
          margin: '0 auto',
          padding: '0 var(--space-8) var(--space-16)',
        }}
      >
        <ColorsSection />
        <TypographySection />
        <ComponentsSection />
        <SpacingSection />
        <MotionSection />
        <ShadowsSection />
        <BordersSection />
        <ZIndexSection />
      </div>
    </div>
  );
}
