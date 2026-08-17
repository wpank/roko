//! knowledge command handlers.
#![allow(unused_imports)]

use crate::*;
use anyhow::ensure;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub(crate) async fn dispatch_knowledge(cli: &Cli, cmd: KnowledgeCmd) -> Result<i32> {
    match cmd {
        KnowledgeCmd::Query { topic, workdir } => {
            cmd_neuro(cli, NeuroCmd::Query { topic, workdir }).await
        }
        KnowledgeCmd::Stats { workdir } => cmd_neuro(cli, NeuroCmd::Stats { workdir }).await,
        KnowledgeCmd::Gc { workdir } => cmd_neuro(cli, NeuroCmd::Gc { workdir }).await,
        KnowledgeCmd::Export {
            workdir,
            output,
            force,
            top_n,
        } => {
            cmd_neuro(
                cli,
                NeuroCmd::Export {
                    workdir,
                    output,
                    force,
                    top_n,
                },
            )
            .await
        }
        KnowledgeCmd::Import {
            workdir,
            input,
            decay_factor,
            legacy_raw,
        } => {
            cmd_neuro(
                cli,
                NeuroCmd::Import {
                    workdir,
                    input,
                    decay_factor,
                    legacy_raw,
                },
            )
            .await
        }
        KnowledgeCmd::Backup {
            workdir,
            destination,
            force,
            top_n,
        } => {
            cmd_neuro(
                cli,
                NeuroCmd::Backup {
                    workdir,
                    destination,
                    force,
                    top_n,
                },
            )
            .await
        }
        KnowledgeCmd::Restore {
            workdir,
            source,
            force,
            types,
            min_confidence,
            generation,
            decay_factor,
            legacy_raw,
        } => {
            cmd_neuro(
                cli,
                NeuroCmd::Restore {
                    workdir,
                    source,
                    force,
                    types,
                    min_confidence,
                    generation,
                    decay_factor,
                    legacy_raw,
                },
            )
            .await
        }
        KnowledgeCmd::Sync {
            peer,
            workdir,
            direction,
            max_send,
        } => {
            cmd_neuro(
                cli,
                NeuroCmd::Sync {
                    peer,
                    workdir,
                    direction,
                    max_send,
                },
            )
            .await
        }
        KnowledgeCmd::Dream { cmd } => dispatch_knowledge_dream(cli, cmd).await,
        KnowledgeCmd::Custody { cmd } => {
            dispatch_knowledge_custody(cli, cmd)?;
            Ok(EXIT_SUCCESS)
        }
        KnowledgeCmd::Archive {
            older_than,
            batch_size,
            workdir,
            dry_run,
        } => cmd_archive(cli, workdir, &older_than, batch_size, dry_run).await,
        KnowledgeCmd::BackfillHdc { workdir } => cmd_backfill_hdc(cli, workdir).await,
    }
}

