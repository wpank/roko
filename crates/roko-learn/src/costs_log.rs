//! Append-only JSONL persistence for [`crate::costs_db::CostRecord`].
//!
//! `CostsDb` is intentionally in-memory. This module provides the durable,
//! file-backed companion used by runtime wiring: append each completed call as
//! one JSON line, then reload on process start.

use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{collections::HashMap, hash::Hash};

use chrono::{DateTime, Utc};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::costs_db::CostRecord;

/// What a [`CostsLog`] recorded on one UTC calendar day.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DaySpend {
    /// Sum of the day's priced calls, in USD.
    pub cost_usd: f64,
    /// Calls whose cost is unknown: recorded at $0 although they used
    /// tokens, so never priced, or at a cost that is negative or not a
    /// number. Their cost is unknown, not zero.
    pub unpriced_calls: usize,
}

/// Append-only JSONL log for [`CostRecord`] values.
#[derive(Debug, Clone)]
pub struct CostsLog {
    path: PathBuf,
    fsync: bool,
}

impl CostsLog {
    /// Construct a log at `path`.
    #[must_use]
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            fsync: true,
        }
    }

    /// Create parent directories and return a log at `path`.
    ///
    /// # Errors
    ///
    /// Returns an error when parent directories cannot be created.
    pub async fn open_creating(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        Ok(Self { path, fsync: true })
    }

    /// Path to the underlying JSONL file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Disable fsync after appends.
    #[must_use]
    pub const fn without_fsync(mut self) -> Self {
        self.fsync = false;
        self
    }

    /// Append one [`CostRecord`] as one JSON line.
    ///
    /// Each call opens, writes, optionally fsyncs, and closes the file.
    /// For high-throughput paths (many concurrent agents), prefer collecting
    /// records into a `Vec` and calling [`append_all`] in a periodic flush
    /// to amortize the syscall overhead.
    ///
    /// # Errors
    ///
    /// Returns an error for serialization or file I/O failures.
    pub async fn append(&self, record: &CostRecord) -> io::Result<()> {
        let mut line = serde_json::to_string(record)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        line.push('\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;
        file.write_all(line.as_bytes()).await?;
        file.flush().await?;
        if self.fsync {
            file.sync_data().await?;
        }
        Ok(())
    }

    /// Append many records with one open/close cycle.
    ///
    /// # Errors
    ///
    /// Returns an error for serialization or file I/O failures.
    pub async fn append_all(&self, records: &[CostRecord]) -> io::Result<()> {
        if records.is_empty() {
            return Ok(());
        }
        let mut buf = String::new();
        for record in records {
            let line = serde_json::to_string(record)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            buf.push_str(&line);
            buf.push('\n');
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;
        file.write_all(buf.as_bytes()).await?;
        file.flush().await?;
        if self.fsync {
            file.sync_data().await?;
        }
        Ok(())
    }

    /// Return whether the log has fsync enabled.
    ///
    /// Callers that do high-frequency appends can check this and batch
    /// records themselves before calling [`append_all`].
    #[must_use]
    pub const fn fsync_enabled(&self) -> bool {
        self.fsync
    }

    /// Read all valid records; malformed lines are skipped.
    ///
    /// # Errors
    ///
    /// Returns an error only for file open/read failures.
    pub async fn read_all(&self) -> io::Result<Vec<CostRecord>> {
        let file = match tokio::fs::File::open(&self.path).await {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(err),
        };
        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut out = Vec::new();
        while let Some(line) = lines.next_line().await? {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(record) = serde_json::from_str::<CostRecord>(trimmed) {
                out.push(record);
            }
        }
        Ok(out)
    }

    /// Return the total recorded cost in USD.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying log cannot be read.
    pub async fn total_cost(&self) -> io::Result<f64> {
        let total: f64 = self
            .read_all()
            .await?
            .into_iter()
            .map(|record| record.cost_usd)
            .sum();
        Ok(total.max(0.0))
    }

    /// Aggregate recorded cost by model slug.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying log cannot be read.
    pub async fn cost_by_model(&self) -> io::Result<HashMap<String, f64>> {
        Ok(aggregate_costs(self.read_all().await?, |record| {
            record.model.clone()
        }))
    }

    /// Aggregate recorded cost by plan id.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying log cannot be read.
    pub async fn cost_by_plan(&self) -> io::Result<HashMap<String, f64>> {
        Ok(aggregate_costs(self.read_all().await?, |record| {
            record.plan_id.clone()
        }))
    }

    /// Return a zero-filled daily cost breakdown for the most recent `days`
    /// calendar days, ordered oldest-to-newest.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying log cannot be read.
    pub async fn daily_cost(&self, days: usize) -> io::Result<Vec<(String, f64)>> {
        if days == 0 {
            return Ok(Vec::new());
        }

        let records = self.read_all().await?;
        let mut totals: HashMap<chrono::NaiveDate, f64> = HashMap::new();
        for record in records {
            if let Some(date) = record_timestamp(&record).map(|ts| ts.date_naive()) {
                *totals.entry(date).or_default() += record.cost_usd;
            }
        }

        let today = Utc::now().date_naive();
        let mut out = Vec::with_capacity(days);
        for offset in (0..days).rev() {
            let day = today - chrono::Duration::days(offset as i64);
            out.push((
                day.format("%Y-%m-%d").to_string(),
                totals.get(&day).copied().unwrap_or(0.0).max(0.0),
            ));
        }
        Ok(out)
    }

    /// Return today's total cost in USD (calendar day in UTC).
    ///
    /// Convenience wrapper around [`daily_cost(1)`](Self::daily_cost).
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying log cannot be read.
    pub async fn cost_today(&self) -> io::Result<f64> {
        Ok(self
            .daily_cost(1)
            .await?
            .last()
            .map(|(_, cost)| *cost)
            .unwrap_or(0.0))
    }

    /// What the log recorded on `day` (UTC calendar day): the priced
    /// spend, and how many calls have an unknown cost. Records whose
    /// timestamp does not parse belong to no day.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying log cannot be read.
    pub async fn spend_on(&self, day: chrono::NaiveDate) -> io::Result<DaySpend> {
        let mut spend = DaySpend::default();
        for record in self.read_all().await? {
            if record_timestamp(&record).map(|timestamp| timestamp.date_naive()) != Some(day) {
                continue;
            }
            if is_unpriced(&record) {
                spend.unpriced_calls += 1;
            } else {
                spend.cost_usd += record.cost_usd;
            }
        }
        Ok(spend)
    }

    /// Return the recent cost rate for the last `window` of wall-clock time.
    ///
    /// The result is expressed in USD/minute.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying log cannot be read.
    pub async fn recent_cost_rate(&self, window: Duration) -> io::Result<f64> {
        let records = self.read_all().await?;
        Ok(recent_cost_rate_from_records(&records, window))
    }

    /// Return `true` when the recent cost rate exceeds `threshold`.
    ///
    /// Uses a conservative 15-minute window by default.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying log cannot be read.
    pub async fn is_cost_spike(&self, threshold: f64) -> io::Result<bool> {
        Ok(self.recent_cost_rate(DEFAULT_COST_SPIKE_WINDOW).await? > threshold)
    }
}

/// Default lookback window for cost-spike detection.
pub const DEFAULT_COST_SPIKE_WINDOW: Duration = Duration::from_mins(15);

fn recent_cost_rate_from_records(records: &[CostRecord], window: Duration) -> f64 {
    if window.is_zero() {
        return 0.0;
    }

    let Ok(window_chrono) = chrono::Duration::from_std(window) else {
        return 0.0;
    };
    let cutoff = Utc::now() - window_chrono;
    let recent_cost: f64 = records
        .iter()
        .filter_map(|record| record_timestamp(record).map(|ts| (ts, record.cost_usd)))
        .filter(|(ts, _)| *ts >= cutoff)
        .map(|(_, cost)| cost)
        .sum();

    recent_cost / (window.as_secs_f64() / 60.0)
}

/// Whether `record`'s cost is unknown: recorded at $0 although the call used
/// tokens, or at a cost that is negative or not a finite number.
fn is_unpriced(record: &CostRecord) -> bool {
    let tokens = record
        .input_tokens
        .saturating_add(record.output_tokens)
        .saturating_add(record.cached_tokens);
    let priced = record.cost_usd.is_finite() && record.cost_usd >= 0.0;
    !priced || (record.cost_usd <= f64::EPSILON && tokens > 0)
}

fn record_timestamp(record: &CostRecord) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&record.timestamp)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc))
}

