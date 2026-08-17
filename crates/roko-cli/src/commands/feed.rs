//! `roko feed` -- inspect registered runtime feeds.
//!
//! The CLI queries a running `roko serve` instance for discovery, lifecycle,
//! and health. Connection failures are reported without panicking.

use anyhow::Result;
use clap::Subcommand;
use roko_core::feed::FeedRuntimeStatus;

use crate::*;

/// Feed management subcommands.
#[derive(Debug, Subcommand)]
pub enum FeedCmd {
    /// List all runtime feeds with their topics and status.
    List,
    /// Show detailed status for a specific feed.
    Status {
        /// Feed identifier to inspect (e.g. `file-watch-roko-dir`).
        id: String,
    },
    /// Start a discoverable runtime feed.
    Start { id: String },
    /// Stop a running feed.
    Stop { id: String },
    /// Show aggregate health for all discoverable feeds.
    Health,
    /// List feed types available to start.
    Discover,
    /// Search available feeds by id, name, description, or kind.
    Search { query: String },
}

pub(crate) async fn cmd_feed(cli: &Cli, cmd: FeedCmd) -> Result<i32> {
    match cmd {
        FeedCmd::List => cmd_list(cli).await,
        FeedCmd::Status { id } => cmd_status(cli, &id).await,
        FeedCmd::Start { id } => cmd_lifecycle(cli, "start", &id).await,
        FeedCmd::Stop { id } => cmd_lifecycle(cli, "stop", &id).await,
        FeedCmd::Health => cmd_health(cli).await,
        FeedCmd::Discover => cmd_discover(cli, None).await,
        FeedCmd::Search { query } => cmd_discover(cli, Some(&query)).await,
    }
}

async fn cmd_lifecycle(cli: &Cli, action: &str, id: &str) -> Result<i32> {
    let url = format!("{}/api/feeds/{action}/{id}", serve_base_url());
    match reqwest::Client::new().post(url).send().await {
        Ok(response) if response.status().is_success() => {
            let status: FeedRuntimeStatus = response.json().await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&status)?);
            } else {
                println!(
                    "{} feed '{}' ({})",
                    if action == "start" {
                        "started"
                    } else {
                        "stopped"
                    },
                    status.id,
                    status.topic
                );
            }
            Ok(EXIT_SUCCESS)
        }
        Ok(response) => {
            if !cli.quiet {
                eprintln!("roko serve returned HTTP {}", response.status());
            }
            Ok(EXIT_FAILURE)
        }
        Err(_) => unavailable(cli),
    }
}

async fn cmd_health(cli: &Cli) -> Result<i32> {
    let url = format!("{}/api/feeds/health", serve_base_url());
    match reqwest::get(url).await {
        Ok(response) if response.status().is_success() => {
            let statuses: Vec<FeedRuntimeStatus> = response.json().await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&statuses)?);
            } else {
                println!(
                    "{:<28} {:<13} {:>10} {:>10}",
                    "FEED", "STATUS", "PULSES", "RATE"
                );
                println!("{}", "-".repeat(66));
                for status in statuses {
                    println!(
                        "{:<28} {:<13} {:>10} {:>8.2} Hz",
                        status.id,
                        if status.connected {
                            "connected"
                        } else if status.error.is_some() {
                            "degraded"
                        } else {
                            "stopped"
                        },
                        status.pulses_produced,
                        status.rate_hz
                    );
                }
            }
            Ok(EXIT_SUCCESS)
        }
        Ok(response) => {
            if !cli.quiet {
                eprintln!("roko serve returned HTTP {}", response.status());
            }
            Ok(EXIT_FAILURE)
        }
        Err(_) => unavailable(cli),
    }
}

async fn cmd_discover(cli: &Cli, query: Option<&str>) -> Result<i32> {
    let base = serve_base_url();
    let client = reqwest::Client::new();
    let request = if let Some(query) = query {
        client
            .get(format!("{base}/api/feeds/search"))
            .query(&[("q", query)])
    } else {
        client.get(format!("{base}/api/feeds/discover"))
    };
    match request.send().await {
        Ok(response) if response.status().is_success() => {
            let feeds: Vec<roko_core::FeedInfo> = response.json().await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&feeds)?);
            } else {
                println!("{:<28} {:<12} {}", "ID", "KIND", "DESCRIPTION");
                println!("{}", "-".repeat(80));
                for feed in feeds {
                    println!(
                        "{:<28} {:<12} {}",
                        feed.id,
                        format!("{:?}", feed.kind),
                        feed.description
                    );
                }
            }
            Ok(EXIT_SUCCESS)
        }
        Ok(response) => {
            if !cli.quiet {
                eprintln!("roko serve returned HTTP {}", response.status());
            }
            Ok(EXIT_FAILURE)
        }
        Err(_) => unavailable(cli),
    }
}

