# TraceCommons UX and Dashboard Design

## What is TraceCommons?

TraceCommons is a privacy-preserving platform for collecting, scoring, and
crediting coding agent traces. When a developer uses a coding agent (Claude
Code, Codex, or any tool producing trajectory-v1 files), the agent generates
a trace -- a structured record of user messages, assistant responses, tool
calls, and outcomes. TraceCommons lets developers voluntarily contribute
these traces to a shared corpus, where each trace is:

1. **Redacted** -- sensitive data (secrets, PII, paths) is stripped locally
   before the trace leaves the contributor's machine.
2. **Scored** -- a TEE-attested scoring pipeline evaluates perplexity (how
   surprising the content is to a language model) and novelty (how distinct
   it is from existing corpus entries).
3. **Credited** -- contributors earn credits proportional to a multiplicative
   quality score `q = f(perplexity) * g(novelty) * a(anomaly)`, with
   anti-gaming properties (concave, log-saturating, anomaly-gated).
4. **Governed by consent** -- contributors choose scope tiers
   (debugging/evaluation, benchmark generation, ranking training, model
   training, public attribution) and can revoke submissions at any time.

The system today has six Rust crates:

| Crate | Role |
|---|---|
| `trace-commons-contributor` | CLI client: discover, redact, submit traces |
| `trace-commons-server` | Hosted ingest, worker, admin, review APIs |
| `trace-commons-protocol` | Shared types: envelopes, consent, onboarding |
| `trace-commons-gate-api` | Gate contract traits: embedder, perplexity scorer, vector index |
| `trace-commons-gate-enclave` | TEE-side scoring implementations |
| `trace-commons-operator-client` | HTTP client + output formatting for operator tools |

The existing user-facing surface is a CLI (`trace-commons-contributor`) with
subcommands: `login`, `list`, `submit`, `status`, `attest`, `profile`,
`whoami`, `logout`, and `mint-grant`. There is no TUI, no web dashboard, no
real-time event stream. This document designs all of those.

---

## 1. Terminal UI (TUI) for Contributors

**Priority: P0** | **Complexity: Medium (2-3 weeks)** | **Crate: `tc-tui`**

### User Stories

- As a contributor, I want a live dashboard showing my submission pipeline so
  I can watch traces get scored without re-running `status` in a loop.
- As a contributor, I want to see my credit balance and quality trend at a
  glance so I know whether my traces are valuable.
- As an operator, I want to see corpus health metrics so I can diagnose
  scoring pipeline issues without querying the database.

### Technology

- **ratatui** + **crossterm** for terminal rendering (same stack as roko's
  existing TUI; battle-tested, no external runtime dependency).
- Binary: `tc-tui` or integrated as `trace-commons-contributor dashboard`.
- Data source: polls `/v1/contributor/summary` and `/v1/events/stream` (SSE).
  Falls back to local receipt file when offline.

### Tab Layout

```
+-- TraceCommons Dashboard -----------------------------------------------+
| [F1 Overview] [F2 My Traces] [F3 Quality] [F4 Credits] [F5 Settings]   |
+-------------------------------------------------------------------------+
```

#### F1: Overview Tab

```
+-- Overview ----------------------------- tc-tui v0.1.0 -- 2026-08-10 --+
|                                                                         |
|  Contributor: zaki-dev (sha256:ad74...bd83)                             |
|  Enrolled:    2026-07-15    Scopes: debugging_evaluation, model_training|
|  Issuer:      issuer.tracecommons.ai                                    |
|                                                                         |
|  +-- Summary --------------------------------------------------------+ |
|  |  Traces Submitted    142     Accepted   138     Quarantined    4   | |
|  |  Acceptance Rate     97.2%   Avg Quality  0.63  Credits Earned 847| |
|  |  Last Submission     2m ago  Last Score   0.71  Corpus Size  28.4K| |
|  +-------------------------------------------------------------------+ |
|                                                                         |
|  +-- Recent Activity ------------------------------------------------+ |
|  |  14:32:01  submitted  claude-code  myproject       0.71   +5.2 cr | |
|  |  14:31:44  submitted  codex        api-refactor    0.68   +4.8 cr | |
|  |  14:30:12  submitted  claude-code  myproject       0.45   +2.1 cr | |
|  |  14:29:58  quarantine claude-code  test-fixtures   --     +0.0 cr | |
|  |  14:28:33  submitted  claude-code  utils           0.82   +7.1 cr | |
|  +-------------------------------------------------------------------+ |
|                                                                         |
|  +-- Live Pipeline ---------------------------------------------------+ |
|  |  [=====>          ] 3 traces scoring...  avg latency: 1.2s        | |
|  +-------------------------------------------------------------------+ |
|                                                                         |
|  q: quit  r: refresh  s: submit new  ?: help                           |
+---------+-----------+---------+----------+------------------------------+
```

#### F2: My Traces Tab

```
+-- My Traces ------------------------------------------- page 1 of 12 --+
|                                                                         |
|  Filter: [all sources v]  [last 7 days v]  [all statuses v]  [Search_] |
|                                                                         |
|  #   Source       Project          Submitted    Status     Quality  Cr  |
|  --- ------------ ---------------- ------------ ---------- ------- --- |
|  142 claude-code  myproject        2m ago       accepted   0.71   5.2  |
|  141 codex        api-refactor     3m ago       accepted   0.68   4.8  |
|  140 claude-code  myproject        5m ago       accepted   0.45   2.1  |
|  139 claude-code  test-fixtures    7m ago       quarantine --     0.0  |
|  138 claude-code  utils            9m ago       accepted   0.82   7.1  |
|  137 trajectory   openhands-bench  12m ago      accepted   0.91   9.3  |
|  136 claude-code  myproject        1h ago       accepted   0.55   3.4  |
|  ...                                                                    |
|                                                                         |
|  [Enter] detail  [d] drill-down  [/] search  [</>] page  [e] export   |
+---------+-----------+---------+----------+------------------------------+
```

#### F3: Quality Tab

```
+-- Quality Analytics ------------------------------------ 7-day view  --+
|                                                                         |
|  Quality Score Distribution (last 7 days, n=87)                         |
|                                                                         |
|   1.0 |                              *                                  |
|   0.9 |                         * *  * *                                |
|   0.8 |                    *  * * * ** * *                              |
|   0.7 |               *  ** ** * * * ** * * *                           |
|   0.6 |          *  * ** ** ** * * * ** * * * *                         |
|   0.5 |     *  * ** ** ** ** ** * * * ** * * * *                        |
|   0.4 |  *  ** ** ** ** ** ** ** * * * ** * * *                         |
|   0.3 |  ** ** ** ** ** ** ** ** * *                                    |
|   0.2 |  ** ** ** **                                                    |
|   0.1 |  **                                                             |
|       +------------------------------------------------------------     |
|        Mon   Tue   Wed   Thu   Fri   Sat   Sun                          |
|                                                                         |
|  Perplexity (rep):  avg 14.2   p50 12.8   p90 28.1   floor 6.0         |
|  Novelty:           avg 0.72   p50 0.68   p90 0.94   floor 0.5         |
|  Anomaly ratio:     avg 1.4    max 2.8    withheld 0                    |
|                                                                         |
|  [1/7/30] range  [p] perplexity chart  [n] novelty chart                |
+---------+-----------+---------+----------+------------------------------+
```

