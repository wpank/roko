# A6: Marketplace pages -- job board, create job, job detail

## Context

**Repo:** `/Users/will/dev/nunchi/nunchi-dashboard`
**Branch:** `demo-rewrite`
**Tech stack:** React 19 + Vite 8 + TypeScript + Tailwind CSS v4
**Backend:** `roko-serve` runs at `http://localhost:6677` with ~85 REST routes + WebSocket at `ws://localhost:6677/ws`
**Auth:** Privy (env var `VITE_PRIVY_APP_ID`) with password fallback
**Design:** ROSEDUST dark palette -- bg_void `#060608`, rose `#AA7088`, bone `#C8B890`, rose_bright `#CC90A8`

### Before starting
1. `cd /Users/will/dev/nunchi/nunchi-dashboard`
2. `git checkout -b demo-rewrite 2>/dev/null || git checkout demo-rewrite`
3. `npm install`
4. Verify: `npm run dev` starts without errors

### After every task
1. `npm run typecheck` passes
2. `npm run dev` -- page renders without console errors
3. All existing tests pass: `npm test` (if test runner is configured)

---

## What this task produces

Three marketplace pages and three shared components. The marketplace is a job board where users post tasks for agents. Agents claim jobs, submit deliverables, and evaluations gate whether payment releases.

**Depends on:** Task A1 (design system, router), Task A2 (API hooks and types).

**Audit update (2026-04-22):** `roko-serve` now has dedicated `/api/jobs` CRUD and lifecycle endpoints. This dashboard task is not complete until the mock marketplace data is replaced with those live hooks.

- [ ] Replace `MOCK_JOBS` in `JobBoard` and `JobDetail` with live `/api/jobs` and `/api/jobs/{id}` hooks, and wire `CreateJob` to `POST /api/jobs`.

---

## Data model reference

The canonical field names (from the `Job` type in A2) are:

```ts
type Job = {
  id: string;
  title: string;
  description: string;
  state: "open" | "assigned" | "in_progress" | "submitted" | "evaluated" | "expired";
  job_type: string;       // "research" | "engineering" | "defi" | "data"
  reward: number;
  required_capabilities: string[];
  posted_by: string;
  assigned_to: string | null;
  deadline: string | null;
  created_at: string;
  metadata: Record<string, unknown>;
};

type JobSubmission = {
  agent_id: string;
  submitted_at: string;
  result_summary: string;
  artifacts: string[];
  gate_results: Record<string, boolean>;
};

type JobEvaluation = {
  score: number;
  passed: boolean;
  feedback: string;
  evaluated_at: string;
};

// Shape of POST /api/jobs body when the endpoint exists
type CreateJobRequest = {
  title: string;
  description: string;
  job_type: string;
  metadata: Record<string, unknown>;
};
```

---

## Checklist

### 1. Create directories

```bash
mkdir -p /Users/will/dev/nunchi/nunchi-dashboard/src/pages/marketplace
mkdir -p /Users/will/dev/nunchi/nunchi-dashboard/src/pages/marketplace/components
```

### 2. Shared: JobCard

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/marketplace/components/JobCard.tsx`:

```tsx
import { useNavigate } from "react-router-dom";
import { Card, Badge } from "../../../design-system/components";
import type { Job } from "../../../types/api";

// Maps job state to a badge variant for visual scanning
const STATE_VARIANT: Record<string, "default" | "success" | "warning" | "error" | "info" | "rose"> = {
  open: "success",
  assigned: "info",
  in_progress: "info",
  submitted: "warning",
  evaluated: "rose",
  expired: "default",
};

// Maps job_type to a badge variant for secondary classification
const JOB_TYPE_VARIANT: Record<string, "default" | "success" | "warning" | "error" | "info" | "rose"> = {
  research: "info",
  engineering: "default",
  defi: "warning",
  data: "rose",
};

type JobCardProps = {
  job: Job;
};

