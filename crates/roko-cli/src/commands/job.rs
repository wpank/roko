//! job command handlers.

use crate::*;

pub(crate) async fn cmd_job(cli: &Cli, cmd: JobCmd) -> Result<i32> {
    let jobs_dir = |wd: &Path| wd.join(".roko").join("jobs");

    match cmd {
        JobCmd::List { workdir, status } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let store = roko_core::FileJobStore::new(jobs_dir(&wd));

            // Build a typed filter from the optional status string.
            let filter = roko_core::JobFilter {
                state: status.as_deref().and_then(roko_core::JobStatus::parse),
                ..Default::default()
            };

            let (jobs, malformed) = store
                .list_with_diagnostics(&filter)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            // Report malformed files (visible in both JSON and human modes).
            for bad in &malformed {
                eprintln!(
                    "warning: malformed job file {}: {}",
                    bad.path.display(),
                    bad.error
                );
            }

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&jobs)?);
                if !malformed.is_empty() {
                    // Non-zero exit when there are corrupted files per the
                    // backlog #328 contract.
                    return Ok(EXIT_FAILURE);
                }
                return Ok(EXIT_SUCCESS);
            }

            if jobs.is_empty() {
                println!("No jobs found.");
            } else {
                for job in &jobs {
                    let effective_status = job.effective_status();
                    let icon = match effective_status {
                        "open" | "pending" => "\u{25cb}",
                        "assigned" => "\u{25d4}",
                        "in_progress" | "active" | "running" => "\u{25b6}",
                        "submitted" => "\u{25d1}",
                        "completed" | "done" => "\u{2713}",
                        "failed" | "cancelled" => "\u{2717}",
                        _ => "\u{00b7}",
                    };
                    println!(
                        "{icon} [{:>12}] {:>10}  {}  {}",
                        job.job_type,
                        effective_status,
                        &job.id[..job.id.len().min(8)],
                        job.title
                    );
                }
                println!("\n{} job(s)", jobs.len());
            }

            if !malformed.is_empty() {
                return Ok(EXIT_FAILURE);
            }
            Ok(EXIT_SUCCESS)
        }
        JobCmd::Create {
            title,
            r#type,
            description,
            priority,
            auto_execute,
            plan_id,
            tag,
            reward,
            posted_by,
            workdir,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let store = roko_core::FileJobStore::new(jobs_dir(&wd));

            let req = roko_core::CreateJobRequest {
                title: title.trim().to_string(),
                description: description.trim().to_string(),
                job_type: r#type.trim().to_string(),
                priority: priority.trim().to_string(),
                auto_execute,
                tags: tag,
                reward: reward.unwrap_or_default(),
                posted_by: posted_by.unwrap_or_default(),
                ..Default::default()
            };

            // Reject blank required fields.
            if req.title.is_empty() {
                anyhow::bail!("job title must not be blank");
            }

            let mut job = store
                .create(&req)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            // plan_id is not part of CreateJobRequest; set it post-create if
            // provided and persist the update.
            if let Some(ref pid) = plan_id {
                if !pid.is_empty() {
                    job.plan_id = pid.clone();
                    store.save(&job).await.map_err(|e| anyhow::anyhow!("{e}"))?;
                }
            }

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&job)?);
            } else {
                println!("Created job: {}", job.id);
                println!("  title:    {}", job.title);
                println!("  type:     {}", job.job_type);
                println!("  priority: {}", job.priority);
                println!("  auto_execute: {}", job.auto_execute);
                if !job.tags.is_empty() {
                    println!("  tags:     {}", job.tags.join(", "));
                }
                if !job.reward.is_empty() {
                    println!("  reward:   {}", job.reward);
                }
            }
            Ok(EXIT_SUCCESS)
        }
        JobCmd::Match {
            title,
            serve_url,
            description,
            language,
            min_tier,
            reward,
            skills,
            workdir,
        } => {
            let default_wd = resolve_workdir(cli);
            let wd = workdir.as_deref().unwrap_or(&default_wd);
            let auth_cfg = load_resolved_config(wd)
                .map(|r| r.config.serve.auth)
                .unwrap_or_default();
            let headers = match auth::resolve_api_key(&auth_cfg, None) {
                Some(resolved) => resolved.headers(),
                None => reqwest::header::HeaderMap::new(),
            };
            let body = serde_json::json!({
                "title": title,
                "description": description,
                "language": language,
                "minTier": min_tier,
                "reward": reward,
                "skills": skills,
            });
            let client = reqwest::Client::new();
            let resp = client
                .post(format!(
                    "{}/api/jobs/match",
                    serve_url.trim_end_matches('/')
                ))
                .headers(headers)
                .json(&body)
                .send()
                .await?;
            let status = resp.status();
            let payload: serde_json::Value = resp.json().await.unwrap_or_default();
            if !status.is_success() {
                anyhow::bail!(
                    "failed to match job: {} {}",
                    status,
                    serde_json::to_string_pretty(&payload).unwrap_or_default()
                );
            }

            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&payload).unwrap_or_default()
                );
                return Ok(EXIT_SUCCESS);
            }

            let candidates = payload
                .get("candidates")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();
            if candidates.is_empty() {
                println!("No matching agents found.");
                return Ok(EXIT_SUCCESS);
            }

            println!(
                "Matched {} candidate(s), total_fee={}, eta_hours={}",
                candidates.len(),
                payload
                    .get("totalFee")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
                payload
                    .get("etaHours")
                    .and_then(serde_json::Value::as_f64)
                    .map(|v| format!("{v:.1}"))
                    .unwrap_or_else(|| "?".to_string())
            );
            println!(
                "{:<24} {:<11} {:>5} {:>9} {:>9} {:>14}",
                "agent", "tier", "rep", "inflight", "jobs", "bid"
            );
            for candidate in candidates {
                let agent = candidate
                    .get("agentId")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                let tier = candidate
                    .get("tier")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                let reputation = candidate
                    .get("reputation")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let inflight = candidate
                    .get("inflightJobs")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let max_concurrent = candidate
                    .get("maxConcurrentJobs")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let past_jobs = candidate
                    .get("pastJobs")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let bid = candidate
                    .get("bidShare")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                println!(
                    "{:<24} {:<11} {:>5} {:>4}/{:<4} {:>9} {:>14}",
                    agent, tier, reputation, inflight, max_concurrent, past_jobs, bid
                );
            }
            Ok(EXIT_SUCCESS)
        }
        JobCmd::Show { id, workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let store = roko_core::FileJobStore::new(jobs_dir(&wd));

            let resolved_id = store
                .resolve_by_prefix(&id)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let job = store
                .get(&resolved_id)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            // Emit legacy-state migration diagnostic when applicable.
            if let Some(diag) = roko_core::FileJobStore::migration_diagnostic(&job) {
                if diag.disagreement {
                    eprintln!(
                        "warning: job '{}' has disagreeing state='{}' vs status='{}'; canonical status wins",
                        diag.job_id, diag.legacy_state, diag.canonical_status,
                    );
                }
            }

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&job)?);
                return Ok(EXIT_SUCCESS);
            }

            let effective_status = job.effective_status();
            println!("id:           {}", job.id);
            println!("title:        {}", job.title);
            println!("type:         {}", job.job_type);
            println!("status:       {effective_status}");
            println!("priority:     {}", job.priority);
            println!("posted_by:    {}", job.posted_by);
            println!("assigned_to:  {}", job.assigned_to);
            println!("auto_execute: {}", job.auto_execute);
            println!("plan_id:      {}", job.plan_id);
            println!("created_at:   {}", job.created_at);
            println!("updated_at:   {}", job.updated_at);
            if !job.tags.is_empty() {
                println!("tags:         {}", job.tags.join(", "));
            }
            if !job.description.is_empty() {
                println!("\n--- description ---\n{}", job.description);
            }
            if let Some(ref sub) = job.submission {
                println!(
                    "\n--- submission ---\n{}",
                    serde_json::to_string_pretty(sub).unwrap_or_default()
                );
            }
            if let Some(ref eval) = job.evaluation {
                println!(
                    "\n--- evaluation ---\n{}",
                    serde_json::to_string_pretty(eval).unwrap_or_default()
                );
            }
            Ok(EXIT_SUCCESS)
        }
        JobCmd::Execute {
            id,
            serve_url,
            workdir,
        } => {
            let mode = if serve_url.is_some() {
                roko_core::JobExecutionMode::Serve
            } else {
                roko_core::JobExecutionMode::Local
            };

            if let Some(url) = serve_url {
                // Delegate to roko-serve via the authenticated HTTP adapter.
                let default_wd = resolve_workdir(cli);
                let wd = workdir.as_deref().unwrap_or(&default_wd);
                let auth_cfg = load_resolved_config(wd)
                    .map(|r| r.config.serve.auth)
                    .unwrap_or_default();
                let headers = match auth::resolve_api_key(&auth_cfg, None) {
                    Some(resolved) => resolved.headers(),
                    None => reqwest::header::HeaderMap::new(),
                };
                let client = reqwest::Client::new();
                let resp = client
                    .post(format!("{url}/api/jobs/{id}/execute"))
                    .headers(headers)
                    .send()
                    .await?;
                let status = resp.status();
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                if status.is_success() {
                    if cli.json {
                        let receipt = serde_json::json!({
                            "job_id": id,
                            "prior_status": "open",
                            "new_status": "executing",
                            "mode": mode.to_string(),
                            "run_id": body.get("run_id").and_then(|v| v.as_str()).unwrap_or(""),
                            "acknowledged": false,
                        });
                        println!("{}", serde_json::to_string_pretty(&receipt)?);
                    } else {
                        println!("Job '{id}' execution started via serve.");
                    }
                } else {
                    anyhow::bail!(
                        "failed to execute job '{id}': {} {}",
                        status,
                        serde_json::to_string_pretty(&body).unwrap_or_default()
                    );
                }
            } else {
                // Local inline execution via JobExecutionService.
                let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
                let svc = roko_core::JobExecutionService::new(jobs_dir(&wd));

                // Start: acquires lease, transitions open -> in_progress.
                let (job, receipt) = match svc.start(&id, mode).await {
                    Ok(pair) => pair,
                    Err(roko_core::JobError::LeaseHeld { id: held_id }) => {
                        if cli.json {
                            let receipt = serde_json::json!({
                                "job_id": held_id,
                                "error": "lease_held",
                                "message": "job already has an active execution lease",
                            });
                            println!("{}", serde_json::to_string_pretty(&receipt)?);
                        } else {
                            println!("Job '{id}' already has an active execution lease.");
                        }
                        return Ok(EXIT_SUCCESS);
                    }
                    Err(e) => bail!("{e}"),
                };

                if !cli.json {
                    println!(
                        "Executing job '{}' locally (run={})...",
                        receipt.job_id, receipt.run_id
                    );
                }

                // Build prompt based on job type.
                let prompt = match job.job_type.as_str() {
                    "research" => format!(
                        "Research the following topic and produce a detailed report with citations:\n\n{}",
                        job.description
                    ),
                    "coding_task" | "coding" => {
                        if !job.plan_id.is_empty() {
                            format!("Execute plan '{}' in the current workspace", job.plan_id)
                        } else {
                            job.description.clone()
                        }
                    }
                    _ => job.description.clone(),
                };

                // Take the cancellation receiver so we can race it against
                // the dispatch. If the job is cancelled while running, the
                // receiver fires and we transition to `cancelled`.
                let cancel_rx = svc.take_cancel_receiver(&receipt.job_id);

                let config = resolve_config_for_workdir(cli, &wd)?;

                // Dispatch through run_once_inline (the canonical execution
                // path) rather than calling run_once directly. This provides
                // proper inline terminal output and consistent observation.
                let result = if let Some(mut rx) = cancel_rx {
                    tokio::select! {
                        biased;
                        _ = &mut rx => {
                            // Cancellation acknowledged.
                            let cancel_receipt = svc
                                .cancel(&receipt.job_id, mode)
                                .await
                                .map_err(|e| anyhow::anyhow!("{e}"))?;
                            if cli.json {
                                println!("{}", serde_json::to_string_pretty(&cancel_receipt)?);
                            } else {
                                println!("Job '{}' cancelled during execution.", receipt.job_id);
                            }
                            return Ok(EXIT_SUCCESS);
                        }
                        res = roko_cli::run_inline::run_once_inline(&wd, &config, &prompt, None) => res,
                    }
                } else {
                    roko_cli::run_inline::run_once_inline(&wd, &config, &prompt, None).await
                };

                match result {
                    Ok(report) => {
                        let submission = serde_json::json!({
                            "result_summary": if report.overall_success() { "success" } else { "completed with failures" },
                            "completed_at": chrono::Utc::now().to_rfc3339(),
                        });
                        let complete_receipt = svc
                            .complete(&receipt.job_id, mode, submission)
                            .await
                            .map_err(|e| anyhow::anyhow!("{e}"))?;
                        if cli.json {
                            println!("{}", serde_json::to_string_pretty(&complete_receipt)?);
                        } else {
                            println!("Job '{}' completed successfully.", receipt.job_id);
                        }
                    }
                    Err(e) => {
                        let fail_receipt = svc
                            .fail(&receipt.job_id, mode, &e.to_string())
                            .await
                            .map_err(|je| anyhow::anyhow!("{je}"))?;
                        if cli.json {
                            println!("{}", serde_json::to_string_pretty(&fail_receipt)?);
                        }
                        return Err(e.context(format!("job '{}' failed", receipt.job_id)));
                    }
                }
            }
            Ok(EXIT_SUCCESS)
        }
        JobCmd::Cancel { id, workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let store = roko_core::FileJobStore::new(jobs_dir(&wd));

            match store.cancel_inactive(&id).await {
                Ok(job) => {
                    if cli.json {
                        println!("{}", serde_json::to_string_pretty(&job)?);
                    } else {
                        println!("Job '{}' cancelled.", job.id);
                    }
                    Ok(EXIT_SUCCESS)
                }
                Err(roko_core::JobError::ActiveCancellationDenied {
                    id: held_id,
                    status,
                }) => {
                    if cli.json {
                        let receipt = serde_json::json!({
                            "job_id": held_id,
                            "error": "active_cancellation_requires_executor_ack",
                            "status": status,
                            "message": "job is active; cancellation requires executor acknowledgement (#371)",
                        });
                        println!("{}", serde_json::to_string_pretty(&receipt)?);
                    } else {
                        eprintln!(
                            "Job '{held_id}' is {status} and cannot be cancelled while active \
                             (active cancellation requires executor acknowledgement, see #371)."
                        );
                    }
                    Ok(EXIT_FAILURE)
                }
                Err(e) => Err(anyhow::anyhow!("{e}")),
            }
        }
        JobCmd::Recover { id, workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let svc = roko_core::JobExecutionService::new(jobs_dir(&wd));

            let receipt = svc
                .recover(&id, roko_core::JobExecutionMode::Local)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&receipt)?);
            } else {
                println!(
                    "Job '{}' recovered ({} -> {}).",
                    receipt.job_id, receipt.prior_status, receipt.new_status
                );
            }
            Ok(EXIT_SUCCESS)
        }
    }
}
