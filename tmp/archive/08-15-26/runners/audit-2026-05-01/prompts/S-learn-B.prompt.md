# S-learn-B: KnowledgeIngestionSink failure budget

## Task
Add a failure budget to `KnowledgeIngestionSink` so ingestion errors don't abort the runner but excessive failures alert. JSONL write remains durable.

## Runner Context
Runner audit-2026-05-01, group S. Depends on T4-29. Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/25-learning-feedback-completion.md` § Phase B.

## Exact changes

`crates/roko-cli/src/runtime_feedback/knowledge.rs`:

```rust
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug)]
pub struct KnowledgeIngestionSink {
    candidates_path: PathBuf,
    file: Mutex<Option<tokio::fs::File>>,
    ingestor: Option<Arc<dyn KnowledgeIngestor>>,
    ingestion_failures: AtomicU32,
    ingestion_total: AtomicU32,
    failure_budget_percent: u32,
}

impl KnowledgeIngestionSink {
    pub fn at(path: ...) -> Self {
        Self {
            // ... existing fields
            ingestion_failures: AtomicU32::new(0),
            ingestion_total: AtomicU32::new(0),
            failure_budget_percent: 5,    // default 5%
        }
    }

    pub fn with_failure_budget_percent(mut self, pct: u32) -> Self {
        self.failure_budget_percent = pct;
        self
    }
}

#[async_trait]
impl FeedbackSink for KnowledgeIngestionSink {
    async fn on_event(&self, event: &FeedbackEvent) -> Result<(), anyhow::Error> {
        // ... existing JSONL write (unchanged)

        if let Some(ingestor) = &self.ingestor {
            self.ingestion_total.fetch_add(1, Ordering::Relaxed);
            if let Err(e) = ingestor.ingest(&candidate).await {
                let f = self.ingestion_failures.fetch_add(1, Ordering::Relaxed) + 1;
                let t = self.ingestion_total.load(Ordering::Relaxed);
                tracing::warn!(error = %e, failures = f, total = t, "knowledge ingestion failed");
                if t > 20 && (f * 100 / t) > self.failure_budget_percent {
                    tracing::error!(
                        rate_percent = (f as f64) * 100.0 / (t as f64),
                        budget_percent = self.failure_budget_percent,
                        "knowledge ingestion failure rate exceeds budget"
                    );
                }
                // Continue: never abort the runner.
            }
        }
        Ok(())
    }
}
```

## Write Scope
- `crates/roko-cli/src/runtime_feedback/knowledge.rs`

## Verify

```bash
rg 'failure_budget_percent|ingestion_failures|ingestion_total' crates/roko-cli/src/runtime_feedback/knowledge.rs
# Expect: 5+ hits
```

## Do NOT

- Do NOT abort the runner on ingestion failure.
- Do NOT skip the JSONL write when ingestor is present (durable record).
- Do NOT bundle with T4-29 / S-learn-A/C/D/E.
- Do NOT use blocking sync primitives (e.g. `std::sync::Mutex<u32>`); `AtomicU32` is fine.