#### F4: Credits Tab

```
+-- Credits ---------------------------------------------- balance: 847 -+
|                                                                         |
|  +-- Balance ---------------------------------------------------------+|
|  |  Total Earned      847.3 cr   This Week    +124.6 cr               ||
|  |  Pending            12.4 cr   Last Week    +98.2 cr                ||
|  |  Lifetime Rank     #42 / 1,204 contributors                       ||
|  +--------------------------------------------------------------------+|
|                                                                         |
|  +-- Recent Transactions --------------------------------------------+|
|  |  Time       Submission              Quality   Credits   Balance    ||
|  |  --------   ----------------------  -------   -------   --------   ||
|  |  14:32:01   sub-a8f3...7d21         0.71      +5.2      847.3     ||
|  |  14:31:44   sub-c4e1...9a03         0.68      +4.8      842.1     ||
|  |  14:30:12   sub-2b7a...f412         0.45      +2.1      837.3     ||
|  |  14:28:33   sub-e91d...5c88         0.82      +7.1      835.2     ||
|  |  ...                                                               ||
|  +--------------------------------------------------------------------+|
|                                                                         |
|  Credit formula: credits = base_rate * q                                |
|  where q = f(perplexity) * g(novelty) * a(anomaly_ratio)               |
|  f, g: log-concave, saturating at ceilings (38.5 ppl, 1.0 nov)         |
|                                                                         |
|  [t] transaction detail  [h] history export  [?] formula explanation    |
+---------+-----------+---------+----------+------------------------------+
```

#### F5: Settings Tab

```
+-- Settings ---------------------------------------------------------   +
|                                                                         |
|  Identity                                                               |
|    Device Key ID:   sha256:ad745f4e...c177bd83                          |
|    Tenant:          tenant-7a3b...e912                                   |
|    Instance:        pilot-alpha-1                                        |
|                                                                         |
|  Consent Scopes                                                         |
|    [x] Debugging & Evaluation    (always on)                            |
|    [ ] Benchmark Generation                                             |
|    [x] Ranking-Model Training                                           |
|    [x] Model Training                                                   |
|    [ ] Public Attribution                                               |
|                                                                         |
|  Endpoints                                                              |
|    Issuer:   https://issuer.tracecommons.ai                             |
|    Ingest:   https://ingest.tracecommons.ai                             |
|    Audience: trace-commons-ingest                                       |
|                                                                         |
|  PII Filter: near-ai (TRACE_NEAR_AI_PRIVACY_API_KEY set)               |
|  Config Dir: ~/.config/trace-commons/                                   |
|                                                                         |
|  [u] update scopes  [p] update profile  [l] logout                     |
+---------+-----------+---------+----------+------------------------------+
```

### Implementation Sketch (Rust)

```rust
// tc-tui/src/main.rs
use crossterm::event::{self, Event, KeyCode};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc;

#[derive(Clone, Copy, PartialEq)]
enum Tab { Overview, Traces, Quality, Credits, Settings }

struct App {
    tab: Tab,
    summary: ContributorSummary,
    traces: Vec<TraceRow>,
    credits: CreditHistory,
    sse_rx: mpsc::Receiver<PipelineEvent>,
}

impl App {
    fn render(&self, frame: &mut Frame) {
        let tabs = Tabs::new(vec!["Overview", "My Traces", "Quality", "Credits", "Settings"])
            .select(self.tab as usize)
            .highlight_style(Style::default().bold().fg(Color::Cyan));
        // ... render active tab content
    }
}

// SSE client feeding pipeline events into the TUI
async fn sse_listener(url: &str, tx: mpsc::Sender<PipelineEvent>) {
    let client = reqwest::Client::new();
    let mut stream = client.get(url).send().await.unwrap().bytes_stream();
    while let Some(chunk) = stream.next().await {
        if let Ok(event) = parse_sse_event(&chunk.unwrap()) {
            let _ = tx.send(event).await;
        }
    }
}
```

---

## 2. Web Dashboard

**Priority: P0** | **Complexity: High (4-6 weeks)** | **Stack: Next.js + React**

### User Stories

- As a contributor, I want a web-based trace browser so I can explore my
  submission history and quality metrics from any device.
- As an operator, I want an admin panel showing system health, TEE status,
  and scoring pipeline throughput so I can monitor production.
- As a potential contributor, I want a public landing page explaining the
  value proposition so I can decide whether to participate.

### Technology Rationale

- **Next.js 14+ (App Router)** -- SSR for the landing page (SEO), client
  components for the interactive dashboard. The team already needs a JS/TS
  surface for embeddable provenance cards (Section 4).
- **Tailwind CSS** -- utility-first, fast iteration, consistent with
  developer tooling aesthetics.
- **SWR or TanStack Query** -- cache management for API polling and SSE
  integration.
- **Alternative considered: Leptos (Rust WASM)** -- compelling for type
  safety with the protocol crate, but the ecosystem is less mature for
  complex dashboards. Recommended as a P2 rewrite target once the feature
  set stabilizes.

### Page Architecture

```
/                           Landing page (public, SSR)
/login                      OAuth / invite-code enrollment
/dashboard                  Contributor summary (authenticated)
/dashboard/traces           Trace list with filters
/dashboard/traces/:id       Trace detail view
/dashboard/quality          Quality analytics charts
/dashboard/credits          Credit balance and transactions
/dashboard/profile          Public handle, bio, badges
/admin                      Operator panel (role-gated)
/admin/health               System health dashboard
/admin/scoring              Scoring pipeline metrics
/admin/tee                  TEE attestation status
/leaderboard                Public contributor rankings (opt-in)
```

### Dashboard Page Wireframe