export function JobCard({ job }: JobCardProps) {
  const navigate = useNavigate();

  return (
    <Card
      padding="md"
      className="cursor-pointer hover:border-[var(--rd-fg-muted)]/20 transition-colors"
    >
      <button
        type="button"
        className="w-full text-left focus:outline-none"
        onClick={() => navigate(`/app/marketplace/${job.id}`)}
      >
        <div className="flex items-start justify-between mb-2">
          <div className="flex-1 min-w-0">
            <h3 className="text-sm font-medium text-[var(--rd-fg-primary)] truncate">
              {job.title}
            </h3>
            <p className="text-[10px] text-[var(--rd-fg-muted)] mt-0.5 line-clamp-2">
              {job.description}
            </p>
          </div>
          <div className="flex items-center gap-1.5 ml-3 shrink-0">
            <Badge
              label={job.state}
              variant={STATE_VARIANT[job.state] ?? "default"}
            />
            <Badge
              label={job.job_type}
              variant={JOB_TYPE_VARIANT[job.job_type] ?? "default"}
            />
          </div>
        </div>

        <div className="flex items-center justify-between mt-3 pt-3 border-t border-[var(--rd-bg-surface-3)]">
          <div className="flex items-center gap-3 text-[10px] text-[var(--rd-fg-muted)]">
            <span className="capitalize">{job.job_type}</span>
            {job.required_capabilities.length > 0 && (
              <span>{job.required_capabilities.join(", ")}</span>
            )}
          </div>
          <div className="flex items-center gap-2">
            {job.metadata?.reward && (
              <span className="text-xs font-mono text-[var(--rd-accent-gold)]">
                {job.metadata.reward} DAEJI
              </span>
            )}
            {job.metadata?.deadline && (
              <span className="text-[10px] text-[var(--rd-fg-muted)]">
                due {new Date(job.metadata.deadline).toLocaleDateString()}
              </span>
            )}
          </div>
        </div>
      </button>
    </Card>
  );
}
```

### 3. Shared: StatusTimeline

The timeline reflects the canonical job state machine: `open → assigned → in_progress → submitted → evaluated`.

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/marketplace/components/StatusTimeline.tsx`:

```tsx
import type { Job } from "../../../types/api";

type TimelineStep = {
  label: string;
  state: Job["state"];
  timestamp: string | null;
};

type StatusTimelineProps = {
  currentState: Job["state"];
  steps: TimelineStep[];
};

// Order of states in the lifecycle; used to determine completed/active
const STATE_ORDER: Job["state"][] = [
  "open",
  "assigned",
  "in_progress",
  "submitted",
  "evaluated",
];

export function StatusTimeline({ currentState, steps }: StatusTimelineProps) {
  const currentIndex = STATE_ORDER.indexOf(currentState);

  return (
    <div className="flex items-start" role="list" aria-label="Job status timeline">
      {steps.map((step, i) => {
        const stepIndex = STATE_ORDER.indexOf(step.state);
        const isCompleted = stepIndex < currentIndex;
        const isActive = step.state === currentState;
        const showLeftConnector = i > 0;
        const showRightConnector = i < steps.length - 1;

        return (
          <div
            key={step.state}
            role="listitem"
            aria-current={isActive ? "step" : undefined}
            className="flex-1 flex flex-col items-center"
          >
            {/* Track row: left connector + dot + right connector */}
            <div className="flex items-center w-full">
              {showLeftConnector && (
                <div
                  className="flex-1 h-0.5 transition-colors"
                  style={{
                    backgroundColor:
                      isCompleted || isActive
                        ? "var(--rd-rose)"
                        : "var(--rd-bg-surface-3)",
                  }}
                />
              )}

              <div
                className={`w-3 h-3 rounded-full border-2 shrink-0 transition-all ${
                  isActive ? "animate-pulse" : ""
                }`}
                style={{
                  backgroundColor: isCompleted ? "var(--rd-rose)" : "transparent",
                  borderColor:
                    isCompleted || isActive
                      ? isActive
                        ? "var(--rd-rose-bright)"
                        : "var(--rd-rose)"
                      : "var(--rd-bg-surface-3)",
                }}
              />

              {showRightConnector && (
                <div
                  className="flex-1 h-0.5 transition-colors"
                  style={{
                    backgroundColor:
                      STATE_ORDER.indexOf(steps[i + 1].state) <= currentIndex
                        ? "var(--rd-rose)"
                        : "var(--rd-bg-surface-3)",
                  }}
                />
              )}
            </div>

            {/* Label + optional timestamp */}
            <div className="text-center mt-2">
              <div
                className="text-[10px] font-medium"
                style={{
                  color:
                    isCompleted || isActive
                      ? "var(--rd-fg-primary)"
                      : "var(--rd-fg-muted)",
                }}
              >
                {step.label}
              </div>
              {step.timestamp && (
                <div className="text-[9px] text-[var(--rd-fg-muted)] mt-0.5">
                  {step.timestamp}
                </div>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}
```

