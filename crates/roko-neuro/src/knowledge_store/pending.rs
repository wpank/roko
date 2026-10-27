//! Confirmations held until a run delivers (gap-1220c5).
//!
//! A guarded run's passes confirm the committed entries their prompts
//! surfaced, and its lessons confirm the committed entries they repeat. Those
//! confirmations wait in the ledger [`PENDING_FILE`] beside the store, tagged
//! with the run's batch, until the batch's guarded commit settles them: a
//! batch that commits applies them, and one that rolls back drops them, as an
//! undelivered run's lessons are dropped (bug-2ad4fa). A run without a batch
//! (`[learning] guarded_commit = "off"`) confirms at once, as before.

use std::collections::{BTreeSet, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::KnowledgeStore;
use super::types::KnowledgeConfirmationRecord;
use crate::tier_progression::TierProgression;
use crate::{KnowledgeEntry, KnowledgeTier};

/// The ledger of held confirmations, beside the knowledge store's file.
pub const PENDING_FILE: &str = "knowledge-pending-confirmations.jsonl";

/// A confirmation of a committed entry that waits for its run's delivery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingConfirmation {
    /// The batch of the run that observed it.
    pub batch: String,
    /// The committed entry it confirms.
    pub entry: String,
    /// When the run observed it.
    pub at: DateTime<Utc>,
    /// How the run confirms the entry.
    #[serde(flatten)]
    pub by: PendingBy,
}

/// How a run confirms a committed entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "by", rename_all = "snake_case")]
pub enum PendingBy {
    /// A verified pass whose prompt surfaced the entry.
    Surfaced {
        /// The pass's context: plan, task and episode.
        context: String,
        /// The pass's episode.
        episode: String,
    },
    /// A lesson the run stored that repeats the entry.
    Repeat {
        /// The lesson's entry.
        confirming: String,
        /// The lesson's source episodes.
        episodes: Vec<String>,
    },
}

impl KnowledgeStore {
    /// The ledger of held confirmations, beside the store's file.
    #[must_use]
    pub fn pending_path(&self) -> PathBuf {
        self.path.parent().map_or_else(
            || PathBuf::from(PENDING_FILE),
            |parent| parent.join(PENDING_FILE),
        )
    }