```
+-- TraceCommons Dashboard -------------------------------------------+
|  [logo]  Dashboard  Traces  Quality  Credits  Profile    [user] [v] |
+---------------------------------------------------------------------+
|                                                                     |
|  +-- Summary Cards -----------------------------------------------+ |
|  | +----------+ +----------+ +----------+ +----------+            | |
|  | | 142      | | 97.2%    | | 0.63     | | 847      |            | |
|  | | Traces   | | Accept   | | Avg      | | Credits  |            | |
|  | | Submitted| | Rate     | | Quality  | | Earned   |            | |
|  | | +12 today| | +0.3%    | | +0.04    | | +38.2    |            | |
|  | +----------+ +----------+ +----------+ +----------+            | |
|  +-----------------------------------------------------------------+ |
|                                                                     |
|  +-- Quality Trend (30d) ---+ +-- Source Breakdown ---------------+ |
|  |                          | |                                   | |
|  |    0.8 __                | |  claude-code  ========== 78%      | |
|  |    0.6   \__    __/      | |  codex        ===        15%      | |
|  |    0.4      \__/         | |  trajectory   =          7%       | |
|  |    0.2                   | |                                   | |
|  |    Jul  Jul  Aug  Aug    | |                                   | |
|  +---------------------------+ +-----------------------------------+ |
|                                                                     |
|  +-- Recent Submissions ------------------------------------------+ |
|  |  Source       Project          Time     Status    Quality  Cr   | |
|  |  claude-code  myproject        2m ago   accepted  0.71   5.2   | |
|  |  codex        api-refactor     3m ago   accepted  0.68   4.8   | |
|  |  claude-code  myproject        5m ago   accepted  0.45   2.1   | |
|  |  claude-code  test-fixtures    7m ago   quarant.  --     0.0   | |
|  |  [View all traces ->]                                          | |
|  +-----------------------------------------------------------------+ |
+---------------------------------------------------------------------+
```

### Trace Detail View Wireframe

```
+-- Trace Detail: sub-a8f3...7d21 ------------------------------------+
|  [< Back to Traces]                                                  |
|                                                                      |
|  +-- Provenance --------------------------------------------------+ |
|  |  Submission ID:  a8f3c2d1-...                                  | |
|  |  Trace ID:       e7a41b9f-...                                  | |
|  |  Source:         claude-code                                    | |
|  |  Project:        myproject (basename only)                      | |
|  |  Model:          claude-sonnet-4-20250514                           | |
|  |  Submitted:      2026-08-10 14:32:01 UTC                       | |
|  |  Consent:        debugging_evaluation, model_training           | |
|  |  Redaction:      ironclaw-deterministic-secret-path-v3          | |
|  |  PII Risk:       low                                            | |
|  +------------------------------------------------------------------+|
|                                                                      |
|  +-- Quality Scores --+ +-- Value Scorecard ----------------------+ |
|  |  Perplexity (rep)  | |  Schema Validity   1.00                 | |
|  |    14.2 micros     | |  Privacy Risk      0.95                 | |
|  |    [====>    ]     | |  Quality           0.71                 | |
|  |  Perplexity (peak) | |  Replayability     0.80                 | |
|  |    18.7 micros     | |  Novelty           0.68                 | |
|  |  Novelty           | |  Duplicate Penalty  0.00                | |
|  |    0.72            | |  Coverage Bonus    0.05                 | |
|  |    [======>  ]     | |  Difficulty        0.60                 | |
|  |  Anomaly Ratio     | |  Online Score      0.71                 | |
|  |    1.3 (normal)    | |  Credit Estimate   5.2 cr               | |
|  |  Gate: PASSED      | |                                         | |
|  +---------------------+ +-----------------------------------------+ |
|                                                                      |
|  +-- Events (12 events) -----------------------------------------+  |
|  |  #  Type              Tool           Latency  Tokens           |  |
|  |  1  user_message      --             --       --               |  |
|  |  2  assistant_message  --            320ms    1,204/489        |  |
|  |  3  tool_call          Read          --       --               |  |
|  |  4  tool_result        Read          45ms     --               |  |
|  |  5  assistant_message  --            890ms    2,108/1,203      |  |
|  |  6  tool_call          Edit          --       --               |  |
|  |  7  tool_result        Edit          12ms     --               |  |
|  |  ...                                                           |  |
|  |  [Click row for redacted content and structured payload]       |  |
|  +-----------------------------------------------------------------+ |
+----------------------------------------------------------------------+
```

### Admin Panel Wireframe

```
+-- Admin: System Health -------------------------------------------------+
|  [Health] [Scoring Pipeline] [TEE] [Contributors] [Corpus] [Config]    |
+-------------------------------------------------------------------------+
|                                                                         |
|  +-- System Status -------------------------------------------------+  |
|  |  Ingest API:        HEALTHY  (p99 latency: 45ms, 2.3K req/hr)   |  |
|  |  Upload Claim Issuer: HEALTHY  (key: attestation-key-1, 24h TTL) |  |
|  |  Vector Worker:     HEALTHY  (queue depth: 12, avg: 1.2s/trace)  |  |
|  |  TEE Enclave:       HEALTHY  (last attestation: 3m ago)          |  |
|  |  PostgreSQL:        HEALTHY  (connections: 14/100, RLS enforced)  |  |
|  |  GCS Artifact Store: HEALTHY  (KEK rotation: current)            |  |
|  +-------------------------------------------------------------------+  |
|                                                                         |
|  +-- Scoring Pipeline (last 1h) ------------------------------------+  |
|  |  Traces Ingested:    142     Scored:      138     Failed: 4      |  |
|  |  Avg Score Time:     1.2s    P99 Score:   4.8s    Queue:  12     |  |
|  |  Perplexity Floor:   6.0M    Novelty Floor: 500K                 |  |
|  |  Gate Pass Rate:     89.2%   Anomaly Withheld: 2                 |  |
|  +-------------------------------------------------------------------+  |
|                                                                         |
|  +-- Credit Quality Distribution -----------------------------------+  |
|  |  Version: V2 (graded affine floors)                              |  |
|  |  Constants: ppl_floor=6.0  ppl_ceil=38.5  nov_floor=0.5          |  |
|  |             nov_ceil=1.0   ppl_floor_mult=0.25  nov_floor_mult=0.30|  |
|  |  Histogram:                                                      |  |
|  |    0.0-0.1  ##                    (4%)                           |  |
|  |    0.1-0.2  ###                   (6%)                           |  |
|  |    0.2-0.3  #####                 (10%)                          |  |
|  |    0.3-0.4  ########              (16%)                          |  |
|  |    0.4-0.5  ##########            (20%)                          |  |
|  |    0.5-0.6  ########              (16%)                          |  |
|  |    0.6-0.7  ######                (12%)                          |  |
|  |    0.7-0.8  ####                  (8%)                           |  |
|  |    0.8-0.9  ##                    (4%)                           |  |
|  |    0.9-1.0  ##                    (4%)                           |  |
|  +-------------------------------------------------------------------+  |
+-------------------------------------------------------------------------+
```

### React Component Sketch