async fn cmd_backfill_hdc(cli: &Cli, workdir: Option<PathBuf>) -> Result<i32> {
    let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
    let store = KnowledgeStore::for_workdir(&wd);

    let changed = store.backfill_hdc_vectors().with_context(|| {
        format!(
            "backfill HDC vectors in knowledge store at {}",
            store.path().display()
        )
    })?;

    if cli.json {
        let payload = serde_json::json!({
            "workdir": wd,
            "path": store.path(),
            "updated": changed,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(EXIT_SUCCESS);
    }

    if changed == 0 {
        println!(
            "backfill-hdc: all entries already have HDC vectors in {}",
            store.path().display()
        );
    } else {
        println!(
            "backfill-hdc: populated HDC vectors for {changed} entr{} in {}",
            if changed == 1 { "y" } else { "ies" },
            store.path().display()
        );
    }
    Ok(EXIT_SUCCESS)
}

pub(crate) async fn dispatch_knowledge_dream(cli: &Cli, cmd: KnowledgeDreamCmd) -> Result<i32> {
    match cmd {
        KnowledgeDreamCmd::Run { workdir } => cmd_dream(cli, DreamCmdLegacy::Run { workdir }).await,
        KnowledgeDreamCmd::Report { workdir } => {
            cmd_dream(cli, DreamCmdLegacy::Report { workdir }).await
        }
        KnowledgeDreamCmd::Schedule { workdir } => {
            cmd_dream(cli, DreamCmdLegacy::Schedule { workdir }).await
        }
        KnowledgeDreamCmd::Journal { limit, workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let journal = roko_dreams::phase2::DreamJournal::standard(&wd);
            match journal.read_recent(limit) {
                Ok(entries) if entries.is_empty() => println!("no dream journal entries found"),
                Ok(entries) => {
                    for entry in &entries {
                        println!(
                            "[{}] cycle={} agent={} hypotheses={}/{}/{} tokens={} {}",
                            entry.cycle_start.format("%Y-%m-%d %H:%M"),
                            entry.cycle_id,
                            entry.agent_id,
                            entry.hypotheses_generated,
                            entry.hypotheses_staged,
                            entry.hypotheses_promoted,
                            entry.total_tokens,
                            if entry.early_termination {
                                "(early termination)"
                            } else {
                                ""
                            },
                        );
                    }
                    println!("\n{} entries shown (of last {})", entries.len(), limit);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    println!(
                        "no dream journal found at {}",
                        journal.journal_path.display()
                    );
                }
                Err(e) => return Err(e.into()),
            }
            Ok(EXIT_SUCCESS)
        }
        KnowledgeDreamCmd::Archive { limit, workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let archive = roko_dreams::phase2::DreamArchive::standard(&wd);
            match archive.read_recent(limit) {
                Ok(entries) if entries.is_empty() => println!("no dream archive entries found"),
                Ok(entries) => {
                    for entry in &entries {
                        println!(
                            "[{}] {} ({:?}) quality={:.2} -- {}",
                            entry.archived_at.format("%Y-%m-%d %H:%M"),
                            entry.entry_id,
                            entry.kind,
                            entry.quality_score,
                            entry.summary,
                        );
                    }
                    println!("\n{} entries shown (of last {})", entries.len(), limit);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    println!(
                        "no dream archive found at {}",
                        archive.archive_path.display()
                    );
                }
                Err(e) => return Err(e.into()),
            }
            Ok(EXIT_SUCCESS)
        }
    }
}

pub(crate) fn dispatch_knowledge_custody(cli: &Cli, cmd: KnowledgeCustodyCmd) -> Result<()> {
    match cmd {
        KnowledgeCustodyCmd::List { limit, workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            roko_cli::custody::cmd_custody_list(&wd, limit)?;
        }
        KnowledgeCustodyCmd::Show { index, workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            roko_cli::custody::cmd_custody_show(&wd, index)?;
        }
        KnowledgeCustodyCmd::Verify { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            roko_cli::custody::cmd_custody_verify(&wd)?;
        }
    }
    Ok(())
}

pub(crate) async fn cmd_archive(
    cli: &Cli,
    workdir: Option<PathBuf>,
    older_than: &str,
    batch_size: usize,
    dry_run: bool,
) -> Result<i32> {
    let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
    let roko_dir = wd.join(".roko");
    if !roko_dir.exists() {
        bail!("no .roko/ directory found in {}", wd.display());
    }

    // Parse duration string (e.g. "30d", "7d", "24h").
    let max_age_ms = parse_duration_to_ms(older_than)
        .ok_or_else(|| anyhow!("invalid duration: {older_than} (expected e.g. '30d' or '7d')"))?;

    let cutoff_ms = chrono::Utc::now().timestamp_millis() - max_age_ms;

    // Open the hot substrate.
    let hot = roko_fs::FileSubstrate::open(&roko_dir).await?;

    // Query for old engrams.
    use roko_core::{Context, Query, Store};
    let ctx = Context::now();
    let query = Query::all().until(cutoff_ms).limit(batch_size);
    let candidates = hot.query(&query, &ctx).await?;

    if candidates.is_empty() {
        println!("no engrams older than {older_than} found");
        return Ok(EXIT_SUCCESS);
    }

    println!(
        "found {} engram(s) older than {older_than}{}",
        candidates.len(),
        if dry_run { " (dry run)" } else { "" }
    );

    if dry_run {
        for e in &candidates {
            let age_days = (chrono::Utc::now().timestamp_millis() - e.created_at_ms) / 86_400_000;
            println!("  {:?} | {} | {}d old", e.kind, &e.id, age_days);
        }
        return Ok(EXIT_SUCCESS);
    }

    // Confirm destructive operation (skipped in quiet / non-TTY mode).
    let prompt_msg = format!(
        "Archive {} engram(s) older than {older_than}?",
        candidates.len()
    );
    if !confirm_destructive(&prompt_msg, cli.quiet) {
        println!("aborted");
        return Ok(EXIT_SUCCESS);
    }

    // Collect IDs before moving candidates into archive_batch.
    let candidate_ids: Vec<roko_core::ContentHash> = candidates.iter().map(|e| e.id).collect();

    // Open cold substrate and archive (dedup-safe: skips already-archived).
    let cold_dir = roko_dir.join("cold");
    let cold = roko_fs::ArchiveColdSubstrate::open(&cold_dir).await?;

    use roko_core::ColdStore;
    let archived = cold.archive_batch(candidates).await?;

    // Prune archived engrams from the hot store and compact the log so
    // they are not re-archived on subsequent runs.
    if !candidate_ids.is_empty() {
        let removed = hot.remove_ids(&candidate_ids);
        hot.compact().await?;
        println!("pruned {removed} engram(s) from hot store");
    }

    println!("archived {archived} engram(s) to {}", cold_dir.display());

    Ok(EXIT_SUCCESS)
}

/// Parse a human duration string like "30d" or "7d" or "24h" to milliseconds.
pub(crate) fn parse_duration_to_ms(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (num_str, unit) = s.split_at(s.len() - 1);
    let num: i64 = num_str.parse().ok()?;
    match unit {
        "d" => Some(num * 24 * 3600 * 1000),
        "h" => Some(num * 3600 * 1000),
        "m" => Some(num * 60 * 1000),
        "s" => Some(num * 1000),
        _ => None,
    }
}

pub(crate) async fn cmd_neuro(cli: &Cli, cmd: NeuroCmd) -> Result<i32> {
    match cmd {
        NeuroCmd::Query { topic, workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let topic = topic.join(" ");
            let topic = topic.trim().to_string();
            if topic.is_empty() {
                anyhow::bail!("provide a topic to query");
            }

            let store = KnowledgeStore::for_workdir(&wd);
            let entries = store.query(&topic, 10).with_context(|| {
                format!(
                    "query knowledge store at {} for topic '{topic}'",
                    store.path().display()
                )
            })?;

            if cli.json {
                let payload = serde_json::json!({
                    "workdir": wd,
                    "topic": topic,
                    "count": entries.len(),
                    "entries": entries,
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(EXIT_SUCCESS);
            }

            println!(
                "Knowledge matches for '{topic}' in {}:",
                store.path().display()
            );
            if entries.is_empty() {
                println!("  (no matches)");
                return Ok(EXIT_SUCCESS);
            }

            for (idx, entry) in entries.iter().enumerate() {
                println!(
                    "{}. [{}] confidence {:.2} {}",
                    idx + 1,
                    format!("{:?}", entry.kind).to_lowercase(),
                    entry.confidence.clamp(0.0, 1.0),
                    entry.content.trim()
                );
                if !entry.tags.is_empty() {
                    println!("   tags: {}", entry.tags.join(", "));
                }
                if !entry.source_episodes.is_empty() {
                    println!("   sources: {}", entry.source_episodes.join(", "));
                }
            }

            Ok(EXIT_SUCCESS)
        }
        NeuroCmd::Stats { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let store = KnowledgeStore::for_workdir(&wd);
            let stats = store.stats().with_context(|| {
                format!("read knowledge store stats from {}", store.path().display())
            })?;

            if cli.json {
                let payload = serde_json::json!({
                    "workdir": wd,
                    "path": store.path(),
                    "stats": stats,
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(EXIT_SUCCESS);
            }

            println!("Knowledge stats for {}:", store.path().display());
            println!("  total entries: {}", stats.total_entries);
            println!("  anti-knowledge: {}", stats.anti_knowledge_count);
            println!(
                "  average confidence: {}",
                stats
                    .average_confidence
                    .map(|confidence| format!("{confidence:.3}"))
                    .unwrap_or_else(|| "n/a".to_owned())
            );
            println!("  entries by kind:");
            if stats.kind_counts.is_empty() {
                println!("    (empty)");
            } else {
                for (kind, count) in &stats.kind_counts {
                    println!("    {kind:<20} {count}");
                }
            }
            println!("  entries by tier:");
            if stats.tier_counts.is_empty() {
                println!("    (empty)");
            } else {
                for (tier, count) in &stats.tier_counts {
                    println!("    {tier:<20} {count}");
                }
            }
            if !stats.source_counts.is_empty() {
                println!("  entries by source:");
                for (source, count) in &stats.source_counts {
                    println!("    {source:<20} {count}");
                }
            }

            match stats.oldest_entry.as_ref() {
                Some(entry) => {
                    println!(
                        "  oldest entry: {} [{}] confidence {:.3} created {}",
                        entry.id,
                        format!("{:?}", entry.kind).to_lowercase(),
                        entry.confidence.clamp(0.0, 1.0),
                        entry.created_at
                    );
                }
                None => println!("  oldest entry: (none)"),
            }

            match stats.newest_entry.as_ref() {
                Some(entry) => {
                    println!(
                        "  newest entry: {} [{}] confidence {:.3} created {}",
                        entry.id,
                        format!("{:?}", entry.kind).to_lowercase(),
                        entry.confidence.clamp(0.0, 1.0),
                        entry.created_at
                    );
                }
                None => println!("  newest entry: (none)"),
            }

            Ok(EXIT_SUCCESS)
        }
        NeuroCmd::Gc { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let store = KnowledgeStore::for_workdir(&wd);
            let before = store.stats().with_context(|| {
                format!("read knowledge store stats from {}", store.path().display())
            })?;
            store.gc(DEFAULT_GC_MIN_CONFIDENCE).with_context(|| {
                format!(
                    "garbage collect knowledge store at {}",
                    store.path().display()
                )
            })?;
            let after = store.stats().with_context(|| {
                format!(
                    "read knowledge store stats from {} after gc",
                    store.path().display()
                )
            })?;
            let removed = before.total_entries.saturating_sub(after.total_entries);

            if cli.json {
                let payload = serde_json::json!({
                    "workdir": wd,
                    "path": store.path(),
                    "threshold": DEFAULT_GC_MIN_CONFIDENCE,
                    "before": before.total_entries,
                    "after": after.total_entries,
                    "removed": removed,
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(EXIT_SUCCESS);
            }

            println!("Knowledge GC for {}:", store.path().display());
            println!("  threshold: {:.3}", DEFAULT_GC_MIN_CONFIDENCE);
            println!("  before: {}", before.total_entries);
            println!("  after: {}", after.total_entries);
            println!("  removed entries: {}", removed);

            Ok(EXIT_SUCCESS)
        }
        NeuroCmd::Export {
            workdir,
            output,
            force,
            top_n,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            if output.exists() && !force {
                bail!(
                    "knowledge export would overwrite {}. Re-run with --force to replace it.",
                    output.display()
                );
            }
            let store = KnowledgeStore::for_workdir(&wd);
            let filter = ExportFilter {
                max_entries: top_n,
                filter_secrets: true,
                ..Default::default()
            };
            let exported = store.export(&output, &filter)?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "workdir": wd,
                        "output": output,
                        "entries_exported": exported,
                        "secret_filtering": true,
                        "top_n": top_n,
                    }))?
                );
            } else {
                println!(
                    "Exported {exported} secret-safe knowledge entries to {}",
                    output.display()
                );
            }
            Ok(EXIT_SUCCESS)
        }
        NeuroCmd::Import {
            workdir,
            input,
            decay_factor,
            legacy_raw,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let store = KnowledgeStore::for_workdir(&wd);
            let result = store.import(
                &input,
                &ImportOptions {
                    confidence_discount: decay_factor,
                    source_label: "knowledge-import".to_owned(),
                    allow_legacy: legacy_raw,
                    ..Default::default()
                },
            )?;
            print_import_result(cli, &input, decay_factor, &result)?;
            Ok(EXIT_SUCCESS)
        }
        NeuroCmd::Backup {
            workdir,
            destination,
            force,
            top_n,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let report = backup_neuro_store(&wd, &destination, force, top_n)?;

            if cli.json {
                let payload = serde_json::json!({
                    "workdir": wd,
                    "backup_dir": destination,
                    "knowledge_store": report.live.knowledge,
                    "knowledge_backup": report.snapshot.knowledge,
                    "confirmations_store": report.live.confirmations,
                    "confirmations_backup": report.snapshot.confirmations,
                    "confirmations_present": report.confirmations_present,
                    "manifest": report.manifest,
                    "top_n": top_n,
                    "entries_exported": report.entries_exported,
                    "force": force,
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(EXIT_SUCCESS);
            }

            println!("Neuro backup written to {}:", destination.display());
            println!("  knowledge: {}", report.snapshot.knowledge.display());
            if let Some(n) = top_n {
                println!("  genomic bottleneck: top {n} entries by confidence");
            }
            println!("  entries exported: {}", report.entries_exported);
            if report.confirmations_present {
                println!(
                    "  confirmations: {}",
                    report.snapshot.confirmations.display()
                );
            } else {
                println!("  confirmations: (none)");
            }

            println!("  manifest: {}", report.manifest.display());

            Ok(EXIT_SUCCESS)
        }
        NeuroCmd::Restore {
            workdir,
            source,
            force,
            types,
            min_confidence,
            generation,
            decay_factor,
            legacy_raw,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));

            // Parse type filters if provided.
            let type_filters: Option<Vec<String>> = types.map(|t| {
                t.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            });

            let report = restore_neuro_store(
                &wd,
                &source,
                force,
                generation,
                decay_factor,
                min_confidence,
                type_filters.as_deref(),
                legacy_raw,
            )?;

            let confidence_decay = decay_factor.powf(f64::from(generation));

            if cli.json {
                let payload = serde_json::json!({
                    "workdir": wd,
                    "backup_dir": source,
                    "knowledge_store": report.live.knowledge,
                    "knowledge_backup": report.snapshot.knowledge,
                    "confirmations_store": report.live.confirmations,
                    "confirmations_backup": report.snapshot.confirmations,
                    "confirmations_present": report.confirmations_present,
                    "manifest": report.manifest,
                    "generation": generation,
                    "confidence_decay": confidence_decay,
                    "entries_restored": report.entries_restored,
                    "entries_filtered": report.entries_filtered,
                    "entries_skipped_dedup": report.entries_skipped_dedup,
                    "entries_skipped_contradiction": report.entries_skipped_contradiction,
                    "malformed_entries": report.malformed_entries,
                    "legacy_input": report.legacy_input,
                    "force": force,
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(EXIT_SUCCESS);
            }

            println!("Neuro backup restored from {}:", source.display());
            println!("  knowledge: {}", report.live.knowledge.display());
            println!("  generation: {generation} (confidence decay: {confidence_decay:.4})");
            println!("  entries restored: {}", report.entries_restored);
            println!("  entries filtered: {}", report.entries_filtered);
            println!(
                "  entries skipped (dedup): {}",
                report.entries_skipped_dedup
            );
            println!(
                "  entries skipped (contradiction): {}",
                report.entries_skipped_contradiction
            );
            println!("  tier: all restored entries set to Transient (quarantine)");
            if report.confirmations_present {
                println!("  confirmations: {}", report.live.confirmations.display());
            } else {
                println!("  confirmations: (none)");
            }

            Ok(EXIT_SUCCESS)
        }
        NeuroCmd::Sync {
            peer,
            workdir,
            direction,
            max_send,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let store = KnowledgeStore::for_workdir(&wd);

            // Load the version vector from persistent state (or create empty).
            let vv_path = wd.join(".roko").join("neuro").join("version-vectors.json");
            let mut version_vectors: HashMap<String, u64> = if vv_path.exists() {
                let text = std::fs::read_to_string(&vv_path)
                    .with_context(|| format!("read version vectors from {}", vv_path.display()))?;
                serde_json::from_str(&text).unwrap_or_default()
            } else {
                HashMap::new()
            };

            let peer_seq = version_vectors.get(&peer).copied().unwrap_or(0);
            let entries = store
                .read_all()
                .with_context(|| format!("read knowledge store from {}", store.path().display()))?;

            let should_send = direction == "send" || direction == "both";
            let should_receive = direction == "receive" || direction == "both";

            let mut sent_count = 0_usize;
            let mut received_count = 0_usize;

            if should_send {
                // Build delta: entries newer than peer's last-seen sequence.
                // Use entry index as a proxy sequence number for local ordering.
                let delta: Vec<_> = entries
                    .iter()
                    .enumerate()
                    .filter(|(idx, _)| (*idx as u64) > peer_seq)
                    .take(max_send)
                    .collect();
                sent_count = delta.len();

                // Write delta to an outbox file for the peer.
                if !delta.is_empty() {
                    let outbox_dir = wd.join(".roko").join("mesh").join("outbox");
                    std::fs::create_dir_all(&outbox_dir)?;
                    let delta_path = outbox_dir.join(format!("delta-{peer}.jsonl"));
                    let mut f = std::fs::OpenOptions::new()
                        .create(true)
                        .truncate(true)
                        .write(true)
                        .open(&delta_path)?;
                    for (_, entry) in &delta {
                        let line = serde_json::to_string(entry)?;
                        use std::io::Write;
                        writeln!(f, "{line}")?;
                    }
                    println!("  outbox: {}", delta_path.display());
                }
            }

            if should_receive {
                // Check inbox for incoming deltas from the peer.
                let inbox_dir = wd.join(".roko").join("mesh").join("inbox");
                let inbox_path = inbox_dir.join(format!("delta-{peer}.jsonl"));
                if inbox_path.exists() {
                    let text = std::fs::read_to_string(&inbox_path)?;
                    let mut imported = Vec::new();
                    for line in text.lines() {
                        if line.trim().is_empty() {
                            continue;
                        }
                        if let Ok(mut entry) =
                            serde_json::from_str::<roko_neuro::KnowledgeEntry>(line)
                        {
                            // Apply received confidence discount (0.7x).
                            entry.confidence *= 0.7;
                            entry.tier = roko_neuro::KnowledgeTier::Transient;
                            entry.source = Some(format!("mesh:{peer}"));
                            imported.push(entry);
                        }
                    }
                    received_count = imported.len();
                    if !imported.is_empty() {
                        store.ingest(imported).with_context(|| {
                            format!("import mesh entries from {}", inbox_path.display())
                        })?;
                    }
                    // Clean up processed inbox file.
                    let _ = std::fs::remove_file(&inbox_path);
                }
            }

            // Update version vector for this peer.
            let new_seq = entries.len() as u64;
            version_vectors.insert(peer.clone(), new_seq);
            if let Some(parent) = vv_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&vv_path, serde_json::to_string_pretty(&version_vectors)?)?;

            if cli.json {
                let payload = serde_json::json!({
                    "peer": peer,
                    "direction": direction,
                    "sent": sent_count,
                    "received": received_count,
                    "local_seq": new_seq,
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(EXIT_SUCCESS);
            }

            println!("Mesh sync with peer '{peer}':");
            println!("  direction: {direction}");
            println!("  sent: {sent_count} engrams");
            println!("  received: {received_count} engrams (0.7x confidence discount)");
            println!("  local sequence: {new_seq}");

            Ok(EXIT_SUCCESS)
        }
    }
}

pub(crate) async fn cmd_dream(cli: &Cli, cmd: DreamCmdLegacy) -> Result<i32> {
    match cmd {
        DreamCmdLegacy::Run { workdir } => {
            let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
            prepare_runtime_hooks(&workdir, cli.quiet);

            let mut runner = build_dream_runner(cli, &workdir)?;
            let report = match runner.consolidate_now() {
                Ok(report) => report,
                Err(e) => {
                    // Appraise dream failure into the daimon affect state.
                    use roko_daimon::{AffectEngine as _, AffectEvent, DaimonState};
                    let daimon_path = workdir.join(".roko").join("daimon").join("affect.json");
                    let mut daimon = DaimonState::load_or_new(&daimon_path);
                    let _ = daimon.appraise(AffectEvent::DreamFailure {
                        task_type: "consolidation".to_string(),
                        failure_count: 1,
                    });
                    return Err(e);
                }
            };
            let cfactor_snapshot = refresh_cfactor_snapshot(workdir.join(".roko").join("learn"))
                .await
                .map_err(|e| anyhow!("refresh c-factor snapshot: {e}"))?;

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else if !cli.quiet {
                println!(
                    "dream cycle completed: {} episodes, {} clusters, {} knowledge entries, {} playbooks",
                    report.processed_episodes,
                    report.clusters.len(),
                    report.knowledge_entries_written,
                    report.playbooks_created
                );
                if let Some(processed_through) = report.processed_through {
                    println!("processed through: {processed_through}");
                }
                println!(
                    "report saved under: {}",
                    workdir.join(".roko").join("dreams").display()
                );
                println!("c-factor: {:.3}", cfactor_snapshot.overall);
            }

            Ok(EXIT_SUCCESS)
        }
        DreamCmdLegacy::Report { workdir } => {
            let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let runner = build_dream_runner(cli, &workdir)?;
            let report = runner.latest_report()?.ok_or_else(|| {
                anyhow!(
                    "no dream report found in {}",
                    workdir.join(".roko").join("dreams").display()
                )
            })?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "dream report: {} episodes, {} clusters, {} knowledge entries, {} playbooks",
                    report.processed_episodes,
                    report.clusters.len(),
                    report.knowledge_entries_written,
                    report.playbooks_created
                );
                println!("started: {}", report.started_at);
                println!("completed: {}", report.completed_at);
                if let Some(processed_through) = report.processed_through {
                    println!("processed through: {processed_through}");
                }
            }
            Ok(EXIT_SUCCESS)
        }
        DreamCmdLegacy::Schedule { workdir } => {
            let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let runner = build_dream_runner(cli, &workdir)?;
            let schedule = runner.schedule_next();
            if cli.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "next_fire_seconds": schedule.map(|duration| duration.as_secs())
                    })
                );
            } else if let Some(duration) = schedule {
                println!("next dream in {:?}", duration);
            } else {
                println!("no dream scheduled");
            }
            Ok(EXIT_SUCCESS)
        }
    }
}