    /// The held confirmations of every batch, oldest first; none without
    /// the ledger. A row that does not parse is skipped.
    ///
    /// # Errors
    ///
    /// The ledger's read error.
    pub fn pending_confirmations(&self) -> Result<Vec<PendingConfirmation>> {
        let path = self.pending_path();
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error).with_context(|| format!("open {}", path.display())),
        };
        let mut rows = Vec::new();
        for line in BufReader::new(file).lines() {
            let line = line.with_context(|| format!("read {}", path.display()))?;
            if let Ok(row) = serde_json::from_str::<PendingConfirmation>(&line) {
                rows.push(row);
            }
        }
        Ok(rows)
    }

    /// Hold `rows`, a guarded run's confirmations of committed entries,
    /// until its batch settles.
    ///
    /// # Errors
    ///
    /// The ledger's write error.
    pub(crate) fn hold_confirmations(&self, rows: &[PendingConfirmation]) -> Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let _guard = self.lock_writes();
        self.append_pending(rows)
    }

    /// [`Self::hold_confirmations`] for a caller that holds the store's
    /// write lock.
    pub(crate) fn append_pending(&self, rows: &[PendingConfirmation]) -> Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let path = self.pending_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        let mut text = String::new();
        for row in rows {
            text.push_str(&serde_json::to_string(row).context("serialize a held confirmation")?);
            text.push('\n');
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("open {}", path.display()))?;
        file.write_all(text.as_bytes())
            .and_then(|()| file.sync_all())
            .with_context(|| format!("append to {}", path.display()))
    }

    /// The batches that hold confirmations.
    pub(crate) fn pending_batches(&self) -> Result<BTreeSet<String>> {
        let rows = self.pending_confirmations()?;
        Ok(rows.into_iter().map(|row| row.batch).collect())
    }

    /// Apply `batch`'s held confirmations to `entries`, and skip an entry a
    /// person has retired since. A run confirms an entry at most once, by
    /// the first of its held confirmations that counts; a later pass that
    /// surfaced the entry is only noted, as a pass that confirmed it already
    /// is (gap-1220c5). Returns how many applied and the confirmation records
    /// of the repeats that counted. The caller holds the write lock and
    /// rewrites `entries`.
    pub(crate) fn apply_pending(
        &self,
        batch: &str,
        entries: &mut [KnowledgeEntry],
    ) -> Result<(usize, Vec<KnowledgeConfirmationRecord>)> {
        let rows = self.pending_confirmations()?;
        let mut confirmed = HashSet::new();
        let (mut applied, mut records) = (0, Vec::new());
        for row in rows.into_iter().filter(|row| row.batch == batch) {
            let Some(entry) = entries
                .iter_mut()
                .find(|entry| entry.id == row.entry && !entry.deprecated)
            else {
                continue;
            };
            applied += 1;
            if confirmed.contains(&row.entry) {
                if let PendingBy::Surfaced { episode, .. } = &row.by {
                    note_surfacing(entry, episode);
                }
                continue;
            }
            let counted = match &row.by {
                PendingBy::Surfaced { context, episode } => {
                    confirm_by_surfacing(entry, context, episode)
                }
                PendingBy::Repeat {
                    confirming,
                    episodes,
                } => {
                    let mut source_episodes = entry.source_episodes.clone();
                    source_episodes.extend(episodes.iter().cloned());
                    source_episodes.sort();
                    source_episodes.dedup();
                    let counted = confirm_by_repeat(entry, episodes);
                    if counted {
                        records.push(KnowledgeConfirmationRecord {
                            created_at: row.at,
                            source_episodes,
                            confirmed_entry_id: row.entry.clone(),
                            confirming_entry_id: confirming.clone(),
                        });
                    }
                    counted
                }
            };
            if counted {
                confirmed.insert(row.entry);
            }
        }
        Ok((applied, records))
    }

    /// Take `batch`'s held confirmations out of the ledger: they applied,
    /// or their run did not deliver. Returns how many. The caller holds the
    /// write lock.
    pub(crate) fn drop_pending(&self, batch: &str) -> Result<usize> {
        let rows = self.pending_confirmations()?;
        let before = rows.len();
        let kept: Vec<PendingConfirmation> =
            rows.into_iter().filter(|row| row.batch != batch).collect();
        let dropped = before - kept.len();
        if dropped == 0 {
            return Ok(0);
        }
        let path = self.pending_path();
        let mut text = String::new();
        for row in &kept {
            text.push_str(&serde_json::to_string(row).context("serialize a held confirmation")?);
            text.push('\n');
        }
        roko_fs::atomic_write_bytes(&path, text.as_bytes())
            .with_context(|| format!("rewrite {}", path.display()))?;
        Ok(dropped)
    }
}

/// Whether `entry` counts a confirmation from `episode` already: a context
/// that is the episode, as a repeat records it, or that ends with it, as a
/// surfacing pass's `plan:task:episode` does.
fn counts_episode(entry: &KnowledgeEntry, episode: &str) -> bool {
    entry.distinct_contexts.iter().any(|context| {
        context == episode
            || context
                .strip_suffix(episode)
                .is_some_and(|rest| rest.ends_with(':'))
    })
}

/// Count a verified pass that surfaced `entry` in `context` as one
/// confirmation of it, once per context, and re-evaluate its tier. A pass
/// confirms an entry once, whether it surfaced it, repeated it or both
/// (gap-1220c5). Returns whether it counted.
pub(crate) fn confirm_by_surfacing(
    entry: &mut KnowledgeEntry,
    context: &str,
    episode: &str,
) -> bool {
    let counted = !counts_episode(entry, episode)
        && !entry.distinct_contexts.iter().any(|known| known == context);
    if counted {
        entry.distinct_contexts.push(context.to_string());
        entry.confirmation_count = entry.confirmation_count.saturating_add(1);
        entry.confidence = (entry.confidence + 0.03).clamp(0.0, 1.0);
    }
    note_surfacing(entry, episode);
    counted
}