```tsx
// components/TraceSummaryCards.tsx
interface SummaryCardsProps {
  submitted: number;
  acceptRate: number;
  avgQuality: number;
  credits: number;
}

function TraceSummaryCards({ submitted, acceptRate, avgQuality, credits }: SummaryCardsProps) {
  return (
    <div className="grid grid-cols-4 gap-4">
      <MetricCard label="Traces Submitted" value={submitted} trend="+12 today" />
      <MetricCard label="Acceptance Rate" value={`${(acceptRate * 100).toFixed(1)}%`} trend="+0.3%" />
      <MetricCard label="Avg Quality" value={avgQuality.toFixed(2)} trend="+0.04" />
      <MetricCard label="Credits Earned" value={credits.toFixed(1)} trend="+38.2" />
    </div>
  );
}

// hooks/useSSE.ts -- real-time event stream
function useSSE(url: string) {
  const [events, setEvents] = useState<PipelineEvent[]>([]);

  useEffect(() => {
    const source = new EventSource(url);
    source.addEventListener('score_computed', (e) => {
      const data = JSON.parse(e.data);
      setEvents(prev => [data, ...prev].slice(0, 100));
    });
    return () => source.close();
  }, [url]);

  return events;
}
```

---

## 3. Progressive Disclosure

**Priority: P0** | **Complexity: Low (design pattern, not standalone feature)**

### Design Principle

Every data surface in TraceCommons follows a three-level disclosure pattern:
summary first, breakdown on demand, raw data at the leaf. This prevents
information overload while allowing deep investigation.

### Disclosure Chains

**Trace list to raw data:**
```
Trace List (table of submissions with one-line status)
  -> Trace Detail (provenance, scores, event timeline)
    -> Event Drill-Down (specific tool call, redacted content)
      -> Structured Payload (raw JSON, token counts, latency)
```

**Quality score to model details:**
```
Quality Score (single number, 0.71)
  -> Score Breakdown (perplexity term, novelty term, anomaly factor)
    -> Per-Rung Scores (V2 graded floor values, raw micros)
      -> Scoring Model Details (calibration constants, version, formula)
```

**Credit balance to calculation:**
```
Credit Balance (847.3 cr)
  -> Transaction History (per-submission credit awards)
    -> Credit Calculation Detail (q_micros, base_rate, multipliers)
      -> Credit Quality Constants (V2 calibration, floor/ceil values)
```

### Implementation Pattern

The disclosure pattern maps directly to URL structure on the web and to
panel stacking in the TUI:

```
Web:  /traces         -> /traces/:id     -> /traces/:id/events/:eid
TUI:  F2 trace list   -> [Enter] detail  -> [Enter] event drill-down
API:  GET /v1/submissions (list)
      GET /v1/submissions/:id (detail)
      GET /v1/submissions/:id/events (events)
```

Each level adds a "back" affordance and a breadcrumb trail. The TUI uses a
panel stack that pushes and pops with Enter/Escape. The web uses standard
browser navigation with Next.js dynamic routes.

### Component Pattern (Web)

```tsx
// Disclosure wrapper used across all drill-down surfaces
interface DisclosureProps {
  summary: React.ReactNode;
  detail: React.ReactNode;
  expanded?: boolean;
}

function Disclosure({ summary, detail, expanded = false }: DisclosureProps) {
  const [open, setOpen] = useState(expanded);
  return (
    <div className="border rounded-lg">
      <button
        className="w-full text-left p-4 flex justify-between"
        onClick={() => setOpen(!open)}
      >
        {summary}
        <ChevronIcon direction={open ? 'up' : 'down'} />
      </button>
      {open && <div className="p-4 border-t bg-gray-50">{detail}</div>}
    </div>
  );
}

// Usage: quality score disclosure
<Disclosure
  summary={<span>Quality Score: <strong>0.71</strong></span>}
  detail={
    <div>
      <ScoreBar label="Perplexity (f)" value={0.82} />
      <ScoreBar label="Novelty (g)" value={0.87} />
      <ScoreBar label="Anomaly (a)" value={1.0} note="ratio 1.3, no penalty" />
      <p className="text-sm text-gray-500 mt-2">
        q = f * g * a = 0.82 * 0.87 * 1.0 = 0.71
      </p>
    </div>
  }
/>
```

---

## 4. Provenance Cards

**Priority: P1** | **Complexity: Medium (1-2 weeks)** | **Endpoint: `GET /api/v1/traces/{id}/card.svg`**

### User Stories

- As a contributor, I want an embeddable badge showing my trace quality so I
  can display it in my GitHub profile or portfolio.
- As a collector, I want to verify a trace's provenance chain without
  querying the full API.

### Card Design

```
+-- Provenance Card ----------------------------------------+
|                                                           |
|  TRACECOMMONS                                             |
|  =========================================================|
|                                                           |
|  Contributor:  zaki-dev                                   |
|  Submitted:    2026-08-10 14:32 UTC                       |
|  Source:       claude-code / claude-sonnet-4-20250514          |
|  Quality:      0.71  [======>    ]  GOLD                  |
|  Gate:         PASSED                                     |
|  TEE Attestation: sha256:e4a1...9f12                      |
|                                                           |
|  Consent: debugging_evaluation, model_training            |
|  Revocable: yes                                           |
|                                                           |
+-----------------------------------------------------------+
```

### Quality Tier Badges

Tiers are derived from the credit quality score:

| Tier | Score Range | Color | Label |
|---|---|---|---|
| Diamond | q >= 0.90 | `#b9f2ff` | Exceptional trace |
| Gold | 0.70 <= q < 0.90 | `#ffd700` | High-quality trace |
| Silver | 0.50 <= q < 0.70 | `#c0c0c0` | Solid trace |
| Bronze | 0.25 <= q < 0.50 | `#cd7f32` | Acceptable trace |
| -- | q < 0.25 | `#888888` | No badge (below threshold) |

### SVG Generation (Rust)

```rust
// trace-commons-server: card endpoint handler
async fn trace_card_handler(
    Path(submission_id): Path<Uuid>,
    State(app): State<Arc<AppState>>,
) -> impl IntoResponse {
    let trace = app.db.get_submission_with_scores(submission_id).await?;
    let contributor_handle = app.db.get_contributor_handle(&trace.principal_ref).await?;

    let tier = quality_tier(trace.credit_quality_micros);
    let svg = render_provenance_card(&ProvenanceCardData {
        handle: contributor_handle.as_deref().unwrap_or("anonymous"),
        submitted_at: trace.submitted_at,
        source: &trace.source_channel,
        model: trace.model_name.as_deref(),
        quality_score: trace.credit_quality_micros as f64 / 1_000_000.0,
        tier,
        gate_passed: trace.gate_passed,
        attestation_hash: &trace.attestation_chain_hash,
        consent_scopes: &trace.consent_scopes,
        revocable: trace.revocable,
    });

    (
        [(header::CONTENT_TYPE, "image/svg+xml"),
         (header::CACHE_CONTROL, "public, max-age=3600")],
        svg,
    )
}

fn quality_tier(q_micros: i64) -> &'static str {
    match q_micros {
        q if q >= 900_000 => "diamond",
        q if q >= 700_000 => "gold",
        q if q >= 500_000 => "silver",
        q if q >= 250_000 => "bronze",
        _ => "none",
    }
}
```