fn unavailable(cli: &Cli) -> Result<i32> {
    if cli.json {
        println!(
            "{}",
            serde_json::json!({"error": "roko serve is not running"})
        );
    } else if !cli.quiet {
        eprintln!("roko serve is not running");
    }
    Ok(EXIT_FAILURE)
}

/// Resolve the roko serve base URL from the environment or default.
fn serve_base_url() -> String {
    std::env::var("ROKO_SERVE_URL")
        .unwrap_or_else(|_| "http://localhost:6677".to_string())
        .trim_end_matches('/')
        .to_string()
}

/// Lightweight summary for list output (mirrors the runtime status JSON).
#[derive(Debug, serde::Deserialize)]
struct FeedSummary {
    id: String,
    topic: String,
    kind: String,
    connected: bool,
    #[serde(default)]
    pulses_produced: u64,
}

async fn cmd_list(cli: &Cli) -> Result<i32> {
    let base = serve_base_url();
    let url = format!("{base}/api/feeds/runtime");

    match reqwest::get(&url).await {
        Ok(resp) if resp.status().is_success() => {
            let feeds: Vec<FeedSummary> = resp.json().await.unwrap_or_default();

            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "feeds": feeds.iter().map(|f| serde_json::json!({
                            "id": f.id,
                            "topic": f.topic,
                            "kind": f.kind,
                            "connected": f.connected,
                            "pulses_produced": f.pulses_produced,
                        })).collect::<Vec<_>>(),
                        "total": feeds.len(),
                    }))?
                );
            } else {
                println!(
                    "{:<24} {:<32} {:<10} {}",
                    "ID", "TOPIC", "KIND", "CONNECTED"
                );
                println!("{}", "-".repeat(80));

                for f in &feeds {
                    println!(
                        "{:<24} {:<32} {:<10} {}",
                        f.id,
                        f.topic,
                        f.kind,
                        if f.connected { "yes" } else { "no" }
                    );
                }
                if feeds.is_empty() {
                    println!("(no feeds registered)");
                }
            }
        }
        Ok(resp) => {
            let status = resp.status();
            if !cli.quiet {
                eprintln!("roko serve returned HTTP {status}");
            }
            return Ok(EXIT_FAILURE);
        }
        Err(_) => {
            if cli.json {
                println!(
                    "{}",
                    serde_json::json!({"error": "roko serve is not running"})
                );
            } else {
                println!("(roko serve is not running; no live feed data available)");
                println!("Start the server with: roko serve");
            }
        }
    }

    Ok(EXIT_SUCCESS)
}

async fn cmd_status(cli: &Cli, id: &str) -> Result<i32> {
    let base = serve_base_url();
    let url = format!("{base}/api/feeds/runtime/{id}");

    match reqwest::get(&url).await {
        Ok(resp) if resp.status().is_success() => {
            let status: FeedRuntimeStatus = resp.json().await?;

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&status)?);
            } else {
                println!("Feed: {}", status.id);
                println!("  topic:            {}", status.topic);
                println!("  kind:             {}", status.kind);
                println!("  connected:        {}", status.connected);
                println!("  rate_hz:          {:.2}", status.rate_hz);
                println!("  pulses_produced:  {}", status.pulses_produced);
                if let Some(ms) = status.last_update_ms {
                    println!("  last_update_ms:   {ms}");
                }
                if let Some(err) = &status.error {
                    println!("  error:            {err}");
                }
            }
            Ok(EXIT_SUCCESS)
        }
        Ok(resp) if resp.status().as_u16() == 404 => {
            if cli.json {
                println!(
                    "{}",
                    serde_json::json!({"error": format!("feed '{id}' not found")})
                );
            } else {
                eprintln!("feed '{id}' not found");
            }
            Ok(EXIT_FAILURE)
        }
        Ok(resp) => {
            let status = resp.status();
            eprintln!("roko serve returned HTTP {status}");
            Ok(EXIT_FAILURE)
        }
        Err(_) => {
            if cli.json {
                println!(
                    "{}",
                    serde_json::json!({"error": "roko serve is not running"})
                );
            } else {
                eprintln!("roko serve is not running");
            }
            Ok(EXIT_FAILURE)
        }
    }
}