/// Note `episode`, a verified pass that surfaced `entry`, as one of its
/// sources, and re-evaluate its tier by its confirmations.
fn note_surfacing(entry: &mut KnowledgeEntry, episode: &str) {
    if !entry.source_episodes.iter().any(|known| known == episode) {
        entry.source_episodes.push(episode.to_string());
    }
    let count = entry.confirmation_count as usize;
    if let Some(tier) = TierProgression::evaluate_tier_progression_v2(entry, count, 0).tier() {
        entry.tier = tier;
    }
}

/// Count `episodes`, the sources of a lesson that repeats `entry`, as one
/// confirmation of it, and promote it by the confirmation thresholds: 2
/// confirmations make a transient entry working, and 3 contexts a working one
/// consolidated (P2-15). A pass confirms an entry once, whether it surfaced
/// it, repeated it or both, so a repeat whose episodes the entry counts
/// already counts nothing (gap-1220c5). Returns whether it counted.
pub(crate) fn confirm_by_repeat(entry: &mut KnowledgeEntry, episodes: &[String]) -> bool {
    let fresh: Vec<&String> = episodes
        .iter()
        .filter(|episode| !counts_episode(entry, episode))
        .collect();
    if !episodes.is_empty() && fresh.is_empty() {
        return false;
    }
    entry.confirmation_count = entry.confirmation_count.saturating_add(1);
    for episode in fresh {
        if !entry.distinct_contexts.contains(episode) {
            entry.distinct_contexts.push(episode.clone());
        }
    }
    let old_tier = entry.tier;
    match entry.tier {
        KnowledgeTier::Transient if entry.confirmation_count >= 2 => {
            entry.tier = KnowledgeTier::Working;
        }
        KnowledgeTier::Working if entry.distinct_contexts.len() >= 3 => {
            entry.tier = KnowledgeTier::Consolidated;
        }
        _ => {}
    }
    if entry.tier != old_tier {
        tracing::info!(
            knowledge_id = %entry.id,
            from_tier = ?old_tier,
            to_tier = ?entry.tier,
            confirmations = entry.confirmation_count,
            distinct_contexts = entry.distinct_contexts.len(),
            "knowledge tier progression"
        );
    }
    true
}

#[cfg(test)]
mod tests {
    use roko_learn::episode_logger::EpisodeGateVerdict;
    use roko_learn::guarded_commit::{CommitDecision, GuardMode, Proposer};

    use super::*;
    use crate::knowledge_store::commit::{
        Delivery, KNOWLEDGE_STORE, KnowledgeChecks, propose_batch, recover_orphaned_batches,
    };
    use crate::{
        KnowledgeKind, KnowledgeTier, RuntimeEpisodeObservation, RuntimeKnowledgeLifecycle,
        SourceChannel,
    };

    /// Undertow's phase 5 lesson on preset loudness, as an earlier run
    /// committed it.
    fn committed_lesson() -> KnowledgeEntry {
        let episode = "1f677933-09aa-4a47-8135-7128e580993f:p5-ship:T1:3";
        KnowledgeEntry {
            id: "kn-loudness".to_string(),
            kind: KnowledgeKind::StrategyFragment,
            content: "In this repo, preset-level loudness matching is enforced by spec, so even when \
                      the preset list/API is correct, `master_gain` often needs per-preset trimming \
                      to land near -18 dBFS RMS."
                .to_string(),
            tags: [
                "core/include/undertow/presets.h",
                "core/src/presets.cpp",
                "plan:p5-ship",
                "strategy_fragment",
                "runtime",
                "gate:passed",
                "lesson",
                "core",
            ]
            .map(str::to_string)
            .to_vec(),
            source: Some("runtime:gate_verdict".to_string()),
            source_episodes: vec![episode.to_string()],
            confirmation_count: 1,
            distinct_contexts: vec![format!("p5-ship:T1:{episode}")],
            tier: KnowledgeTier::Working,
            ..KnowledgeEntry::default()
        }
    }