### Embeddable Badge (compact form)

For README embedding, a compact shield-style badge:

```
+-----------------------------------------------+
| TraceCommons | Quality: 0.71 | GOLD           |
+-----------------------------------------------+
```

Endpoint: `GET /api/v1/traces/{id}/badge.svg`

Usage in markdown:
```markdown
![TraceCommons](https://api.tracecommons.ai/v1/traces/a8f3c2d1.../badge.svg)
```

---

## 5. SSE Real-Time Dashboard

**Priority: P0** | **Complexity: Medium (1-2 weeks)** | **Endpoint: `GET /api/v1/events/stream`**

### User Stories

- As a contributor running `submit --watch`, I want to see scoring results
  appear live without polling.
- As a TUI user, I want the dashboard to update in real time as my traces
  move through the scoring pipeline.
- As a web dashboard user, I want the "Recent Activity" panel to update
  without page refreshes.

### Event Types

| Event | Payload | When |
|---|---|---|
| `trace_submitted` | `{ submission_id, source, project_basename, timestamp }` | Envelope accepted by ingest |
| `scoring_started` | `{ submission_id, queue_position, estimated_latency_ms }` | Worker picks up trace |
| `score_computed` | `{ submission_id, perplexity_micros, novelty_micros, gate_passed, q_micros }` | Gate decision recorded |
| `credit_awarded` | `{ submission_id, credits, balance, tier }` | Credit ledger updated |
| `gate_failed` | `{ submission_id, reason, perplexity_passed, novelty_passed }` | Gate rejection |
| `quarantine` | `{ submission_id, reason_label }` | Envelope quarantined |
| `corpus_stats` | `{ total_traces, total_contributors, avg_quality, corpus_size_bytes }` | Periodic (every 60s) |

### axum SSE Handler (Rust)

```rust
use axum::response::sse::{Event, KeepAlive, Sse};
use tokio::sync::broadcast;
use futures::stream::Stream;

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "data")]
enum PipelineEvent {
    TraceSubmitted { submission_id: Uuid, source: String, project: Option<String> },
    ScoringStarted { submission_id: Uuid, queue_position: u32 },
    ScoreComputed {
        submission_id: Uuid,
        perplexity_micros: u64,
        novelty_micros: u64,
        gate_passed: bool,
        q_micros: i64,
    },
    CreditAwarded { submission_id: Uuid, credits: f64, balance: f64, tier: String },
    GateFailed { submission_id: Uuid, reason: String },
}

async fn events_stream(
    State(app): State<Arc<AppState>>,
    auth: AuthenticatedContributor,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = app.event_bus.subscribe();
    let principal_ref = auth.principal_ref.clone();

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(event) if event.belongs_to(&principal_ref) => {
                    let data = serde_json::to_string(&event).unwrap();
                    yield Ok(Event::default()
                        .event(event.event_type())
                        .data(data));
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    yield Ok(Event::default()
                        .event("lagged")
                        .data(format!("{{\"skipped\": {n}}}")));
                }
                _ => continue,
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}
```

### Event Bus Integration

The event bus is a `tokio::sync::broadcast::Sender<PipelineEvent>` held in
`AppState`. Each stage of the pipeline emits events:

```
Ingest handler        -> TraceSubmitted
Vector worker pickup  -> ScoringStarted
Gate service evaluate -> ScoreComputed / GateFailed
Credit ledger write   -> CreditAwarded
Quarantine handler    -> Quarantine
```

The SSE handler filters events by the authenticated contributor's
`principal_ref`, so each contributor only sees their own traces. The admin
SSE endpoint (`/api/v1/admin/events/stream`) shows all events, gated by
operator credentials.

### Client-Side Integration

```typescript
// Shared SSE client for both web dashboard and CLI --watch mode
class TraceCommonsEventSource {
  private source: EventSource;
  private handlers: Map<string, (data: any) => void> = new Map();

  constructor(baseUrl: string, authToken: string) {
    this.source = new EventSource(
      `${baseUrl}/api/v1/events/stream`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );

    for (const eventType of [
      'trace_submitted', 'scoring_started', 'score_computed',
      'credit_awarded', 'gate_failed', 'quarantine'
    ]) {
      this.source.addEventListener(eventType, (e: MessageEvent) => {
        const handler = this.handlers.get(eventType);
        if (handler) handler(JSON.parse(e.data));
      });
    }
  }

  on(event: string, handler: (data: any) => void) {
    this.handlers.set(event, handler);
  }

  close() { this.source.close(); }
}
```

---

## 6. PWA / Mobile

**Priority: P2** | **Complexity: High (3-4 weeks)**

### User Stories

- As a mobile contributor, I want push notifications when my traces are
  scored so I can track my submissions without being at my desk.
- As a contributor with intermittent connectivity, I want to queue trace
  submissions offline and have them sync when I reconnect.

### Design

The web dashboard (Section 2) is built as a Progressive Web App from the
start, with the following mobile-specific features:

**Push Notifications:**
```
[TraceCommons] Trace scored: 0.71 (Gold) -- +5.2 credits
[TraceCommons] Daily summary: 12 traces, avg quality 0.65, +42.3 credits
[TraceCommons] Quality threshold changed: novelty floor raised to 0.55
```

**Offline Queue:**
When `submit` is triggered without connectivity, the envelope is written to
IndexedDB and a service worker schedules a Background Sync:

```typescript
// service-worker.ts
self.addEventListener('sync', (event: SyncEvent) => {
  if (event.tag === 'trace-submit') {
    event.waitUntil(submitQueuedTraces());
  }
});

async function submitQueuedTraces() {
  const db = await openDB('tc-offline', 1);
  const queued = await db.getAll('pending-envelopes');
  for (const envelope of queued) {
    const res = await fetch('/api/v1/submit', {
      method: 'POST',
      body: JSON.stringify(envelope),
      headers: { 'Content-Type': 'application/json' },
    });
    if (res.ok) {
      await db.delete('pending-envelopes', envelope.id);
    }
  }
}
```

**Responsive Layout:**
The dashboard grid collapses to a single column on mobile. Summary cards
stack vertically. The trace list becomes a card-based layout with swipe
gestures for detail navigation.

### PWA Manifest

```json
{
  "name": "TraceCommons",
  "short_name": "TC",
  "start_url": "/dashboard",
  "display": "standalone",
  "background_color": "#0f172a",
  "theme_color": "#06b6d4",
  "icons": [
    { "src": "/icon-192.png", "sizes": "192x192", "type": "image/png" },
    { "src": "/icon-512.png", "sizes": "512x512", "type": "image/png" }
  ]
}
```

---

## 7. Onboarding Flow

**Priority: P0** | **Complexity: Medium (1-2 weeks)**

