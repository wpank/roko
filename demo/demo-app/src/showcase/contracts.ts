/**
 * Showcase data contracts (S10 §5): the provenance envelope, the bundle manifest, the event
 * stream and the R1 read models.
 *
 * The browser computes no statistic. Every displayed number is an `Estimate` copied from one
 * S08 `vb.metric_record/1` row in the bundle's `data/metrics.jsonl`; `metric_ref` names that
 * row as `m-` plus the first 16 hex digits of the SHA-256 of its JSON line. Each view also
 * lists the metrics it shows (`metrics`), so the render guard can check n and the CI of every
 * number before anything is drawn.
 *
 * `./schemas/*.schema.json` describe the same shapes in the JSON Schema subset that
 * `benchmarks/viabilitybench/schema/validate.py` implements (type, required, enum, const,
 * properties, items, minItems, additionalProperties), so the Python bundle builder and the
 * e2e specs check bundles against one definition. Keep the two in step.
 */

export const PROVENANCE_SCHEMA = 'showcase-provenance/1';
export const BUNDLE_SCHEMA = 'showcase-bundle/1';
export const MANIFEST_SCHEMA = 'showcase-manifest/1';
export const EVENT_SCHEMA = 'showcase-event/1';

/** The R1 read models, by view id (S10 §5.2). */
export type ViewId = 'overview' | 'p1-head-to-head' | 'm4-audits';

export const R1_VIEWS: readonly ViewId[] = ['overview', 'p1-head-to-head', 'm4-audits'];

export const VIEW_SCHEMAS: Readonly<Record<ViewId, string>> = {
  overview: 'showcase-view/overview/1',
  'p1-head-to-head': 'showcase-view/p1-head-to-head/1',
  'm4-audits': 'showcase-view/m4-audits/1',
};

/**
 * The S09 experiment that will first measure a view, shown as "not yet measured · planned in
 * <id>" when a bundle has no data for it (S10 §4.1).
 */
export const PLANNED_IN: Readonly<Record<ViewId, string>> = {
  overview: 'PILOT',
  'p1-head-to-head': 'LOG1',
  'm4-audits': 'E-H5-live',
};

/**
 * Where a view's data came from. Only `measured` and `replay` render in production; `fixture`
 * renders only in a test build with `VITE_ALLOW_FIXTURES=1` (S10 §4.5).
 */
export type ProvenanceKind = 'measured' | 'replay' | 'fixture';

/** S09's claim states, copied from the pre-registered test record and never phrased by the UI. */
export type ClaimState = 'SUPPORTED' | 'NOT_SUPPORTED' | 'INCONCLUSIVE' | 'NOT_YET_MEASURED';

export const CLAIM_STATES: readonly ClaimState[] = [
  'SUPPORTED',
  'NOT_SUPPORTED',
  'INCONCLUSIVE',
  'NOT_YET_MEASURED',
];

/** Why the render guard refused a view; `RefusedPanel` names the code (S10 §4.3 L). */
export type RefusalReason =
  | 'simulated'
  | 'fixture'
  | 'no_run_ids'
  | 'integrity'
  | 'missing_provenance'
  | 'ci_missing'
  | 'n_missing';

export const REFUSAL_REASONS: readonly RefusalReason[] = [
  'simulated',
  'fixture',
  'no_run_ids',
  'integrity',
  'missing_provenance',
  'ci_missing',
  'n_missing',
];

/** How a metric is drawn: rates on a fixed [0,1] axis, dollars on a labelled log axis. */
export type MetricKind = 'rate' | 'usd' | 'ratio' | 'count' | 'score';

/** S01 §4.4: where a dollar figure came from. `cli_usage` is the CLI's API-equivalent. */
export type CostSource = 'provider_usage' | 'cli_usage';

// ── Provenance envelope `showcase-provenance/1` (S10 §5.1) ─────────────────────

export interface ProvenanceSource {
  /** Path inside the bundle, e.g. `data/metrics.jsonl`. */
  path: string;
  /** The record schema of the file, e.g. `vb.metric_record/1`. */
  schema: string;
  /** SHA-256 of the file, hex. */
  sha256: string;
  rows: number;
  simulated: boolean;
  /**
   * Set by the loader, never trusted from the file: true only when the file's digest matches
   * both `sha256` and the bundle's `SHA256SUMS` (S10 §4.8). Bundles store `false`.
   */
  sha256_verified: boolean;
}

export interface CiSpec {
  /** e.g. `paired_stratified_bootstrap`, `wilson`, `ht_wilson_eff_n`. */
  method: string;
  strata: string[];
  level: number;
  resamples: number | null;
}