    /// Phase 5.1's T1 lesson, which repeats [`committed_lesson`] in other
    /// words.
    const REPEAT: &str = "in this repo, preset spectrum fixes on the final filter often need a \
                          paired master_gain trim to keep the loudness check passing.";

    /// A verified pass of phase 5.1's `task` whose prompt surfaced
    /// `surfaced` and whose agent stated `lesson`.
    fn pass(
        episode: &str,
        task: &str,
        lesson: Option<&str>,
        surfaced: &[&str],
    ) -> RuntimeEpisodeObservation {
        RuntimeEpisodeObservation {
            episode_id: episode.to_string(),
            task_id: task.to_string(),
            plan_id: Some("p5-1-the-renders-as-they-play".to_string()),
            task_type: Some("focused".to_string()),
            model: Some("gpt-5.4-mini".to_string()),
            agent_id: Some("implementer".to_string()),
            gate_passed: true,
            gate_verdicts: vec![EpisodeGateVerdict::new("verify[0:test]", true)],
            gate_output: "verify[0:test] `tools/spec.sh 5 presets` passed".to_string(),
            agent_output: String::new(),
            lesson: lesson.map(str::to_string),
            context_entry_ids: surfaced.iter().map(|id| (*id).to_string()).collect(),
            task_tags: vec![
                "core/src/presets.cpp".to_string(),
                "plan:p5-1-the-renders-as-they-play".to_string(),
            ],
            source_channel: SourceChannel::GateVerdict,
            observed_at: Utc::now(),
        }
    }

    /// The store's entry `id`.
    fn read(store: &KnowledgeStore, id: &str) -> KnowledgeEntry {
        let entries = store.read_all().expect("read the store");
        entries
            .into_iter()
            .find(|entry| entry.id == id)
            .expect("the entry")
    }

    /// What confirmations changed in `entry`: its count, contexts, tier and
    /// confidence. Reinforcement, its balance, is no confirmation.
    fn confirmations(entry: &KnowledgeEntry) -> (u32, Vec<String>, KnowledgeTier, String) {
        let confidence = format!("{:.6}", entry.confidence);
        let contexts = entry.distinct_contexts.clone();
        (entry.confirmation_count, contexts, entry.tier, confidence)
    }

    /// Propose `batch` for a run that ended as `delivery` says.
    fn settle(
        store: &KnowledgeStore,
        learn_dir: &std::path::Path,
        batch: &str,
        delivery: Delivery,
    ) -> Option<CommitDecision> {
        let checks = KnowledgeChecks::new(store, batch, Vec::new(), delivery);
        let proposer = Proposer::evolved("knowledge", KNOWLEDGE_STORE, batch);
        propose_batch(
            store,
            batch,
            learn_dir,
            GuardMode::Observe,
            &checks,
            &proposer,
        )
        .expect("proposed")
    }