### User Stories

- As a new contributor, I want a guided setup process so I can go from zero
  to first submission in under five minutes.
- As a contributor who just enrolled, I want to see my first trace scored so
  I understand the value proposition immediately.

### CLI Wizard: `tc-contributor init`

The existing `login` subcommand handles enrollment, but it assumes the user
already has an invite link or grant. The `init` wizard provides a complete
guided experience:

```
$ trace-commons-contributor init

  Welcome to TraceCommons!
  ========================

  TraceCommons collects coding agent traces to improve AI assistants.
  Your traces are redacted locally, scored for quality, and earn credits.

  Step 1 of 5: Create Identity
  ----------------------------
  Generating Ed25519 keypair...
  Device Key ID: sha256:ad745f4e...c177bd83
  Stored at: ~/.config/trace-commons/device.pk8 (mode 0600)

  Step 2 of 5: Connect to TraceCommons
  -------------------------------------
  Enter your invite link (from your TraceCommons invitation email):
  > https://issuer.tracecommons.ai/onboard#INV9K3RT5FBQ72JX

  Enrolling device...
  Tenant:   tenant-7a3b...e912
  Issuer:   https://issuer.tracecommons.ai
  Ingest:   https://ingest.tracecommons.ai

  Step 3 of 5: Configure Consent
  ------------------------------
  How may your submitted traces be used?
    Debugging and evaluation                 [always on]
    Benchmark generation                     [y/N] y
    Ranking-model training                   [y/N] n
    Model training                           [y/N] y
    Public attribution of your handle        [y/N] n

  Scopes: debugging_evaluation, benchmark_only, model_training

  Step 4 of 5: Discover Traces
  ----------------------------
  Scanning for coding agent sessions...

    Source       Sessions  Newest        Oldest
    -----------  --------  -----------   -----------
    claude-code  47        2 minutes ago 14 days ago
    codex        12        1 hour ago    7 days ago

  59 sessions found.

  Step 5 of 5: Submit First Trace
  --------------------------------
  Submit your most recent session? [Y/n] y

  Redacting session (ironclaw-deterministic-secret-path-v3)...
    Redaction counts: { "api_key": 2, "path": 14, "email": 1 }
    PII risk: low
    Envelope size: 48.2 KB (limit: 1 MB)

  Uploading...
  Submission ID: a8f3c2d1-7d21-5e9a-b4f8-3c1e9d2a7b56
  Status: accepted

  Waiting for scoring results...
  [=====>              ] scoring in progress...

  Score computed!
    Perplexity (rep): 14.2   (floor: 6.0, ceiling: 38.5)
    Novelty:          0.72   (floor: 0.5, ceiling: 1.0)
    Quality (q):      0.71   Tier: GOLD
    Credits earned:   5.2

  Setup complete! Run `trace-commons-contributor dashboard` to watch live.
```

### Web Onboarding Flow

The web onboarding mirrors the CLI wizard but uses a step-by-step card UI:

```
+-- Onboarding --------------------------------------------------------+
|                                                                       |
|  Step 1 of 5                  [*]--[*]--[*]--[ ]--[ ]                 |
|                                                                       |
|  Configure Your Agent Integration                                     |
|  ================================                                     |
|                                                                       |
|  Which coding agent do you use?                                       |
|                                                                       |
|  +-- [x] Claude Code ---------+  +-- [ ] Codex ------------------+   |
|  |  ~/.claude/projects         |  |  ~/.codex/sessions            |   |
|  |  47 sessions found          |  |  12 sessions found            |   |
|  +-----------------------------+  +-------------------------------+   |
|                                                                       |
|  +-- [ ] Trajectory Files ----+                                       |
|  |  Drag & drop or browse     |                                       |
|  |  for trajectory-v1 files   |                                       |
|  +-----------------------------+                                      |
|                                                                       |
|                                     [Back]  [Continue ->]             |
+-----------------------------------------------------------------------+
```

---

## 8. API Documentation

**Priority: P1** | **Complexity: Low (1 week)**

### User Stories

- As an integration developer, I want interactive API docs so I can test
  endpoints without writing code first.
- As a contributor building automation, I want code examples in my language
  so I can integrate programmatically.

### OpenAPI Spec Generation

The ingest server already uses axum handlers with typed extractors. The spec
is generated using `utoipa` (Rust OpenAPI generator that integrates with
axum) and served at `/api/docs`:

```rust
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    paths(
        submit_handler,
        status_handler,
        score_attestation_handler,
        events_stream_handler,
        trace_card_handler,
        community_profile_handler,
    ),
    components(schemas(
        TraceContributionEnvelope,
        TraceSubmissionReceipt,
        ScoreAttestationClaims,
        CommunityProfile,
        PipelineEvent,
    )),
    tags(
        (name = "submission", description = "Trace submission and status"),
        (name = "scoring", description = "Quality scoring and attestation"),
        (name = "community", description = "Public profiles and leaderboard"),
        (name = "events", description = "Real-time event stream"),
    )
)]
struct ApiDoc;

fn api_docs_routes() -> Router {
    Router::new()
        .merge(SwaggerUi::new("/api/docs").url("/api/openapi.json", ApiDoc::openapi()))
}
```

### Code Examples

**Rust (using trace-commons-operator-client):**
```rust
use trace_commons_operator_client::Client;

let client = Client::new("https://ingest.tracecommons.ai", auth_token)?;
let status = client.get("/v1/submissions/status").await?;
println!("Submissions: {:?}", status);
```

**Python:**
```python
import requests

headers = {"Authorization": f"Bearer {token}"}
r = requests.get("https://ingest.tracecommons.ai/v1/submissions/status", headers=headers)
for sub in r.json()["submissions"]:
    print(f"{sub['submission_id']}: {sub['status']} q={sub.get('credit_quality_micros', 'pending')}")
```

**TypeScript:**
```typescript
const res = await fetch('https://ingest.tracecommons.ai/v1/submissions/status', {
  headers: { Authorization: `Bearer ${token}` },
});
const { submissions } = await res.json();
submissions.forEach(s =>
  console.log(`${s.submission_id}: ${s.status} q=${s.credit_quality_micros ?? 'pending'}`)
);
```

**curl:**
```bash
curl -H "Authorization: Bearer $TOKEN" \
  https://ingest.tracecommons.ai/v1/submissions/status | jq .
```

### Authentication Guide

TraceCommons uses a two-phase auth flow:

1. **Device enrollment** -- Ed25519 keypair generated locally, registered via
   invite link or instance grant. Produces a `device_key_id` bound to a
   tenant.
2. **Upload claims** -- short-lived JWTs (EdDSA-signed) obtained from the
   upload-claim issuer before each submission batch. The claim is scoped to
   the device's tenant and consent scopes.