pub(crate) fn build_dream_runner(cli: &Cli, workdir: &Path) -> Result<DreamRunner> {
    let cli_config = resolve_config_for_workdir(cli, workdir)?;
    Ok(DreamRunner::new(
        workdir.to_path_buf(),
        DreamLoopConfig {
            auto_dream: cli_config.dreams.auto_dream,
            idle_threshold_mins: cli_config.dreams.idle_threshold_mins,
            min_episodes_for_dream: cli_config.dreams.min_episodes_for_dream,
            schedule: cli_config.dreams.schedule_policy(),
            agent: DreamAgentConfig {
                command: cli_config.agent.command.clone(),
                args: cli_config.agent.args.clone(),
                model: cli_config.agent.model.clone(),
                bare_mode: cli_config.agent.bare_mode,
                effort: cli_config.agent.effort.clone(),
                fallback_model: cli_config.agent.fallback_model.clone(),
                timeout_ms: cli_config.agent.timeout_ms,
                env: cli_config.agent.env.clone(),
            },
        },
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NeuroTransferReport {
    pub(crate) live: NeuroFileSet,
    pub(crate) snapshot: NeuroFileSet,
    pub(crate) confirmations_present: bool,
    /// Versioned backup manifest path.
    pub(crate) manifest: PathBuf,
    /// Number of entries exported (only relevant for backup with --top-n).
    pub(crate) entries_exported: usize,
    /// Number of entries restored (only relevant for restore).
    pub(crate) entries_restored: usize,
    /// Number of entries filtered out during restore.
    pub(crate) entries_filtered: usize,
    /// Number skipped as exact or semantic duplicates during restore.
    pub(crate) entries_skipped_dedup: usize,
    /// Number skipped because they contradict high-confidence knowledge.
    pub(crate) entries_skipped_contradiction: usize,
    /// Number of malformed entries. Always zero on success.
    pub(crate) malformed_entries: usize,
    /// Whether restore used the explicit legacy migration path.
    pub(crate) legacy_input: bool,
}

const NEURO_BACKUP_MANIFEST_FILE: &str = "manifest.json";
const NEURO_BACKUP_MANIFEST_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct NeuroArtifactDigest {
    bytes: u64,
    sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct NeuroBackupManifest {
    version: u32,
    created_at: chrono::DateTime<chrono::Utc>,
    entry_count: usize,
    top_n: Option<usize>,
    source_path: String,
    #[serde(default)]
    knowledge_format_version: u32,
    #[serde(default)]
    confirmations_present: bool,
    #[serde(default)]
    confirmations: Option<NeuroArtifactDigest>,
}

pub(crate) fn backup_neuro_store(
    workdir: &Path,
    destination: &Path,
    force: bool,
    top_n: Option<usize>,
) -> Result<NeuroTransferReport> {
    let live = neuro_live_files(workdir);
    let snapshot = neuro_snapshot_files(destination);
    reject_backup_alias(destination, &live)?;

    if destination.exists() && !destination.is_dir() {
        bail!(
            "backup target must be a directory, found file at {}",
            destination.display()
        );
    }
    let destination_populated = destination.exists()
        && std::fs::read_dir(destination)
            .with_context(|| format!("read backup directory {}", destination.display()))?
            .next()
            .transpose()?
            .is_some();
    if destination_populated && !force {
        bail!(
            "backup would replace populated directory {}. Re-run with --force to replace it.",
            destination.display()
        );
    }

    let parent = parent_dir(destination);
    std::fs::create_dir_all(parent)
        .with_context(|| format!("create backup parent directory {}", parent.display()))?;
    let staging = tempfile::Builder::new()
        .prefix(".roko-neuro-backup-stage-")
        .tempdir_in(parent)
        .context("create backup staging directory")?;
    let staged = neuro_snapshot_files(staging.path());

    let store = KnowledgeStore::for_workdir(workdir);
    let entries_exported = store.export(
        &staged.knowledge,
        &ExportFilter {
            max_entries: top_n,
            filter_secrets: true,
            ..Default::default()
        },
    )?;
    let confirmation_bytes = if live.confirmations.exists() {
        Some(std::fs::read(&live.confirmations).with_context(|| {
            format!(
                "read live confirmation records from {}",
                live.confirmations.display()
            )
        })?)
    } else {
        None
    };
    if let Some(bytes) = &confirmation_bytes {
        roko_fs::atomic_write_bytes(&staged.confirmations, bytes).with_context(|| {
            format!(
                "write staged confirmation records to {}",
                staged.confirmations.display()
            )
        })?;
    }
    let confirmations_present = confirmation_bytes.is_some();
    let manifest = NeuroBackupManifest {
        version: NEURO_BACKUP_MANIFEST_VERSION,
        created_at: chrono::Utc::now(),
        entry_count: entries_exported,
        top_n,
        source_path: live.knowledge.display().to_string(),
        knowledge_format_version: roko_neuro::knowledge_store::KNOWLEDGE_BACKUP_VERSION,
        confirmations_present,
        confirmations: confirmation_bytes.as_deref().map(neuro_artifact_digest),
    };
    let staged_manifest = staging.path().join(NEURO_BACKUP_MANIFEST_FILE);
    roko_fs::atomic_write_json(&staged_manifest, &manifest)
        .with_context(|| format!("write staged manifest to {}", staged_manifest.display()))?;

    publish_staged_directory(staging.path(), destination)?;
    let manifest_path = destination.join(NEURO_BACKUP_MANIFEST_FILE);

    Ok(NeuroTransferReport {
        live,
        snapshot,
        confirmations_present,
        manifest: manifest_path,
        entries_exported,
        entries_restored: 0,
        entries_filtered: 0,
        entries_skipped_dedup: 0,
        entries_skipped_contradiction: 0,
        malformed_entries: 0,
        legacy_input: false,
    })
}

pub(crate) fn restore_neuro_store(
    workdir: &Path,
    source: &Path,
    force: bool,
    generation: u32,
    decay_factor: f64,
    min_confidence: Option<f64>,
    type_filters: Option<&[String]>,
    allow_legacy: bool,
) -> Result<NeuroTransferReport> {
    let live = neuro_live_files(workdir);
    let snapshot = neuro_snapshot_files(source);
    let kinds = type_filters.map(parse_knowledge_kinds).transpose()?;
    ensure!(
        decay_factor.is_finite() && (0.0..=1.0).contains(&decay_factor),
        "decay factor must be between 0.0 and 1.0"
    );
    let confidence_multiplier = decay_factor.powf(f64::from(generation));
    ensure!(
        snapshot.knowledge.is_file(),
        "restore source file not found: {}",
        snapshot.knowledge.display()
    );
    reject_restore_alias(source, &live)?;

    let (manifest, legacy_manifest) = read_neuro_backup_manifest(source, allow_legacy)?;
    let confirmation_bytes = if snapshot.confirmations.exists() {
        ensure!(
            snapshot.confirmations.is_file(),
            "restore confirmations path is not a file: {}",
            snapshot.confirmations.display()
        );
        Some(std::fs::read(&snapshot.confirmations).with_context(|| {
            format!(
                "read backup confirmation records from {}",
                snapshot.confirmations.display()
            )
        })?)
    } else {
        None
    };
    validate_confirmation_artifact(manifest.as_ref(), confirmation_bytes.as_deref())?;

    for path in [&live.knowledge, &live.confirmations] {
        if path.exists() {
            ensure!(
                path.is_file(),
                "restore target is not a regular file: {}",
                path.display()
            );
        }
    }
    let knowledge_populated = live
        .knowledge
        .metadata()
        .map(|metadata| metadata.len() > 0)
        .unwrap_or(false);
    if !force && (knowledge_populated || live.confirmations.exists()) {
        bail!(
            "restore would modify existing neuro state at {}. Re-run with --force to proceed.",
            parent_dir(&live.knowledge).display()
        );
    }

    let live_root = live
        .knowledge
        .parent()
        .ok_or_else(|| anyhow!("resolve live neuro directory"))?;
    let live_parent = parent_dir(live_root);
    let staging_parent = nearest_existing_directory(live_parent)?;
    let staging = tempfile::Builder::new()
        .prefix(".roko-neuro-restore-stage-")
        .tempdir_in(&staging_parent)
        .context("create restore staging directory")?;
    let staged = neuro_snapshot_files(staging.path());
    if live.knowledge.exists() {
        let live_bytes = std::fs::read(&live.knowledge)
            .with_context(|| format!("read live knowledge store {}", live.knowledge.display()))?;
        roko_fs::atomic_write_bytes(&staged.knowledge, &live_bytes)
            .context("seed staged knowledge store")?;
    }

    let staged_store = KnowledgeStore::new(staged.knowledge.clone());
    let import_result = staged_store.import(
        &snapshot.knowledge,
        &ImportOptions {
            confidence_discount: confidence_multiplier,
            source_label: format!("restore:gen{generation}"),
            kinds,
            min_confidence,
            allow_legacy,
            ..Default::default()
        },
    )?;
    if !staged.knowledge.exists() {
        roko_fs::atomic_write_bytes(&staged.knowledge, b"")
            .context("materialize empty staged knowledge store")?;
    }
    if let Some(bytes) = &confirmation_bytes {
        roko_fs::atomic_write_bytes(&staged.confirmations, bytes)
            .context("stage confirmation records")?;
    }

    if let Some(manifest) = &manifest {
        ensure!(
            manifest.entry_count == import_result.source_entries,
            "backup manifest entry_count mismatch: manifest={}, knowledge={}",
            manifest.entry_count,
            import_result.source_entries
        );
        if manifest.version == NEURO_BACKUP_MANIFEST_VERSION {
            ensure!(
                !import_result.legacy_input,
                "canonical backup manifest cannot contain legacy knowledge data"
            );
        }
    }

    publish_staged_neuro_files(&staged, &live)?;
    let confirmations_present = confirmation_bytes.is_some();

    Ok(NeuroTransferReport {
        live,
        snapshot,
        confirmations_present,
        manifest: source.join(NEURO_BACKUP_MANIFEST_FILE),
        entries_exported: 0,
        entries_restored: import_result.imported,
        entries_filtered: import_result.skipped_filter,
        entries_skipped_dedup: import_result.skipped_dedup,
        entries_skipped_contradiction: import_result.skipped_contradiction,
        malformed_entries: import_result.malformed,
        legacy_input: import_result.legacy_input || legacy_manifest,
    })
}

fn parse_knowledge_kinds(values: &[String]) -> Result<Vec<KnowledgeKind>> {
    values
        .iter()
        .map(|value| match value.trim().to_ascii_lowercase().as_str() {
            "insight" | "fact" => Ok(KnowledgeKind::Insight),
            "heuristic" | "procedure" => Ok(KnowledgeKind::Heuristic),
            "anti_knowledge" | "antiknowledge" | "anti-knowledge" => {
                Ok(KnowledgeKind::AntiKnowledge)
            }
            "warning" | "constraint" => Ok(KnowledgeKind::Warning),
            "causal_link" | "causallink" | "causal-link" => Ok(KnowledgeKind::CausalLink),
            "strategy_fragment" | "strategyfragment" | "strategy-fragment" | "playbook" => {
                Ok(KnowledgeKind::StrategyFragment)
            }
            other => bail!("unknown knowledge type `{other}`"),
        })
        .collect()
}

fn print_import_result(
    cli: &Cli,
    input: &Path,
    decay_factor: f64,
    result: &ImportResult,
) -> Result<()> {
    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "input": input,
                "decay_factor": decay_factor,
                "result": result,
            }))?
        );
    } else {
        println!("Knowledge import from {}:", input.display());
        println!("  entries imported: {}", result.imported);
        println!("  entries skipped (dedup): {}", result.skipped_dedup);
        println!(
            "  entries skipped (contradiction): {}",
            result.skipped_contradiction
        );
        println!("  entries filtered: {}", result.skipped_filter);
        println!("  malformed entries: {}", result.malformed);
        if result.legacy_input {
            println!("  source format: explicit legacy migration");
        }
    }
    Ok(())
}