export interface Provenance {
  schema: typeof PROVENANCE_SCHEMA;
  kind: ProvenanceKind;
  simulated: boolean;
  bundle_id: string;
  /** S09 experiment ids. */
  experiment_ids: string[];
  run_ids: string[];
  sources: ProvenanceSource[];
  harness_commit: string;
  dirty: boolean;
  analysis_commit: string;
  /** S01 BLAKE3 config hashes, `b3:` strings. */
  config_hashes: string[];
  price_snapshot_id: string;
  models: string[];
  n: number;
  seeds: number[];
  window: { from: string; to: string };
  estimator: string;
  record_filter: string;
  ci: CiSpec;
  cost_usd: number;
  generated_at: string;
  reproduce: string[];
}

// ── Numbers ───────────────────────────────────────────────────────────────────

/** One displayed number with its interval, copied from a metric record. */
export interface Estimate {
  /** `null` when undefined, e.g. $/verified with no verified success. */
  value: number | null;
  /** `[low, high]`, or `null` for a metric without an interval. */
  ci: number[] | null;
  metric_ref: string;
}

/** A metric shown in a view, with what the guard and the drawer need to know about it. */
export interface ViewMetric {
  metric_ref: string;
  /** S08 metric name, e.g. `vs_rate`, `usd_per_vs`, `pass_hat_3`, `false_green_rate`. */
  metric: string;
  kind: MetricKind;
  value: number | null;
  ci: number[] | null;
  /** e.g. `wilson`, `paired_stratified_bootstrap`; `none` for a metric without an interval. */
  ci_method: string;
  n: number;
  estimator: string;
  arm: string | null;
  envelope_level: number | null;
}

export interface Claim {
  /** S09 hypothesis id, e.g. `H1`. */
  hypothesis: string;
  state: ClaimState;
  /** S09's lock hash; null for descriptive analyses such as the pilot. */
  prereg_id: string | null;
  /** S09 experiment ids that will measure it. */
  planned_in: string[];
  /** The claim sentence, copied from S09's pre-registered test record. */
  text: string;
}

/** Fields every read model carries. */
export interface ViewBase {
  schema: string;
  metrics: ViewMetric[];
  provenance: Provenance;
}

// ── Overview `showcase-view/overview/1` ────────────────────────────────────────

export interface TileRow {
  label: string;
  estimate: Estimate;
}

export interface OverviewTile {
  id: string;
  pillar: 'P1' | 'P2';
  /** `M1`–`M4` for P2 tiles. */
  mechanism: string | null;
  title: string;
  hypothesis: string | null;
  claim_state: ClaimState;
  rows: TileRow[];
  planned_in: string[];
  /** The view the tile opens. */
  view: ViewId | null;
}

export type NegativeKind = 'frontier_wins' | 'loop_dormant' | 'fgr_above_bound' | 'other';

/** A result against the thesis, shown in the always-visible "where it does not hold" strip. */
export interface Negative {
  id: string;
  kind: NegativeKind;
  text: string;
  rows: TileRow[];
  view: ViewId | null;
}

export interface OverviewView extends ViewBase {
  schema: 'showcase-view/overview/1';
  tiles: OverviewTile[];
  negatives: Negative[];
}

// ── P1 head-to-head `showcase-view/p1-head-to-head/1` ─────────────────────────

export interface PassHatK {
  '1': Estimate;
  '3': Estimate;
  '5': Estimate;
}

/**
 * One arm (S08 §4.9 arm ids). D1/D2: three arms plus the 48-task frontier·roko probe; extra
 * frontier-direct rows (`fd_claude_lite`, `fd_codex`) are `extra`.
 */
export interface ArmRow {
  arm: string;
  /** Display label from the bundle, e.g. `cheap·roko`. */
  label: string;
  tier: 'cheap' | 'frontier';
  harness: 'direct' | 'roko';
  role: 'arm' | 'probe' | 'extra';
  /** e.g. `probe (48 H3 tasks)`. */
  note: string | null;
  models: string[];
  status: 'run' | 'not_run';
  /** Why the arm was not run. */
  reason: string | null;
  n_tasks: number;
  n_trials: number;
  resolve: Estimate | null;
  usd_per_verified: Estimate | null;
  cost_source: CostSource | null;
  pass_hat_k: PassHatK | null;
  outcome_sd: Estimate | null;
}

export type EnvelopeVerdict = 'within_target' | 'frontier_wins' | 'inconclusive' | 'not_measured';

/** S09 §4.2 cumulative envelope level j (tasks with ℓ ≤ j), or a named slice such as SWE-V. */
export interface EnvelopeRow {
  envelope_level: number | null;
  label: string;
  n_tasks: number;
  cheap_resolve: Estimate | null;
  frontier_resolve: Estimate | null;
  resolve_ratio: Estimate | null;
  usd_ratio: Estimate | null;
  verdict: EnvelopeVerdict;
}