The API docs include a "Try It" flow that accepts a pre-minted upload claim
for testing, with clear warnings that claims are short-lived (default 5s
TTL) and should not be shared.

### Rate Limiting

| Endpoint | Limit | Window |
|---|---|---|
| `POST /v1/submit` | 60 req/min | Per device_key_id |
| `GET /v1/submissions/status` | 120 req/min | Per device_key_id |
| `GET /v1/events/stream` | 5 concurrent | Per device_key_id |
| `GET /v1/community/leaderboard` | 30 req/min | Per IP |

Rate limit headers follow the `RateLimit-*` draft spec:
```
RateLimit-Limit: 60
RateLimit-Remaining: 58
RateLimit-Reset: 1723295520
```

---

## 9. Quality Visualization

**Priority: P1** | **Complexity: Medium (2-3 weeks)**

### User Stories

- As a contributor, I want to see how my trace quality compares to the
  corpus so I know whether my submissions are above or below average.
- As an operator, I want to see corpus coverage heatmaps so I can identify
  underrepresented domains and incentivize contributions.
- As an operator, I want to track scoring latency and queue depth so I can
  plan capacity.

### Visualization Catalog

#### 1. Quality Score Distribution Histogram

Shows the distribution of `q_micros` across all scored traces, with the
contributor's own scores highlighted.

```
Web implementation:  Observable Plot (lightweight, declarative, D3-based)
TUI implementation:  ratatui BarChart widget with custom bin labels
```

```
                     Corpus Quality Distribution (n=28,412)
  Count
   800 |              +--+
   700 |           +--+  +--+
   600 |        +--+  |  |  +--+
   500 |     +--+  |  |  |  |  +--+
   400 |  +--+  |  |  |  |  |  |  +--+
   300 |  |  |  |  |  |  |  |  |  |  +--+
   200 |  |  |  |  |  |  |  |  |  |  |  +--+
   100 |  |  |  |  |  |  |  |  |  |  |  |  |
       +--+--+--+--+--+--+--+--+--+--+--+--+--
        0.0 0.1 0.2 0.3 0.4 0.5 0.6 0.7 0.8 0.9 1.0
                           Quality Score (q)

   Your traces: [marked with * overlay on matching bins]
```

#### 2. Perplexity vs. Novelty Scatter Plot

Each trace as a dot in (perplexity, novelty) space. Color encodes quality
tier. Shaded regions show the gate pass/fail boundaries.

```
  Novelty
   1.0 |  -------- ceiling --------  .  .  .  * .
       |                          .  .  *  .  .
   0.8 |                       .  .  .  .  *  .
       |                    .  .  .  .  .  .
   0.6 |                 .  .  .  .  .  .  x
       |  floor --+   .  .  .  .  .  .
   0.5 | ---------|-.-.-.-.-.-.-.-.-.-.-.-.----
   0.4 |          |  x  x        x
       |          |     x
   0.2 |          |
       |          |
       +----|-----+-------------------------------
            6.0                           38.5
                     Perplexity (rep)

   . = passed gate    x = failed gate    * = your traces
   Floor and ceiling lines mark the V2 calibration constants
```

#### 3. Novelty Score Time Series

Tracks per-submission novelty over time, showing whether the corpus is
growing stale (novelty trending down) or receiving diverse contributions.

#### 4. Corpus Coverage Heatmap

A grid showing which tool categories and source channels are well-covered:

```
                   claude-code  codex  trajectory  total
  Read/Write         ████████    ███    ██         13.2K
  Bash/Shell         ██████      ██     █          8.4K
  Search/Grep        █████       ██     ██         7.1K
  Edit               ████        █      █          4.8K
  Web/API            ██          █      ██         3.1K
  Git                ██          █               2.0K
  Notebook                              ██         0.8K

  ████ = well-covered (>1000 traces)
  ██   = moderate (100-1000)
  █    = sparse (<100, needs more)
```

#### 5. System Health Dashboard (Operator)

```
  Scoring Latency (p50 / p90 / p99)
  1.2s / 3.4s / 8.1s

  Queue Depth (last 24h)
   50 |     __
   40 |    /  \
   30 |   /    \___
   20 |  /         \____
   10 | /                \_______
    0 +--+--+--+--+--+--+--+--+--
       00  03  06  09  12  15  18  21

  TEE Uptime: 99.97% (last 30d)
  Last Attestation: 3 minutes ago
  Attestation Key: attestation-key-1 (TTL: 24h)
```

### Observable Plot Implementation (Web)

```typescript
import * as Plot from "@observablehq/plot";

function QualityDistribution({ traces, myTraces }: Props) {
  return Plot.plot({
    marks: [
      Plot.rectY(traces, Plot.binX({ y: "count" }, {
        x: "quality_score",
        fill: "#94a3b8",
        thresholds: 20,
      })),
      Plot.rectY(myTraces, Plot.binX({ y: "count" }, {
        x: "quality_score",
        fill: "#06b6d4",
        thresholds: 20,
      })),
      Plot.ruleX([0.25, 0.50, 0.70, 0.90], {
        stroke: "#e2e8f0",
        strokeDasharray: "4,4",
      }),
    ],
    x: { label: "Quality Score (q)", domain: [0, 1] },
    y: { label: "Count" },
    width: 600,
    height: 300,
  });
}
```

### ratatui Implementation (TUI)

```rust
use ratatui::widgets::{BarChart, Block, Borders};

fn render_quality_histogram(frame: &mut Frame, area: Rect, scores: &[f64]) {
    let bins = histogram_bins(scores, 10, 0.0, 1.0);
    let data: Vec<(&str, u64)> = bins.iter()
        .map(|(label, count)| (label.as_str(), *count))
        .collect();

    let chart = BarChart::default()
        .block(Block::default().title("Quality Distribution").borders(Borders::ALL))
        .bar_width(5)
        .bar_gap(1)
        .bar_style(Style::default().fg(Color::Cyan))
        .value_style(Style::default().fg(Color::White))
        .data(&data);

    frame.render_widget(chart, area);
}
```

---

## 10. CLI UX Improvements

**Priority: P0** | **Complexity: Low-Medium (1-2 weeks)**

### User Stories

- As a contributor, I want colored output and progress bars so the CLI
  feels responsive during long operations.
- As a CI integrator, I want `--format json` output so I can parse
  submission results programmatically.
- As a new user, I want shell completions so I can discover subcommands
  via tab completion.

### Colored Output with Progress Bars

The existing CLI prints plain text. Adding `indicatif` for progress bars
and `console` or `colored` for styled output:

```
$ trace-commons-contributor submit --all --since 1d

  Discovering sessions...
  Found 12 sessions (9 claude-code, 3 codex)

  Redacting and submitting:
    [1/12] claude-code  myproject       [################] 100%  accepted  0.71
    [2/12] codex        api-refactor    [################] 100%  accepted  0.68
    [3/12] claude-code  myproject       [################] 100%  accepted  0.45
    [4/12] claude-code  test-fixtures   [################] 100%  quarantine
    [5/12] claude-code  utils           [########        ]  50%  uploading...

  Summary:
    Submitted: 11    Quarantined: 1    Failed: 0
    Credits earned: +42.3 (balance: 847.3)
    Average quality: 0.63
```