pub(crate) fn neuro_live_files(workdir: &Path) -> NeuroFileSet {
    let store = KnowledgeStore::for_workdir(workdir);
    NeuroFileSet {
        knowledge: store.path().to_path_buf(),
        confirmations: store.confirmations_path().to_path_buf(),
    }
}

pub(crate) fn neuro_snapshot_files(root: &Path) -> NeuroFileSet {
    NeuroFileSet {
        knowledge: root.join(NEURO_KNOWLEDGE_FILE),
        confirmations: root.join(NEURO_CONFIRMATIONS_FILE),
    }
}

fn neuro_artifact_digest(bytes: &[u8]) -> NeuroArtifactDigest {
    let digest = Sha256::digest(bytes);
    NeuroArtifactDigest {
        bytes: bytes.len() as u64,
        sha256: digest.iter().fold(String::new(), |mut output, byte| {
            use std::fmt::Write;
            let _ = write!(output, "{byte:02x}");
            output
        }),
    }
}

fn read_neuro_backup_manifest(
    source: &Path,
    allow_legacy: bool,
) -> Result<(Option<NeuroBackupManifest>, bool)> {
    let path = source.join(NEURO_BACKUP_MANIFEST_FILE);
    if !path.exists() {
        ensure!(
            allow_legacy,
            "backup manifest not found at {}; use explicit --legacy-raw migration only for a trusted legacy backup",
            path.display()
        );
        return Ok((None, true));
    }
    ensure!(
        path.is_file(),
        "backup manifest is not a file: {}",
        path.display()
    );
    let bytes = std::fs::read(&path)
        .with_context(|| format!("read backup manifest from {}", path.display()))?;
    let manifest: NeuroBackupManifest = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse backup manifest from {}", path.display()))?;
    match manifest.version {
        NEURO_BACKUP_MANIFEST_VERSION => {
            ensure!(
                manifest.knowledge_format_version
                    == roko_neuro::knowledge_store::KNOWLEDGE_BACKUP_VERSION,
                "unsupported knowledge format version {} in backup manifest",
                manifest.knowledge_format_version
            );
            ensure!(
                !manifest.source_path.trim().is_empty(),
                "canonical backup manifest has an empty source path"
            );
            if let Some(limit) = manifest.top_n {
                ensure!(
                    manifest.entry_count <= limit,
                    "backup manifest entry_count {} exceeds top_n {limit}",
                    manifest.entry_count
                );
            }
            Ok((Some(manifest), false))
        }
        1 if allow_legacy => Ok((Some(manifest), true)),
        1 => bail!("legacy backup manifest version 1 requires explicit --legacy-raw migration"),
        version => bail!(
            "unsupported backup manifest version {version} (this build supports version {NEURO_BACKUP_MANIFEST_VERSION})"
        ),
    }
}