### 4. Shared: DeliverableViewer

Renders a `JobSubmission` using its canonical fields (`result_summary`, `artifacts`, `gate_results`).

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/marketplace/components/DeliverableViewer.tsx`:

```tsx
import { Card, Badge } from "../../../design-system/components";
import type { JobSubmission, JobEvaluation } from "../../../types/api";

type DeliverableViewerProps = {
  submission: JobSubmission | null;
  evaluation: JobEvaluation | null;
};

export function DeliverableViewer({ submission, evaluation }: DeliverableViewerProps) {
  if (!submission) {
    return (
      <Card>
        <div className="text-xs text-[var(--rd-fg-muted)] text-center py-8">
          No deliverable submitted yet.
        </div>
      </Card>
    );
  }

  const gateEntries = Object.entries(submission.gate_results ?? {});

  return (
    <div className="space-y-3">
      <Card>
        <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-3">
          Submission
        </div>
        <div className="flex items-center justify-between mb-2">
          <code className="text-xs font-mono text-[var(--rd-fg-secondary)]">
            {submission.agent_id}
          </code>
          <time
            dateTime={submission.submitted_at}
            className="text-[10px] text-[var(--rd-fg-muted)]"
          >
            {new Date(submission.submitted_at).toLocaleString()}
          </time>
        </div>

        {/* Result summary */}
        <div className="p-3 rounded-md bg-[var(--rd-bg-surface-0)] text-xs text-[var(--rd-fg-secondary)] whitespace-pre-wrap font-mono">
          {submission.result_summary}
        </div>

        {/* Artifacts */}
        {submission.artifacts.length > 0 && (
          <div className="mt-3">
            <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-1.5">
              Artifacts
            </div>
            <ul className="space-y-1">
              {submission.artifacts.map((path) => (
                <li key={path} className="text-[10px] font-mono text-[var(--rd-rose)] truncate">
                  {path}
                </li>
              ))}
            </ul>
          </div>
        )}

        {/* Gate results */}
        {gateEntries.length > 0 && (
          <div className="mt-3">
            <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-1.5">
              Gate results
            </div>
            <div className="flex flex-wrap gap-1.5">
              {gateEntries.map(([gate, passed]) => (
                <Badge
                  key={gate}
                  label={gate}
                  variant={passed ? "success" : "error"}
                />
              ))}
            </div>
          </div>
        )}
      </Card>

      {evaluation && (
        <Card>
          <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-3">
            Evaluation
          </div>
          <div className="flex items-center gap-4 mb-3">
            <div className="flex items-center gap-1.5">
              <span
                className="w-2 h-2 rounded-full"
                style={{
                  backgroundColor: evaluation.passed
                    ? "var(--rd-success)"
                    : "var(--rd-error)",
                }}
              />
              <span className="text-xs text-[var(--rd-fg-primary)]">
                {evaluation.passed ? "Passed" : "Failed"}
              </span>
            </div>
            <span className="text-xs font-mono text-[var(--rd-fg-muted)]">
              Score: {evaluation.score}/100
            </span>
            <time
              dateTime={evaluation.evaluated_at}
              className="text-[10px] text-[var(--rd-fg-muted)] ml-auto"
            >
              {new Date(evaluation.evaluated_at).toLocaleString()}
            </time>
          </div>
          <p className="text-xs text-[var(--rd-fg-secondary)]">
            {evaluation.feedback}
          </p>
        </Card>
      )}
    </div>
  );
}
```

### 5. JobBoard page

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/marketplace/JobBoard.tsx`:

```tsx
import { useState, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { Button, Badge, Select, EmptyState } from "../../design-system/components";
import { JobCard } from "./components/JobCard";
import type { Job } from "../../types/api";

// MOCK: replace with useJobs() when GET /api/jobs exists.
// All fields match the canonical Job type: state, job_type, assigned_to, posted_by, metadata.
const MOCK_JOBS: Job[] = [
  {
    id: "job-001",
    title: "Detect anomalous funding rate spreads",
    description: "Monitor perpetual futures across Binance, Bybit, and dYdX. Flag when funding rate spread exceeds 2 standard deviations from 30-day mean.",
    state: "open",
    job_type: "defi",
    reward: 25,
    required_capabilities: ["data-analysis", "market-monitoring"],
    posted_by: "human",
    assigned_to: null,
    created_at: "2026-04-20T14:00:00Z",
    deadline: "2026-04-25T00:00:00Z",
    metadata: {},
  },
  {
    id: "job-002",
    title: "Generate test coverage report for roko-gate",
    description: "Run cargo tarpaulin on the roko-gate crate. Produce a coverage report identifying uncovered branches in the adaptive threshold logic.",
    state: "in_progress",
    job_type: "engineering",
    reward: 15,
    required_capabilities: ["rust", "testing"],
    posted_by: "human",
    assigned_to: "researcher-02",
    created_at: "2026-04-19T10:00:00Z",
    deadline: null,
    metadata: {},
  },
  {
    id: "job-003",
    title: "Cross-venue liquidation cascade risk assessment",
    description: "Analyze open interest concentration across top 5 perp venues. Estimate cascading liquidation thresholds given a 10% BTC price drop.",
    state: "evaluated",
    job_type: "research",
    reward: 50,
    required_capabilities: ["research", "risk-analysis"],
    posted_by: "human",
    assigned_to: "sentinel-0x9b",
    created_at: "2026-04-18T08:00:00Z",
    deadline: "2026-04-20T00:00:00Z",
    metadata: {},
  },
  {
    id: "job-004",
    title: "Optimize cascade router cold-start behavior",
    description: "The cascade router defaults to the most expensive model for unknown task types. Implement a similarity-based fallback that uses HDC fingerprints.",
    state: "open",
    job_type: "engineering",
    reward: 30,
    required_capabilities: ["rust", "ml"],
    posted_by: "human",
    assigned_to: null,
    created_at: "2026-04-21T09:00:00Z",
    deadline: null,
    metadata: {},
  },
];

const STATE_FILTER_OPTIONS = [
  { value: "all", label: "All states" },
  { value: "open", label: "Open" },
  { value: "assigned", label: "Assigned" },
  { value: "in_progress", label: "In progress" },
  { value: "submitted", label: "Submitted" },
  { value: "evaluated", label: "Evaluated" },
  { value: "expired", label: "Expired" },
];

const TYPE_FILTER_OPTIONS = [
  { value: "all", label: "All types" },
  { value: "research", label: "Research" },
  { value: "engineering", label: "Engineering" },
  { value: "defi", label: "DeFi" },
  { value: "data", label: "Data" },
];

export default function JobBoard() {
  const navigate = useNavigate();
  const [stateFilter, setStateFilter] = useState("all");
  const [typeFilter, setTypeFilter] = useState("all");

  const filtered = MOCK_JOBS.filter((job) => {
    if (stateFilter !== "all" && job.state !== stateFilter) return false;
    if (typeFilter !== "all" && job.job_type !== typeFilter) return false;
    return true;
  });

  const openCount = MOCK_JOBS.filter((j) => j.state === "open").length;

  const handleStateFilter = useCallback((v: string) => setStateFilter(v), []);
  const handleTypeFilter = useCallback((v: string) => setTypeFilter(v), []);

  return (
    <section className="p-6">
      <header className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)]">
            Job board
          </h1>
          <p className="text-xs text-[var(--rd-fg-muted)] mt-0.5">
            {MOCK_JOBS.length} job{MOCK_JOBS.length !== 1 ? "s" : ""}, {openCount} open
          </p>
        </div>
        <Button size="sm" onClick={() => navigate("/app/marketplace/create")}>
          Post a job
        </Button>
      </header>

      {/* Filters */}
      <div className="flex items-center gap-3 mb-4">
        <div className="w-40">
          <Select
            value={stateFilter}
            onChange={handleStateFilter}
            options={STATE_FILTER_OPTIONS}
          />
        </div>
        <div className="w-40">
          <Select
            value={typeFilter}
            onChange={handleTypeFilter}
            options={TYPE_FILTER_OPTIONS}
          />
        </div>
        {(stateFilter !== "all" || typeFilter !== "all") && (
          <Badge
            label={`${filtered.length} match${filtered.length !== 1 ? "es" : ""}`}
            variant="info"
          />
        )}
      </div>

      {/* Job list */}
      {filtered.length === 0 ? (
        <EmptyState
          title="No jobs match your filters"
          description="Try changing the state or type filter."
        />
      ) : (
        <div className="space-y-2">
          {filtered.map((job) => (
            <JobCard key={job.id} job={job} />
          ))}
        </div>
      )}
    </section>
  );
}
```