### Structured Output

Every subcommand gains `--format` support:

```bash
# Default: human-readable table
$ trace-commons-contributor status
  SUBMISSION_ID                          STATUS     QUALITY  CREDITS
  a8f3c2d1-7d21-5e9a-b4f8-3c1e9d2a7b56  accepted   0.71     5.2
  c4e17a29-9a03-5f1b-a2d6-8e4c3b7f1a95  accepted   0.68     4.8

# JSON for scripting (already partially supported via --json)
$ trace-commons-contributor status --format json
  [{"submission_id":"a8f3c2d1-...","status":"accepted","quality":0.71,"credits":5.2},...]

# CSV for spreadsheets
$ trace-commons-contributor status --format csv
  submission_id,status,quality,credits
  a8f3c2d1-7d21-5e9a-b4f8-3c1e9d2a7b56,accepted,0.71,5.2
```

Implementation leverages the existing `--json` flag (already wired) and
extends it to a `--format` enum:

```rust
#[derive(Clone, Copy, ValueEnum)]
enum OutputFormat {
    Table,  // default, human-readable
    Json,   // machine-readable JSON
    Csv,    // comma-separated values
}

// Backward compat: --json is equivalent to --format json
#[arg(long, global = true)]
json: bool,
#[arg(long, global = true, default_value = "table")]
format: OutputFormat,
```

### Shell Completions

Using `clap_complete` (clap already in use):

```rust
// trace-commons-contributor completions <shell>
use clap_complete::{generate, Shell};

fn completions(shell: Shell) {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "trace-commons-contributor", &mut std::io::stdout());
}
```

```bash
# Install completions
$ trace-commons-contributor completions zsh > ~/.zfunc/_trace-commons-contributor
$ trace-commons-contributor completions bash > /etc/bash_completion.d/trace-commons-contributor
$ trace-commons-contributor completions fish > ~/.config/fish/completions/trace-commons-contributor.fish
```

### `tc status` One-Liner Health Check

A quick health check combining local state and server connectivity:

```
$ trace-commons-contributor status --health

  Identity:    sha256:ad74...bd83 (enrolled 2026-07-15)
  Issuer:      https://issuer.tracecommons.ai      OK (12ms)
  Ingest:      https://ingest.tracecommons.ai      OK (45ms)
  Local State: ~/.config/trace-commons/             OK (3 files, 0600)
  Receipts:    142 submissions (138 accepted, 4 quarantined)
  PII Filter:  near-ai (API key set)                OK
  Sources:     claude-code (47 sessions), codex (12 sessions)
```

### `tc submit --watch` for Live Scoring Feedback

Extends the `submit` subcommand with `--watch` to wait for scoring results
via SSE after upload:

```
$ trace-commons-contributor submit --all --since 1h --watch

  Submitting 5 sessions...
  [1/5] claude-code myproject  uploaded   waiting for score...
  [2/5] codex api-refactor     uploaded   waiting for score...
  [3/5] claude-code myproject  uploaded   waiting for score...

  Live scoring results (Ctrl+C to stop watching):
    14:32:03  sub-a8f3...  scored  q=0.71  GOLD     +5.2 cr
    14:32:05  sub-c4e1...  scored  q=0.68  SILVER   +4.8 cr
    14:32:08  sub-2b7a...  scored  q=0.45  BRONZE   +2.1 cr
    14:32:09  sub-e91d...  scored  q=0.82  GOLD     +7.1 cr
    14:32:11  sub-f3a2...  scored  q=0.55  SILVER   +3.4 cr

  All 5 traces scored. Total credits: +21.5
```

Implementation: after upload, open an SSE connection to
`/api/v1/events/stream` and filter for `score_computed` events matching
the just-submitted `submission_id` set. Exit when all IDs have been seen
or after a configurable timeout (default 60s).

---

## Summary: Priority and Dependency Map

```
P0 (launch blockers):
  Section 1  TUI Dashboard           <- needs Section 5 (SSE)
  Section 2  Web Dashboard           <- needs Section 5 (SSE)
  Section 3  Progressive Disclosure  <- design pattern for Sections 1, 2
  Section 5  SSE Real-Time           <- foundational; everything consumes it
  Section 7  Onboarding Flow         <- first-run experience
  Section 10 CLI UX Improvements     <- polish existing surface

P1 (fast follow):
  Section 4  Provenance Cards        <- standalone, no dependencies
  Section 8  API Documentation       <- standalone, enables third-party integrations
  Section 9  Quality Visualization   <- depends on Sections 1, 2 for rendering hosts

P2 (future):
  Section 6  PWA / Mobile            <- depends on Section 2 (web dashboard)
```

### Recommended Build Order

1. **SSE event bus** (Section 5) -- wire `broadcast::Sender<PipelineEvent>` into
   the ingest and worker binaries. This is the foundation.
2. **CLI UX** (Section 10) -- `indicatif` progress bars, `--format`, completions,
   `--watch`. Quick wins on the existing surface.
3. **Onboarding wizard** (Section 7) -- `init` subcommand with guided setup.
4. **TUI dashboard** (Section 1) -- `tc-tui` binary consuming SSE events.
5. **Web dashboard** (Section 2) -- Next.js app with SSE integration.
6. **Progressive disclosure** (Section 3) -- applied as Sections 1+2 are built.
7. **API docs** (Section 8) -- `utoipa` + SwaggerUI on the ingest server.
8. **Provenance cards** (Section 4) -- SVG generation endpoint.
9. **Quality visualizations** (Section 9) -- Observable Plot for web, ratatui
   widgets for TUI.
10. **PWA** (Section 6) -- service worker, push notifications, offline queue.

### Crate Structure

```
trace-commons-server/
  crates/
    trace-commons-contributor/     # existing CLI (enhanced in Sections 7, 10)
    trace-commons-server/          # existing server (enhanced in Sections 4, 5, 8)
    trace-commons-protocol/        # existing types (new SSE event types)
    trace-commons-gate-api/        # existing (unchanged)
    trace-commons-gate-enclave/    # existing (unchanged)
    trace-commons-operator-client/ # existing (unchanged)
    tc-tui/                        # NEW: ratatui TUI binary (Section 1)
  web/                             # NEW: Next.js web dashboard (Sections 2, 6)
```

The TUI is a separate binary crate rather than a feature flag on the
contributor CLI, because ratatui + crossterm add significant compile-time
dependencies that most contributor CLI users (especially CI runners) do not
need. The web dashboard lives outside the Rust workspace entirely.