    /// gap-1220c5: a guarded run's pass that surfaced a committed lesson, and
    /// its own lesson that repeats it, confirm that lesson only when the run
    /// delivers. Until the run's end both wait beside its batch; an
    /// undelivered run's commit drops them, and a delivered run's applies
    /// them as one confirmation, the repeat's, with its record.
    #[test]
    fn an_undelivered_run_confirms_nothing() {
        for delivered in [false, true] {
            let temp = tempfile::tempdir().expect("tempdir");
            let roko_dir = temp.path().join(".roko");
            let learn_dir = roko_dir.join("learn");
            let store = KnowledgeStore::for_roko_dir(&roko_dir);
            store
                .rewrite_all(&[committed_lesson()])
                .expect("seed the store");
            let before = read(&store, "kn-loudness");
            let batch = if delivered {
                "run-1220c5-delivered"
            } else {
                "run-1220c5-cancelled"
            };
            let lifecycle =
                RuntimeKnowledgeLifecycle::for_roko_dir(&roko_dir).with_commit_batch(batch);
            let episode = format!("{batch}:p5-1-the-renders-as-they-play:T1:1");
            let record = lifecycle
                .ingest_observation(pass(&episode, "T1", Some(REPEAT), &["kn-loudness"]))
                .expect("T1#1");
            assert!(record.candidate_entry_id.is_some(), "{record:?}");

            let held = read(&store, "kn-loudness");
            assert_eq!(held.confirmation_count, before.confirmation_count);
            assert_eq!(held.distinct_contexts, before.distinct_contexts);
            assert_eq!(held.tier, before.tier);
            let pending = store.pending_confirmations().expect("the ledger");
            assert_eq!(pending.len(), 2, "{pending:?}");
            assert!(
                pending
                    .iter()
                    .all(|row| row.batch == batch && row.entry == "kn-loudness")
            );
            assert!(store.read_confirmations().expect("records").is_empty());

            let delivery = if delivered {
                Delivery::Delivered("the run delivered plan p5-1".to_string())
            } else {
                Delivery::Undelivered(
                    "plan p5-1 interrupted; a cancel request stopped the run".to_string(),
                )
            };
            let decision = settle(&store, &learn_dir, batch, delivery);
            let after = read(&store, "kn-loudness");
            if delivered {
                assert_eq!(decision, Some(CommitDecision::Committed));
                assert_eq!(after.confirmation_count, before.confirmation_count + 1);
                assert_eq!(
                    after.distinct_contexts.len(),
                    before.distinct_contexts.len() + 1
                );
                let records = store.read_confirmations().expect("records");
                assert_eq!(records.len(), 1, "the repeat's record: {records:?}");
                assert_eq!(records[0].confirmed_entry_id, "kn-loudness");
            } else {
                assert_eq!(decision, Some(CommitDecision::RolledBack));
                assert_eq!(after.confirmation_count, before.confirmation_count);
                assert_eq!(after.distinct_contexts, before.distinct_contexts);
                assert!(store.read_confirmations().expect("records").is_empty());
            }
            assert!(
                store
                    .pending_confirmations()
                    .expect("the ledger")
                    .is_empty()
            );
        }
    }

    /// gap-1220c5 with bug-b7f6a8's resume: a run that dies leaves its held
    /// confirmations behind, and the resumed run, a new batch, rolls the dead
    /// run's batch back at its start with them. The resumed run applies only
    /// its own when it delivers, once though one pass reached the sink twice,
    /// and a second settle applies nothing more.
    #[test]
    fn a_resumed_run_applies_its_pending_confirmations_once() {
        let temp = tempfile::tempdir().expect("tempdir");
        let roko_dir = temp.path().join(".roko");
        let learn_dir = roko_dir.join("learn");
        let store = KnowledgeStore::for_roko_dir(&roko_dir);
        store
            .rewrite_all(&[committed_lesson()])
            .expect("seed the store");
        let before = read(&store, "kn-loudness");
        let lifecycle = RuntimeKnowledgeLifecycle::for_roko_dir(&roko_dir);

        // The first run's T1#1 surfaced the lesson and passed; then its
        // process died.
        let first = lifecycle.clone().with_commit_batch("run-1220c5-first");
        let died = pass("run-1220c5-first:p5-1:T1:1", "T1", None, &["kn-loudness"]);
        first.ingest_observation(died).expect("T1#1");
        let pending = store.pending_confirmations().expect("the ledger");
        assert_eq!(pending.len(), 1, "{pending:?}");

        // The resume is a new batch. Its start rolls the dead run's batch
        // back, its held confirmation with it.
        let recovered = recover_orphaned_batches(
            &store,
            Some("run-1220c5-resumed"),
            &learn_dir,
            GuardMode::Observe,
            &[],
        )
        .expect("recovered");
        assert_eq!(recovered, [("run-1220c5-first".to_string(), None)]);
        assert!(
            store
                .pending_confirmations()
                .expect("the ledger")
                .is_empty()
        );
        assert_eq!(
            confirmations(&read(&store, "kn-loudness")),
            confirmations(&before)
        );

        // The resumed run's T2#1 surfaced the lesson and passed, and the same
        // pass reached the sink twice. The run delivers.
        let resumed = lifecycle.with_commit_batch("run-1220c5-resumed");
        let again = pass("run-1220c5-resumed:p5-1:T2:1", "T2", None, &["kn-loudness"]);
        resumed.ingest_observation(again.clone()).expect("T2#1");
        resumed.ingest_observation(again).expect("T2#1 again");
        let delivered = || Delivery::Delivered("the run delivered plan p5-1".to_string());
        assert_eq!(
            settle(&store, &learn_dir, "run-1220c5-resumed", delivered()),
            None
        );
        let after = read(&store, "kn-loudness");
        assert_eq!(after.confirmation_count, before.confirmation_count + 1);
        assert_eq!(
            after.distinct_contexts.len(),
            before.distinct_contexts.len() + 1
        );
        assert!(
            store
                .pending_confirmations()
                .expect("the ledger")
                .is_empty()
        );

        assert_eq!(
            settle(&store, &learn_dir, "run-1220c5-resumed", delivered()),
            None
        );
        assert_eq!(
            confirmations(&read(&store, "kn-loudness")),
            confirmations(&after)
        );
    }