### 6. CreateJob page

The form fields match `CreateJobRequest` (`title`, `description`, `job_type`, `metadata`). Priority is stored in `metadata.priority` so it can be passed through without a backend schema change.

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/marketplace/CreateJob.tsx`:

```tsx
import { useState, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { Card, Button, Input, Select } from "../../design-system/components";
import { useToast } from "../../design-system/components";
import type { CreateJobRequest } from "../../types/api";

const JOB_TYPE_OPTIONS = [
  { value: "engineering", label: "Engineering" },
  { value: "research", label: "Research" },
  { value: "defi", label: "DeFi" },
  { value: "data", label: "Data" },
];

const PRIORITY_OPTIONS = [
  { value: "low", label: "Low" },
  { value: "normal", label: "Normal" },
  { value: "high", label: "High" },
  { value: "critical", label: "Critical" },
];

export default function CreateJob() {
  const navigate = useNavigate();
  const { toast } = useToast();

  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [jobType, setJobType] = useState("engineering");
  const [priority, setPriority] = useState("normal");
  const [reward, setReward] = useState("10");
  const [capabilities, setCapabilities] = useState("");

  const handleSubmit = useCallback(() => {
    if (!title.trim()) {
      toast("Title is required", "error");
      return;
    }
    if (!description.trim()) {
      toast("Description is required", "error");
      return;
    }

    // Build a CreateJobRequest when the endpoint is available
    const _request: CreateJobRequest = {
      title: title.trim(),
      description: description.trim(),
      job_type: jobType,
      metadata: {
        priority,
        reward: Number(reward) || 0,
        required_capabilities: capabilities
          .split(",")
          .map((s) => s.trim())
          .filter(Boolean),
      },
    };

    // MOCK: wire to POST /api/jobs when endpoint exists
    toast("Job created (mock)", "success");
    navigate("/app/marketplace");
  }, [title, description, jobType, priority, reward, capabilities, toast, navigate]);

  return (
    <section className="p-6 max-w-2xl">
      <header className="mb-6">
        <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)]">
          Post a job
        </h1>
        <p className="text-xs text-[var(--rd-fg-muted)] mt-0.5">
          Define a task for agents to claim and execute.
        </p>
      </header>

      <Card>
        <div className="space-y-4">
          <Input
            label="Title"
            placeholder="What needs to be done?"
            value={title}
            onChange={setTitle}
          />

          <Input
            label="Description"
            placeholder="Describe the task, acceptance criteria, and any constraints..."
            value={description}
            onChange={setDescription}
            type="textarea"
          />

          <div className="grid grid-cols-2 gap-4">
            <Select
              label="Job type"
              value={jobType}
              onChange={setJobType}
              options={JOB_TYPE_OPTIONS}
            />

            <Select
              label="Priority"
              value={priority}
              onChange={setPriority}
              options={PRIORITY_OPTIONS}
            />
          </div>

          <Input
            label="Reward (DAEJI)"
            placeholder="10"
            value={reward}
            onChange={setReward}
            type="number"
          />

          <Input
            label="Required capabilities (comma-separated)"
            placeholder="rust, testing, data-analysis"
            value={capabilities}
            onChange={setCapabilities}
          />

          <div className="flex items-center justify-end gap-3 pt-4 border-t border-[var(--rd-bg-surface-3)]">
            <Button
              variant="ghost"
              onClick={() => navigate("/app/marketplace")}
            >
              Cancel
            </Button>
            <Button onClick={handleSubmit}>Create job</Button>
          </div>
        </div>
      </Card>
    </section>
  );
}
```

### 7. JobDetail page

- [ ] Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/marketplace/JobDetail.tsx`:

```tsx
import { useParams, useNavigate } from "react-router-dom";
import { Card, Badge, EmptyState } from "../../design-system/components";
import { StatusTimeline } from "./components/StatusTimeline";
import { DeliverableViewer } from "./components/DeliverableViewer";
import type { Job, JobSubmission, JobEvaluation } from "../../types/api";

type MockEntry = {
  job: Job;
  submission: JobSubmission | null;
  evaluation: JobEvaluation | null;
};

// MOCK: replace with useJob(id) when GET /api/jobs/:id exists.
// All job fields use the canonical names: state, job_type, assigned_to, posted_by, metadata.
// All submission fields use canonical names: result_summary, artifacts, gate_results.
const MOCK_JOBS: Record<string, MockEntry> = {
  "job-001": {
    job: {
      id: "job-001",
      title: "Detect anomalous funding rate spreads",
      description: "Monitor perpetual futures across Binance, Bybit, and dYdX. Flag when funding rate spread exceeds 2 standard deviations from 30-day mean.\n\nAcceptance criteria:\n- Pull live funding rates from at least 3 venues\n- Compute rolling 30-day mean and stddev\n- Alert when spread > 2 sigma\n- Output structured JSON with venue, pair, rate, z-score",
      state: "open",
      job_type: "defi",
      reward: 25,
      required_capabilities: ["data-analysis", "market-monitoring"],
      posted_by: "human",
      assigned_to: null,
      created_at: "2026-04-20T14:00:00Z",
      deadline: "2026-04-25T00:00:00Z",
      metadata: {},
    },
    submission: null,
    evaluation: null,
  },
  "job-003": {
    job: {
      id: "job-003",
      title: "Cross-venue liquidation cascade risk assessment",
      description: "Analyze open interest concentration across top 5 perp venues. Estimate cascading liquidation thresholds given a 10% BTC price drop.",
      state: "evaluated",
      job_type: "research",
      reward: 50,
      required_capabilities: ["research", "risk-analysis"],
      posted_by: "human",
      assigned_to: "sentinel-0x9b",
      created_at: "2026-04-18T08:00:00Z",
      deadline: "2026-04-20T00:00:00Z",
      metadata: {},
    },
    submission: {
      agent_id: "sentinel-0x9b",
      submitted_at: "2026-04-19T22:15:00Z",
      result_summary: "Analysis complete. Found 3 concentration hotspots:\n1. Binance BTC perp: $4.2B OI, liquidation cluster at $58,200\n2. Bybit ETH perp: $1.8B OI, liquidation cluster at $2,340\n3. dYdX BTC perp: $890M OI, liquidation cluster at $57,800\n\nCascade risk: HIGH. A 10% BTC drop would trigger ~$2.1B in liquidations across venues, with Binance acting as the primary cascade initiator.",
      artifacts: ["analysis/liquidation-cascade-2026-04-19.json"],
      gate_results: {
        compile: true,
        test: true,
        lint: true,
        review: true,
      },
    },
    evaluation: {
      score: 92,
      passed: true,
      feedback: "Thorough analysis with specific price levels and OI figures. Cross-venue correlation correctly identified. Minor gap: no analysis of stablecoin depegging risk as a secondary cascade trigger.",
      evaluated_at: "2026-04-20T01:30:00Z",
    },
  },
};

// Build timeline steps from canonical job state machine
function buildTimelineSteps(job: Job, submission: JobSubmission | null, evaluation: JobEvaluation | null) {
  return [
    {
      label: "Created",
      state: "open" as Job["state"],
      timestamp: new Date(job.created_at).toLocaleDateString(),
    },
    {
      label: "Assigned",
      state: "assigned" as Job["state"],
      timestamp: job.assigned_to ? `to ${job.assigned_to}` : null,
    },
    {
      label: "In progress",
      state: "in_progress" as Job["state"],
      timestamp: null,
    },
    {
      label: "Submitted",
      state: "submitted" as Job["state"],
      timestamp: submission?.submitted_at
        ? new Date(submission.submitted_at).toLocaleDateString()
        : null,
    },
    {
      label: "Evaluated",
      state: "evaluated" as Job["state"],
      timestamp: evaluation?.evaluated_at
        ? new Date(evaluation.evaluated_at).toLocaleDateString()
        : null,
    },
  ];
}

const STATE_BADGE_VARIANT: Record<string, "default" | "success" | "warning" | "error" | "info" | "rose"> = {
  open: "success",
  assigned: "info",
  in_progress: "info",
  submitted: "warning",
  evaluated: "rose",
  expired: "default",
};

export default function JobDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const entry = id ? MOCK_JOBS[id] : null;

  if (!entry) {
    return (
      <section className="p-6">
        <EmptyState
          title="Job not found"
          description={`No job with ID "${id ?? "unknown"}"`}
          action={{ label: "Back to board", onClick: () => navigate("/app/marketplace") }}
        />
      </section>
    );
  }

  const { job, submission, evaluation } = entry;
  const timelineSteps = buildTimelineSteps(job, submission, evaluation);

  return (
    <section className="p-6 max-w-3xl">
      {/* Back link */}
      <button
        type="button"
        onClick={() => navigate("/app/marketplace")}
        className="text-xs text-[var(--rd-fg-muted)] hover:text-[var(--rd-fg-secondary)] mb-4 flex items-center gap-1 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--rd-rose)] rounded"
      >
        <span className="material-symbols-outlined text-[14px]">arrow_back</span>
        Back to board
      </button>

      {/* Header */}
      <header className="flex items-start justify-between mb-6">
        <div>
          <h1 className="text-lg font-semibold text-[var(--rd-fg-primary)]">
            {job.title}
          </h1>
          <div className="flex items-center gap-2 mt-1">
            <Badge
              label={job.state}
              variant={STATE_BADGE_VARIANT[job.state] ?? "default"}
            />
            <Badge label={job.job_type} variant="default" />
            {job.metadata?.reward && (
              <span className="text-xs font-mono text-[var(--rd-accent-gold)]">
                {job.metadata.reward} DAEJI
              </span>
            )}
          </div>
        </div>
      </header>

      {/* Timeline */}
      <Card className="mb-4">
        <StatusTimeline currentState={job.state} steps={timelineSteps} />
      </Card>

      {/* Description + metadata */}
      <Card className="mb-4">
        <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-2">
          Description
        </div>
        <div className="text-sm text-[var(--rd-fg-secondary)] whitespace-pre-wrap">
          {job.description}
        </div>
        <div className="mt-4 pt-3 border-t border-[var(--rd-bg-surface-3)] grid grid-cols-3 gap-4 text-xs text-[var(--rd-fg-muted)]">
          <div>
            <div className="text-[10px] uppercase tracking-wider mb-0.5">Job type</div>
            <div className="text-[var(--rd-fg-secondary)] capitalize">{job.job_type}</div>
          </div>
          <div>
            <div className="text-[10px] uppercase tracking-wider mb-0.5">Capabilities</div>
            <div className="text-[var(--rd-fg-secondary)]">
              {job.required_capabilities.length > 0
                ? job.required_capabilities.join(", ")
                : "None specified"}
            </div>
          </div>
          <div>
            <div className="text-[10px] uppercase tracking-wider mb-0.5">Deadline</div>
            <div className="text-[var(--rd-fg-secondary)]">
              {job.deadline ? new Date(job.deadline).toLocaleDateString() : "None"}
            </div>
          </div>
        </div>
      </Card>

      {/* Deliverable */}
      <div className="text-[10px] uppercase tracking-wider text-[var(--rd-fg-muted)] mb-2">
        Deliverable
      </div>
      <DeliverableViewer submission={submission} evaluation={evaluation} />
    </section>
  );
}
```