fn validate_confirmation_artifact(
    manifest: Option<&NeuroBackupManifest>,
    bytes: Option<&[u8]>,
) -> Result<()> {
    let Some(manifest) = manifest else {
        return Ok(());
    };
    if manifest.version != NEURO_BACKUP_MANIFEST_VERSION {
        return Ok(());
    }
    ensure!(
        manifest.confirmations_present == bytes.is_some(),
        "backup confirmation presence does not match manifest"
    );
    match (&manifest.confirmations, bytes) {
        (Some(expected), Some(bytes)) => ensure!(
            expected == &neuro_artifact_digest(bytes),
            "backup confirmation integrity verification failed"
        ),
        (None, None) => {}
        _ => bail!("backup confirmation digest does not match manifest presence"),
    }
    Ok(())
}

fn publish_staged_directory(staged: &Path, destination: &Path) -> Result<()> {
    let parent = parent_dir(destination);
    let rollback = tempfile::Builder::new()
        .prefix(".roko-neuro-backup-rollback-")
        .tempdir_in(parent)
        .context("create backup rollback directory")?;
    let previous = rollback.path().join("previous");
    let had_previous = destination.exists();
    if had_previous {
        std::fs::rename(destination, &previous).with_context(|| {
            format!(
                "stage existing backup directory {} for replacement",
                destination.display()
            )
        })?;
    }
    if let Err(error) = std::fs::rename(staged, destination) {
        if had_previous {
            let _ = std::fs::rename(&previous, destination);
        }
        return Err(error).with_context(|| {
            format!(
                "publish staged backup directory to {}",
                destination.display()
            )
        });
    }
    if let Err(error) = sync_neuro_directory(parent) {
        let _ = std::fs::remove_dir_all(destination);
        if had_previous {
            let _ = std::fs::rename(&previous, destination);
        }
        return Err(error).context("durably publish backup directory");
    }
    Ok(())
}