    /// gap-1220c5: one pass that surfaces a committed lesson and restates it
    /// confirms it once, not once per path, and writes one record, the
    /// repeat's. An unguarded run confirms per pass; a guarded run confirms
    /// an entry at most once per batch, whatever its passes did.
    #[test]
    fn a_pass_that_surfaces_and_restates_an_entry_confirms_it_once() {
        let transient = || KnowledgeEntry {
            confirmation_count: 0,
            distinct_contexts: Vec::new(),
            tier: KnowledgeTier::Transient,
            ..committed_lesson()
        };

        // Unguarded: each pass confirms the lesson once.
        let temp = tempfile::tempdir().expect("tempdir");
        let roko_dir = temp.path().join(".roko");
        let store = KnowledgeStore::for_roko_dir(&roko_dir);
        store.rewrite_all(&[transient()]).expect("seed the store");
        let lifecycle = RuntimeKnowledgeLifecycle::for_roko_dir(&roko_dir);
        let first = pass("off:p5-1:T1:1", "T1", Some(REPEAT), &["kn-loudness"]);
        lifecycle.ingest_observation(first).expect("T1#1");
        let once = read(&store, "kn-loudness");
        assert_eq!(once.confirmation_count, 1, "{once:?}");
        assert_eq!(once.distinct_contexts.len(), 1, "{once:?}");
        assert_eq!(store.read_confirmations().expect("records").len(), 1);
        let second = pass("off:p5-1:T2:1", "T2", None, &["kn-loudness"]);
        lifecycle.ingest_observation(second).expect("T2#1");
        assert_eq!(read(&store, "kn-loudness").confirmation_count, 2);
        assert_eq!(store.read_confirmations().expect("records").len(), 1);

        // Guarded: the run's two passes confirm it once, when it delivers.
        let temp = tempfile::tempdir().expect("tempdir");
        let roko_dir = temp.path().join(".roko");
        let learn_dir = roko_dir.join("learn");
        let store = KnowledgeStore::for_roko_dir(&roko_dir);
        store.rewrite_all(&[transient()]).expect("seed the store");
        let batch = "run-1220c5-once";
        let lifecycle = RuntimeKnowledgeLifecycle::for_roko_dir(&roko_dir).with_commit_batch(batch);
        let first = pass("run:p5-1:T1:1", "T1", Some(REPEAT), &["kn-loudness"]);
        lifecycle.ingest_observation(first).expect("T1#1");
        let second = pass("run:p5-1:T2:1", "T2", None, &["kn-loudness"]);
        lifecycle.ingest_observation(second).expect("T2#1");
        assert_eq!(store.pending_confirmations().expect("the ledger").len(), 3);
        let delivered = Delivery::Delivered("the run delivered plan p5-1".to_string());
        assert_eq!(
            settle(&store, &learn_dir, batch, delivered),
            Some(CommitDecision::Committed)
        );
        let after = read(&store, "kn-loudness");
        assert_eq!(after.confirmation_count, 1, "{after:?}");
        assert_eq!(after.distinct_contexts.len(), 1, "{after:?}");
        assert_eq!(store.read_confirmations().expect("records").len(), 1);
    }
}
