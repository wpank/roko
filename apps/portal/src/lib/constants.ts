/**
 * Shared application-level constants for the roko portal.
 *
 * All values are declared `as const` so TypeScript narrows them to their
 * literal types, enabling exhaustive checks and autocomplete.
 */

// ---------------------------------------------------------------------------
// Ring buffer / data-retention limits
// ---------------------------------------------------------------------------

/**
 * Maximum number of entries to retain in each in-memory ring buffer.
 *
 * These limits prevent unbounded memory growth in long-running dashboard
 * sessions where the server streams many events.
 */
export const RING_BUFFER_LIMITS = {
  /** Recent gate execution results kept in the gate ring. */
  gates: 256,
  /** Recent agent episodes kept in the episode ring. */
  episodes: 128,
  /** Recent error events kept in the error ring. */
  errors: 64,
  /** General-purpose event log entries shown in the Observe tab. */
  eventLog: 200,
  /** Lines of agent output retained for the current task. */
  agentOutputLines: 500,
  /** Lines of gate output retained for the current gate run. */
  gateOutputLines: 500,
} as const;

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

/**
 * Top-level navigation items rendered in the sidebar.
 *
 * `icon` values correspond to lucide-react component names and are resolved
 * dynamically in the Nav component.
 */
export const NAV_ITEMS = [
  { label: 'Overview',      href: '/',             icon: 'LayoutDashboard' },
  { label: 'Work',          href: '/work',         icon: 'Briefcase' },
  { label: 'Agents',        href: '/agents',       icon: 'Bot' },
  { label: 'Intelligence',  href: '/intelligence', icon: 'Brain' },
  { label: 'Observe',       href: '/observe',      icon: 'Eye' },
  { label: 'Providers',     href: '/providers',    icon: 'Activity' },
  { label: 'Settings',      href: '/settings',     icon: 'Settings' },
] as const;

// ---------------------------------------------------------------------------
// Color mappings
// ---------------------------------------------------------------------------

/**
 * CSS variable–backed color for each agent role label.
 *
 * Keys match the `role` field on agent/task records returned by roko-serve.
 */
export const ROLE_COLORS: Record<string, string> = {
  implementer: 'var(--rose)',
  strategist:  'var(--dream)',
  architect:   'var(--bone)',
  auditor:     'var(--sage)',
  critic:      'var(--ember)',
  conductor:   'var(--accent-cyan)',
  researcher:  'var(--dream-bright)',
} as const;

/**
 * CSS variable–backed color for each task/plan execution status.
 *
 * Keys match the `status` discriminant on task and plan records.
 */
export const STATUS_COLORS: Record<string, string> = {
  pending:   'var(--text-muted)',
  running:   'var(--warning)',
  gating:    'var(--dream)',
  completed: 'var(--sage)',
  failed:    'var(--accent-error)',
  paused:    'var(--text-faint)',
  cancelled: 'var(--text-ghost)',
} as const;

/**
 * CSS variable–backed color for each knowledge tier level.
 *
 * Tiers progress from `transient` (low confidence) to `persistent` (high
 * confidence) as evidence accumulates.
 */
export const TIER_COLORS: Record<string, string> = {
  transient:    'var(--warning)',
  working:      'var(--dream)',
  consolidated: 'var(--rose-dim)',
  persistent:   'var(--rose-glow)',
} as const;

/**
 * CSS variable–backed color for each log/event severity level.
 */
export const SEVERITY_COLORS: Record<string, string> = {
  error:   'var(--ember)',
  warning: 'var(--warning)',
  success: 'var(--sage)',
  info:    'var(--dream)',
} as const;