fn publish_staged_neuro_files(staged: &NeuroFileSet, live: &NeuroFileSet) -> Result<()> {
    let live_root = live
        .knowledge
        .parent()
        .ok_or_else(|| anyhow!("resolve live neuro directory"))?;
    std::fs::create_dir_all(live_root)
        .with_context(|| format!("create live neuro directory {}", live_root.display()))?;
    let rollback = tempfile::Builder::new()
        .prefix(".restore-rollback-")
        .tempdir_in(live_root)
        .context("create restore rollback directory")?;
    let old_knowledge = rollback.path().join(NEURO_KNOWLEDGE_FILE);
    let old_confirmations = rollback.path().join(NEURO_CONFIRMATIONS_FILE);
    let had_knowledge = live.knowledge.exists();
    let had_confirmations = live.confirmations.exists();

    if had_knowledge {
        std::fs::rename(&live.knowledge, &old_knowledge)
            .context("stage live knowledge store for transactional restore")?;
    }
    if had_confirmations
        && let Err(error) = std::fs::rename(&live.confirmations, &old_confirmations)
    {
        if had_knowledge {
            let _ = std::fs::rename(&old_knowledge, &live.knowledge);
        }
        return Err(error).context("stage live confirmations for transactional restore");
    }

    let publish_result = (|| -> Result<()> {
        std::fs::rename(&staged.knowledge, &live.knowledge)
            .context("publish restored knowledge store")?;
        if staged.confirmations.exists() {
            std::fs::rename(&staged.confirmations, &live.confirmations)
                .context("publish restored confirmations")?;
        }
        sync_neuro_directory(live_root)?;
        Ok(())
    })();

    if let Err(error) = publish_result {
        let _ = std::fs::remove_file(&live.knowledge);
        let _ = std::fs::remove_file(&live.confirmations);
        if had_knowledge {
            let _ = std::fs::rename(&old_knowledge, &live.knowledge);
        }
        if had_confirmations {
            let _ = std::fs::rename(&old_confirmations, &live.confirmations);
        }
        return Err(error);
    }
    Ok(())
}