### 8. Wire pages into the router

- [ ] Update `/Users/will/dev/nunchi/nunchi-dashboard/src/router.tsx` -- replace the three marketplace placeholders:

Find:
```
{ path: "marketplace", element: <Placeholder name="Job board" /> },
{ path: "marketplace/create", element: <Placeholder name="Create job" /> },
{ path: "marketplace/:id", element: <Placeholder name="Job detail" /> },
```

Replace with:
```tsx
{ path: "marketplace", element: lazyPage(() => import("./pages/marketplace/JobBoard")) },
{ path: "marketplace/create", element: lazyPage(() => import("./pages/marketplace/CreateJob")) },
{ path: "marketplace/:id", element: lazyPage(() => import("./pages/marketplace/JobDetail")) },
```

---

## Verification

Run from `/Users/will/dev/nunchi/nunchi-dashboard`:

- [ ] `npm run typecheck` -- exits 0
- [ ] `npm run dev` -- navigate to each route:
  - `/app/marketplace` -- job board renders with 4 mock jobs, filters work
  - `/app/marketplace/create` -- form renders, validation fires on empty fields, "Create job" shows toast
  - `/app/marketplace/job-001` -- detail page with timeline at "open" step, description, no deliverable
  - `/app/marketplace/job-003` -- detail page with "evaluated" timeline, `result_summary` in code block, artifacts list, gate result badges, evaluation with score
- [ ] "Post a job" button navigates to create form
- [ ] "Back to board" link navigates back
- [ ] All mock job data uses new field names: `state`, `job_type`, `assigned_to`, `posted_by`, `metadata`
- [ ] State filter dropdown says "All states" (not "All statuses" or "All domains")
- [ ] Type filter dropdown says "All types" (not "All domains")
- [ ] `STATE_VARIANT` map includes all 6 states: `open`, `assigned`, `in_progress`, `submitted`, `evaluated`, `expired`
- [ ] `StatusTimeline` receives `currentState` prop and computes completed/active from `STATE_ORDER`
- [ ] `DeliverableViewer` renders `result_summary` (not `content`), `artifacts` (not `files`), and `gate_results`
- [ ] `CreateJob` form has "Job type" dropdown (not "Domain"); priority stored in `metadata.priority`
- [ ] JobDetail badge shows `job.state` variant from `STATE_BADGE_VARIANT` (not hardcoded "info")
- [ ] JobDetail metadata label says "Job type" (not "Domain")
- [ ] No `any` types; no broken variable/setter name pairs (`stateFilter`/`setStateFilter`, `typeFilter`/`setTypeFilter`)
- [ ] No console errors