fn aggregate_costs<K, F>(records: Vec<CostRecord>, mut key_fn: F) -> HashMap<K, f64>
where
    K: Eq + Hash,
    F: FnMut(&CostRecord) -> K,
{
    let mut out = HashMap::new();
    for record in records {
        *out.entry(key_fn(&record)).or_default() += record.cost_usd;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;
    use tempfile::TempDir;

    fn record(task: &str, cost: f64) -> CostRecord {
        CostRecord {
            timestamp: Utc::now().to_rfc3339(),
            model: "claude-opus-4-6".to_string(),
            provider: "anthropic".to_string(),
            role: "Implementer".to_string(),
            plan_id: "plan-1".to_string(),
            task_id: task.to_string(),
            complexity_band: "standard".to_string(),
            input_tokens: 100,
            output_tokens: 50,
            cached_tokens: 0,
            cost_usd: cost,
            duration_ms: 1234,
            success: true,
            session_id: "sess-1".to_string(),
        }
    }

    /// A day's spend sums its priced calls, counts the calls whose cost is
    /// unknown, and leaves other days and undated records out.
    #[tokio::test]
    async fn spend_on_counts_one_day_and_tells_unpriced_calls_apart() {
        let tmp = TempDir::new().unwrap();
        let log = CostsLog::at(tmp.path().join("costs.jsonl"));
        let today = Utc::now();
        let free = CostRecord {
            input_tokens: 0,
            output_tokens: 0,
            ..record("free", 0.0)
        };
        let yesterday = CostRecord {
            timestamp: (today - ChronoDuration::days(1)).to_rfc3339(),
            ..record("yesterday", 5.0)
        };
        let undated = CostRecord {
            timestamp: "not a timestamp".to_string(),
            ..record("undated", 5.0)
        };
        log.append_all(&[
            record("priced", 0.25),
            record("priced", 0.5),
            record("unpriced", 0.0),
            record("negative", -1.0),
            free,
            yesterday,
            undated,
        ])
        .await
        .unwrap();

        let spend = log.spend_on(today.date_naive()).await.unwrap();
        assert!((spend.cost_usd - 0.75).abs() < 1e-9, "{spend:?}");
        assert_eq!(spend.unpriced_calls, 2);

        let missing = CostsLog::at(tmp.path().join("missing.jsonl"));
        assert_eq!(
            missing.spend_on(today.date_naive()).await.unwrap(),
            DaySpend::default()
        );
    }

    #[tokio::test]
    async fn append_and_read_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("costs.jsonl");
        let log = CostsLog::at(&path);
        let r1 = record("t1", 0.1);
        let r2 = record("t2", 0.2);

        log.append(&r1).await.unwrap();
        log.append(&r2).await.unwrap();

        let all = log.read_all().await.unwrap();
        assert_eq!(all, vec![r1, r2]);
    }

    #[tokio::test]
    async fn append_all_writes_batch() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("costs.jsonl");
        let log = CostsLog::at(&path);
        let batch = vec![record("t1", 0.1), record("t2", 0.2), record("t3", 0.3)];
        log.append_all(&batch).await.unwrap();
        let all = log.read_all().await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn read_all_skips_malformed_lines() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("costs.jsonl");
        tokio::fs::write(
            &path,
            format!(
                "{}\n{}\n{}\n",
                serde_json::to_string(&record("ok-1", 0.1)).unwrap(),
                "{ malformed json",
                serde_json::to_string(&record("ok-2", 0.2)).unwrap()
            ),
        )
        .await
        .unwrap();
        let log = CostsLog::at(&path);
        let all = log.read_all().await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].task_id, "ok-1");
        assert_eq!(all[1].task_id, "ok-2");
    }

    #[tokio::test]
    async fn recent_cost_rate_and_spike_detection_use_recent_records_only() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("costs.jsonl");
        let log = CostsLog::at(&path);
        let now = Utc::now();
        let recent = CostRecord {
            timestamp: now.to_rfc3339(),
            ..record("recent", 0.75)
        };
        let stale = CostRecord {
            timestamp: (now - ChronoDuration::minutes(30)).to_rfc3339(),
            ..record("stale", 4.0)
        };
        log.append_all(&[recent.clone(), stale]).await.unwrap();

        let rate = log
            .recent_cost_rate(Duration::from_secs(10 * 60))
            .await
            .unwrap();
        assert!(rate > 0.0);
        assert!(rate < 1.0);
        assert!(log.is_cost_spike(0.01).await.unwrap());
        assert!(!log.is_cost_spike(100.0).await.unwrap());
    }

    #[tokio::test]
    async fn cost_aggregations_cover_model_plan_and_daily_trends() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("costs.jsonl");
        let log = CostsLog::at(&path);
        let today = Utc::now().date_naive();
        let make_record =
            |date: chrono::NaiveDate, hour: u32, model: &str, plan: &str, cost: f64| CostRecord {
                timestamp: date.and_hms_opt(hour, 0, 0).unwrap().and_utc().to_rfc3339(),
                model: model.to_string(),
                provider: "anthropic".to_string(),
                role: "Implementer".to_string(),
                plan_id: plan.to_string(),
                task_id: format!("{plan}-{hour}"),
                complexity_band: "standard".to_string(),
                input_tokens: 100,
                output_tokens: 50,
                cached_tokens: 0,
                cost_usd: cost,
                duration_ms: 1234,
                success: true,
                session_id: "sess-1".to_string(),
            };

        let two_days_ago = today - ChronoDuration::days(2);
        let yesterday = today - ChronoDuration::days(1);
        log.append_all(&[
            make_record(two_days_ago, 9, "glm-5.1", "plan-a", 1.25),
            make_record(yesterday, 10, "glm-5.1", "plan-b", 2.50),
            make_record(today, 11, "claude-opus-4-6", "plan-a", 3.75),
        ])
        .await
        .unwrap();

        assert!((log.total_cost().await.unwrap() - 7.50).abs() < f64::EPSILON);

        let by_model = log.cost_by_model().await.unwrap();
        assert!((by_model["glm-5.1"] - 3.75).abs() < f64::EPSILON);
        assert!((by_model["claude-opus-4-6"] - 3.75).abs() < f64::EPSILON);

        let by_plan = log.cost_by_plan().await.unwrap();
        assert!((by_plan["plan-a"] - 5.00).abs() < f64::EPSILON);
        assert!((by_plan["plan-b"] - 2.50).abs() < f64::EPSILON);

        let daily = log.daily_cost(3).await.unwrap();
        assert_eq!(daily.len(), 3);
        assert_eq!(daily[0].0, two_days_ago.format("%Y-%m-%d").to_string());
        assert_eq!(daily[1].0, yesterday.format("%Y-%m-%d").to_string());
        assert_eq!(daily[2].0, today.format("%Y-%m-%d").to_string());
        assert!((daily[0].1 - 1.25).abs() < f64::EPSILON);
        assert!((daily[1].1 - 2.50).abs() < f64::EPSILON);
        assert!((daily[2].1 - 3.75).abs() < f64::EPSILON);
    }
}