export interface HeadToHeadView extends ViewBase {
  schema: 'showcase-view/p1-head-to-head/1';
  claim: Claim;
  arms: ArmRow[];
  /** The frontier line is computed offline; the chart only joins `frontier_arms` in order. */
  pareto: { x: 'usd_per_verified'; y: 'resolve'; frontier_arms: string[] };
  envelope: EnvelopeRow[];
}

// ── M4 audit lottery `showcase-view/m4-audits/1` ──────────────────────────────

/** S05 §4.3 battery check ids. */
export type AuditCheckKind = 'A1_tamper' | 'A2_rerun' | 'B1_hidden' | 'B2_mutation' | 'B3_review';

export interface CheckCount {
  run: number;
  failed: number;
}

export interface AuditDraw {
  audit_id: string;
  attempt_key: string;
  /** Inclusion probability of the draw. */
  pi: number;
  checks: { kind: AuditCheckKind; passed: boolean }[];
  outcome: 'clean' | 'caught' | 'inconclusive';
  feedback_actions: { target: string; from: number | string; to: number | string }[];
}

/** The false-green rate: the Hájek ratio form of the HT estimator, Wilson on n_eff (S05 SC1). */
export interface FalseGreen extends Estimate {
  estimator: string;
  ci_method: string;
  n_eff: number;
  /** The S06 essential-variable bound, when one is set. */
  bound: number | null;
}

export interface SeriesPoint {
  t: string;
  estimate: Estimate;
}

export interface M4AuditsView extends ViewBase {
  schema: 'showcase-view/m4-audits/1';
  claim: Claim;
  visible_passes: number;
  policy: { pi_base: number; pi_honeypot: number; pi_gaming_prone: number };
  audited: number;
  caught: number;
  false_green: FalseGreen | null;
  gaming_rate: Estimate | null;
  detector: { honeypots: number; recall: Estimate | null; precision: Estimate | null };
  by_check: {
    A1_tamper: CheckCount;
    A2_rerun: CheckCount;
    B1_hidden: CheckCount;
    B2_mutation: { run: number; mean_score: Estimate | null };
    B3_review: { run: number; agree_with_B1: number };
  };
  draws: AuditDraw[];
  series: SeriesPoint[];
}

export type ShowcaseView = OverviewView | HeadToHeadView | M4AuditsView;

/** The read model type of each view id. */
export interface ViewTypes {
  overview: OverviewView;
  'p1-head-to-head': HeadToHeadView;
  'm4-audits': M4AuditsView;
}

// ── Bundle manifest `showcase-bundle/1` (S10 §5.5) ─────────────────────────────

export interface BundleFile {
  path: string;
  schema: string;
  sha256: string;
  rows: number;
}

export interface BundleManifest {
  schema: typeof BUNDLE_SCHEMA;
  bundle_id: string;
  kind: ProvenanceKind;
  simulated: boolean;
  title: string;
  created_at: string;
  featured: boolean;
  experiment_ids: string[];
  run_ids: string[];
  harness_commit: string;
  dirty: boolean;
  analysis_commit: string;
  config_hashes: string[];
  price_snapshot_id: string;
  models: string[];
  views: ViewId[];
  cost_usd: number;
  redaction: {
    transcripts: 'excluded';
    prompts: 'sha256';
    hidden_tests: 'sha256';
    diffs: 'included' | 'excluded';
  };
  files: BundleFile[];
  reproduce: string[];
}

/** One bundle in the index. `status` is the staging or server verdict; the client checks anyway. */
export interface BundleSummary {
  id: string;
  title: string;
  status: 'verified' | 'rejected' | 'unverified';
  created_at: string;
  featured: boolean;
}

/** `bundles/index.json`, the static twin of `GET /api/showcase/manifest`. */
export interface ShowcaseManifest {
  schema: typeof MANIFEST_SCHEMA;
  showcase_mode: boolean;
  live_enabled: boolean;
  featured_bundle: string | null;
  bundles: BundleSummary[];
  harness_commit: string | null;
}

// ── Events `showcase-event/1` (S10 §5.4) ───────────────────────────────────────

export type ShowcaseEventType =
  | 'run.started'
  | 'run.finished'
  | 'task.started'
  | 'task.finished'
  | 'm3.prediction'
  | 'route.decision'
  | 'escalation'
  | 'gate.verdict'
  | 'm4.audit'
  | 'm1.ev'
  | 'm1.disturbance'
  | 'm1.action'
  | 'm2.verdict'
  | 'metric.update'
  | 'budget.update';

export interface ShowcaseEvent {
  schema: typeof EVENT_SCHEMA;
  seq: number;
  t_ms: number;
  source: 'replay' | 'live';
  run_id: string | null;
  attempt_key: string | null;
  type: ShowcaseEventType;
  payload: Record<string, unknown>;
}

/** The payload of a `metric.update` event: the running value of one metric record. */
export interface MetricUpdatePayload {
  metric_ref: string;
  value: number | null;
  ci: number[] | null;
  n: number;
}