fn reject_backup_alias(destination: &Path, live: &NeuroFileSet) -> Result<()> {
    let destination = resolved_path(destination)?;
    let live_knowledge = resolved_path(&live.knowledge)?;
    ensure!(
        !live_knowledge.starts_with(&destination),
        "backup destination cannot equal or contain the live neuro store"
    );
    Ok(())
}

fn reject_restore_alias(source: &Path, live: &NeuroFileSet) -> Result<()> {
    let source = resolved_path(source)?;
    let live_root = live
        .knowledge
        .parent()
        .ok_or_else(|| anyhow!("resolve live neuro directory"))?;
    ensure!(
        source != resolved_path(live_root)?,
        "restore source cannot be the live neuro directory"
    );
    Ok(())
}

fn resolved_path(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut cursor = absolute.as_path();
    let mut suffix = Vec::new();
    while !cursor.exists() {
        let name = cursor
            .file_name()
            .ok_or_else(|| anyhow!("resolve path {}", path.display()))?;
        suffix.push(name.to_os_string());
        cursor = cursor
            .parent()
            .ok_or_else(|| anyhow!("resolve parent of {}", path.display()))?;
    }
    let mut resolved = std::fs::canonicalize(cursor)
        .with_context(|| format!("resolve existing path ancestor {}", cursor.display()))?;
    for component in suffix.into_iter().rev() {
        resolved.push(component);
    }
    Ok(resolved)
}

fn nearest_existing_directory(path: &Path) -> Result<PathBuf> {
    let mut candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    while !candidate.exists() {
        ensure!(
            candidate.pop(),
            "no existing ancestor found for {}",
            path.display()
        );
    }
    ensure!(
        candidate.is_dir(),
        "existing path ancestor is not a directory: {}",
        candidate.display()
    );
    Ok(candidate)
}

fn parent_dir(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

#[cfg(unix)]
fn sync_neuro_directory(path: &Path) -> Result<()> {
    std::fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .with_context(|| format!("sync directory {}", path.display()))
}

#[cfg(not(unix))]
fn sync_neuro_directory(_path: &Path) -> Result<()> {
    Ok(())
}

pub(crate) const NEURO_KNOWLEDGE_FILE: &str = "knowledge.jsonl";
pub(crate) const NEURO_CONFIRMATIONS_FILE: &str = "knowledge-confirmations.jsonl";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NeuroFileSet {
    pub(crate) knowledge: PathBuf,
    pub(crate) confirmations: PathBuf,
}
