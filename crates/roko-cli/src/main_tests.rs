//! Tests for the `roko` binary's command line and dispatch (`main.rs`).

use super::*;
use clap::Parser;
use commands::config_cmd::{
    ConfigModelCmd, ConfigProviderCmd, ModelListRow, ProviderHealthRow, ProviderLatencySummary,
    ProviderListRow, build_model_list_row, build_provider_health_row, format_model_rows,
    format_provider_health_rows, format_provider_rows, reset_provider_health,
    select_provider_test_model,
};
use commands::dashboard::dashboard_output;
use commands::knowledge::{
    NEURO_CONFIRMATIONS_FILE, NEURO_KNOWLEDGE_FILE, backup_neuro_store, neuro_live_files,
    restore_neuro_store,
};
use commands::learn::InspectSubsystem;
use commands::util::persist_capture_episode;
use tempfile::tempdir;
use tokio::fs;

#[test]
fn cli_parses_no_args() {
    // With no args and no subcommand, cli.prompt and cli.command are None.
    let cli = Cli::try_parse_from(["roko"]).unwrap();
    assert!(cli.command.is_none());
    assert!(cli.prompt.is_none());
    assert!(!cli.json);
    assert!(!cli.quiet);
    assert!(!cli.headless);
}

#[test]
fn cli_parses_global_flags() {
    let cli = Cli::try_parse_from([
        "roko",
        "--role",
        "engineer",
        "--model",
        "gpt-4",
        "--repo",
        "/tmp/proj",
        "--effort",
        "high",
        "--json",
        "--quiet",
        "--headless",
    ])
    .unwrap();
    assert_eq!(cli.role.as_deref(), Some("engineer"));
    assert_eq!(cli.model.as_deref(), Some("gpt-4"));
    assert_eq!(cli.repo, Some(PathBuf::from("/tmp/proj")));
    assert_eq!(cli.effort, Some(Effort::High));
    assert!(cli.json);
    assert!(cli.quiet);
    assert!(cli.headless);
}

#[test]
fn cli_parses_learn_reflexes_workdir() {
    let cli = Cli::try_parse_from([
        "roko",
        "learn",
        "reflexes",
        "--workdir",
        "/tmp/reflex-project",
    ])
    .expect("parse learn reflexes");

    assert!(matches!(
        cli.command,
        Some(Command::Learn {
            cmd: LearnCmd::Reflexes {
                workdir: Some(ref workdir),
            },
        }) if workdir == std::path::Path::new("/tmp/reflex-project")
    ));
}

// ── #311: learn inspect / config preset parser tests ──────────

#[test]
fn cli_parses_learn_inspect_gates() {
    let cli = Cli::try_parse_from(["roko", "learn", "inspect", "gates"])
        .expect("parse learn inspect gates");
    assert!(matches!(
        cli.command,
        Some(Command::Learn {
            cmd: LearnCmd::Inspect {
                subsystem: InspectSubsystem::Gates { workdir: None },
            },
        })
    ));
}

#[test]
fn cli_parses_learn_inspect_routing_with_workdir() {
    let cli = Cli::try_parse_from([
        "roko",
        "learn",
        "inspect",
        "routing",
        "--workdir",
        "/tmp/proj",
    ])
    .expect("parse learn inspect routing --workdir");
    assert!(matches!(
        cli.command,
        Some(Command::Learn {
            cmd: LearnCmd::Inspect {
                subsystem: InspectSubsystem::Routing { workdir: Some(ref wd) },
            },
        }) if wd == std::path::Path::new("/tmp/proj")
    ));
}

#[test]
fn cli_parses_learn_inspect_budget() {
    let cli = Cli::try_parse_from(["roko", "learn", "inspect", "budget"])
        .expect("parse learn inspect budget");
    assert!(matches!(
        cli.command,
        Some(Command::Learn {
            cmd: LearnCmd::Inspect {
                subsystem: InspectSubsystem::Budget { workdir: None },
            },
        })
    ));
}

#[test]
fn cli_bench_swe_requires_agent_mode() {
    // A forgotten flag must not silently run the gold control.
    let err = Cli::try_parse_from(["roko", "bench", "swe"])
        .expect_err("bench swe without --agent-mode should not parse");
    assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);

    let cli = Cli::try_parse_from(["roko", "bench", "swe", "--agent-mode", "command"])
        .expect("parse bench swe --agent-mode command");
    assert!(matches!(
        cli.command,
        Some(Command::Bench {
            cmd: BenchCmd::Swe {
                agent_mode: roko_cli::bench::SweAgentMode::Command,
                ..
            },
        })
    ));
}

#[test]
fn cli_parses_learn_tune_deprecated_alias() {
    let cli = Cli::try_parse_from(["roko", "learn", "tune", "routing"])
        .expect("parse learn tune routing");
    assert!(matches!(
        cli.command,
        Some(Command::Learn {
            cmd: LearnCmd::Tune {
                ref subsystem,
                dry_run: false,
                workdir: None,
            },
        }) if subsystem == "routing"
    ));
}

#[test]
fn cli_parses_learn_tune_dry_run_flag() {
    let cli = Cli::try_parse_from(["roko", "learn", "tune", "--dry-run", "gates"])
        .expect("parse learn tune --dry-run");
    assert!(matches!(
        cli.command,
        Some(Command::Learn {
            cmd: LearnCmd::Tune {
                ref subsystem,
                dry_run: true,
                workdir: None,
            },
        }) if subsystem == "gates"
    ));
}

#[test]
fn cli_parses_config_preset_gates() {
    let cli = Cli::try_parse_from(["roko", "config", "preset", "gates"])
        .expect("parse config preset gates");
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::Preset {
                cmd: ConfigPresetCmd::Gates {
                    dry_run: false,
                    yes: false,
                    global: false,
                    ..
                },
            },
        })
    ));
}

#[test]
fn cli_parses_config_preset_routing_dry_run() {
    let cli = Cli::try_parse_from(["roko", "config", "preset", "routing", "--dry-run"])
        .expect("parse config preset routing --dry-run");
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::Preset {
                cmd: ConfigPresetCmd::Routing {
                    dry_run: true,
                    yes: false,
                    ..
                },
            },
        })
    ));
}

#[test]
fn cli_parses_config_preset_model_with_name() {
    let cli = Cli::try_parse_from(["roko", "config", "preset", "model", "sonnet", "--yes"])
        .expect("parse config preset model sonnet --yes");
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::Preset {
                cmd: ConfigPresetCmd::Model {
                    ref name,
                    yes: true,
                    dry_run: false,
                    ..
                },
            },
        }) if name == "sonnet"
    ));
}

#[test]
fn cli_parses_config_preset_budget_global() {
    let cli = Cli::try_parse_from(["roko", "config", "preset", "budget", "--global"])
        .expect("parse config preset budget --global");
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::Preset {
                cmd: ConfigPresetCmd::Budget { global: true, .. },
            },
        })
    ));
}

#[test]
fn cli_parses_top_level_tune_deprecated() {
    let cli = Cli::try_parse_from(["roko", "tune", "gates"]).expect("parse top-level tune gates");
    assert!(matches!(
        cli.command,
        Some(Command::Tune(TuneCmd::Gates { workdir: None }))
    ));
}

#[test]
fn force_model_alias_arms_the_existing_highest_precedence_override() {
    let cli = Cli::try_parse_from(["roko", "--force-model", "model-b", "status"])
        .expect("parse --force-model alias");
    assert_eq!(cli.model.as_deref(), Some("model-b"));
}

#[test]
fn cli_parses_positional_prompt() {
    let cli = Cli::try_parse_from(["roko", "fix the bug"]).unwrap();
    assert_eq!(cli.prompt.as_deref(), Some("fix the bug"));
    assert!(cli.command.is_none());
}

/// bug-17f0e4: a bare word that names no subcommand is an unknown
/// command, not a one-shot prompt, with `--help` or another subcommand
/// after it too.
#[test]
fn cli_rejects_unknown_single_word_command() {
    use clap::error::ErrorKind;

    for args in [
        &["roko", "dreem"][..],
        &["roko", "dream", "--help"],
        &["roko", "stauts", "--json"],
        &["roko", "fix", "status"],
    ] {
        let err = Cli::try_parse_from(args).expect_err("a bare word is not a prompt");
        assert_eq!(err.kind(), ErrorKind::InvalidSubcommand, "{args:?}");
    }
    let message = Cli::try_parse_from(["roko", "stauts"])
        .expect_err("a typo")
        .to_string();
    assert!(
        message.contains("unrecognized subcommand 'stauts'"),
        "{message}"
    );
    assert!(message.contains("'roko status'"), "{message}");
    assert!(message.contains("'roko run stauts'"), "{message}");
    let message = Cli::try_parse_from(["roko", "dream"])
        .expect_err("a nested command")
        .to_string();
    assert!(message.contains("'roko knowledge dream'"), "{message}");
    // A stale `roko dream run` fails too, before any agent runs.
    assert!(Cli::try_parse_from(["roko", "dream", "run"]).is_err());

    // Prompts of several words and `roko run <word>` still parse.
    let cli = Cli::try_parse_from(["roko", "fix the bug"]).expect("a quoted prompt");
    assert_eq!(cli.prompt.as_deref(), Some("fix the bug"));
    let cli = Cli::try_parse_from(["roko", "run", "fix"]).expect("a one-word run");
    assert!(matches!(cli.command, Some(Command::Run { .. })));
}

/// bug-8589fc: an unknown first word is the error whatever follows it,
/// not the missing prompt of the `run` clap parses after it.
#[test]
fn dream_run_reports_an_unknown_command() {
    use clap::error::ErrorKind;

    for args in [
        &["roko", "dream", "run"][..],
        &["roko", "--json", "dream", "run"],
        &["roko", "--config", "roko.toml", "dream", "run"],
        &["roko", "--force-model=m", "dream", "run"],
    ] {
        let error = try_parse_cli(args).expect_err("dream is not a command");
        assert_eq!(error.kind(), ErrorKind::InvalidSubcommand, "{args:?}");
        let message = error.to_string();
        assert!(message.contains("subcommand 'dream'"), "{message}");
        assert!(message.contains("'roko knowledge dream'"), "{message}");
    }
    // An error about a real command stays that command's error.
    for args in [&["roko", "run"][..], &["roko", "--model", "dream", "run"]] {
        let error = try_parse_cli(args).expect_err("run needs a prompt");
        let kind = error.kind();
        assert_eq!(kind, ErrorKind::MissingRequiredArgument, "{args:?}");
    }
    assert!(try_parse_cli(["roko", "status"]).is_ok());
}

#[test]
fn cli_parses_run_subcommand() {
    let cli = Cli::try_parse_from(["roko", "run", "do something"]).unwrap();
    assert!(matches!(cli.command, Some(Command::Run { .. })));
}

#[test]
fn cli_parses_run_flags() {
    let cli = Cli::try_parse_from([
        "roko",
        "run",
        "--plan",
        "--complexity",
        "medium",
        "--dry-run",
        "--workdir",
        "/tmp/run-workdir",
        "--provider",
        "openai",
        "--yes",
        "--no-cascade",
        "--context",
        "src/lib.rs",
        "--max-retries",
        "2",
        "add",
        "login",
    ])
    .unwrap();
    match cli.command {
        Some(Command::Run {
            prompt,
            plan,
            dry_run,
            yes,
            complexity: Some(complexity),
            context,
            no_cascade,
            workdir: Some(workdir),
            provider: Some(provider),
            max_retries: Some(max_retries),
            domain: None,
            serve,
            share,
            fresh,
            resume_plan,
        }) => {
            assert_eq!(prompt, vec!["add".to_string(), "login".to_string()]);
            assert!(plan);
            assert!(dry_run);
            assert!(yes);
            assert_eq!(complexity, RunComplexity::Medium);
            assert_eq!(context, vec![PathBuf::from("src/lib.rs")]);
            assert!(no_cascade);
            assert_eq!(workdir, PathBuf::from("/tmp/run-workdir"));
            assert_eq!(provider, "openai");
            assert_eq!(max_retries, 2);
            assert!(!serve && !share && !fresh);
            assert_eq!(resume_plan, None);
        }
        other => panic!("expected run command, got {other:?}"),
    }
}

/// 9121: `roko run --domain <label>` names the one-task run's work domain.
#[test]
fn cli_parses_run_domain() {
    let cli =
        Cli::try_parse_from(["roko", "run", "--domain", "research", "summarise", "it"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Run { domain: Some(ref domain), .. }) if domain == "research"
    ));
}

#[test]
fn cli_parses_run_of_a_plan_directory() {
    let cli = Cli::try_parse_from(["roko", "run", "plans/add-login", "--fresh"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Run { ref prompt, fresh: true, .. }) if prompt == &vec!["plans/add-login".to_string()]
    ));
    let cli = Cli::try_parse_from(["roko", "run", "plans/add-login", "--resume-plan"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Run {
            resume_plan: Some(ref path),
            ..
        }) if path == &PathBuf::from(".roko/state/state-snapshot.json")
    ));
}

#[test]
fn removed_commands_still_parse_so_they_can_report_the_migration() {
    // `roko do` keeps its old flags (and its `d` alias), so an old script
    // reaches the removal error instead of a usage error.
    let cli = Cli::try_parse_from([
        "roko",
        "do",
        "--plan",
        "--complexity",
        "medium",
        "--ghost",
        "--compare",
        "--continue",
        "work-123",
        "do",
        "something",
    ])
    .unwrap();
    assert!(matches!(cli.command, Some(Command::Do { .. })));
    let cli = Cli::try_parse_from(["roko", "d", "fix it"]).unwrap();
    assert!(matches!(cli.command, Some(Command::Do { .. })));
    // `develop` printed its migration for a month (since 2026-09-04) and is
    // now an unknown subcommand.
    assert!(try_parse_cli(["roko", "develop", "build", "it"]).is_err());
    for args in [
        vec!["roko", "prd"],
        vec!["roko", "prd", "list"],
        vec!["roko", "prd", "idea", "wire", "the", "runner"],
        vec!["roko", "prd", "draft", "new", "a", "title"],
        vec!["roko", "prd", "plan", "my-prd", "--dry-run"],
    ] {
        let cli = Cli::try_parse_from(args.clone()).unwrap();
        assert!(
            matches!(cli.command, Some(Command::Prd { .. })),
            "{args:?} should parse as the removed prd command"
        );
    }
}

#[test]
fn removed_commands_stay_out_of_help() {
    let mut cmd = Cli::command();
    cmd.build();
    for name in ["do", "prd"] {
        let sub = cmd
            .get_subcommands()
            .find(|sub| sub.get_name() == name)
            .unwrap_or_else(|| panic!("{name} should still parse"));
        assert!(sub.is_hide_set(), "{name} should be hidden from --help");
    }
}

#[test]
fn cli_parses_init_subcommand() {
    let cli = Cli::try_parse_from(["roko", "init", "/tmp/project"]).unwrap();
    match cli.command {
        Some(Command::Init {
            path,
            cloud,
            profile,
            demo,
        }) => {
            assert_eq!(path, Some(PathBuf::from("/tmp/project")));
            assert!(!cloud);
            assert!(profile.is_none());
            assert!(!demo);
        }
        other => panic!("expected init command, got {other:?}"),
    }
}

#[test]
fn cli_parses_init_cloud_flag() {
    let cli = Cli::try_parse_from(["roko", "init", "--cloud"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Init { cloud: true, .. })
    ));
}

#[test]
fn cli_parses_init_demo_flag() {
    let cli = Cli::try_parse_from(["roko", "init", "--demo"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Init { demo: true, .. })
    ));
}

#[test]
fn cli_parses_status_subcommand() {
    let cli = Cli::try_parse_from(["roko", "status"]).unwrap();
    assert!(matches!(cli.command, Some(Command::Status { .. })));
}

#[test]
fn cli_parses_github_status_subcommand() {
    let cli =
        Cli::try_parse_from(["roko", "github", "status", "--workdir", "/tmp/project"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Github {
            cmd: commands::github::GithubCmd::Status { workdir: Some(_) }
        })
    ));
}

#[test]
fn cli_parses_status_quick_flag() {
    let cli = Cli::try_parse_from(["roko", "status", "--quick"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Status { quick: true, .. })
    ));
}

#[test]
fn cli_rejects_status_quick_with_surfaces() {
    let result = Cli::try_parse_from(["roko", "status", "--quick", "--surfaces"]);
    assert!(result.is_err(), "--quick and --surfaces should conflict");
}

#[test]
fn cli_rejects_status_quick_with_cfactor() {
    let result = Cli::try_parse_from(["roko", "status", "--quick", "--cfactor"]);
    assert!(result.is_err(), "--quick and --cfactor should conflict");
}

#[test]
fn cli_parses_doctor_subcommand() {
    let cli = Cli::try_parse_from([
        "roko",
        "doctor",
        "--workdir",
        "/tmp/project",
        "--serve-url",
        "http://localhost:9090",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Doctor {
            subject: None,
            workdir: Some(_),
            serve_url: Some(_),
            fix: false,
        })
    ));
}

#[test]
fn cli_parses_doctor_disk_subreport() {
    let cli = Cli::try_parse_from(["roko", "doctor", "disk", "--workdir", "/tmp/project"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Doctor {
            subject: Some(DoctorSubject::Disk),
            workdir: Some(_),
            ..
        })
    ));
}

#[test]
fn cli_parses_doctor_disk_fix() {
    let cli = Cli::try_parse_from(["roko", "doctor", "disk", "--fix"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Doctor {
            subject: Some(DoctorSubject::Disk),
            fix: true,
            ..
        })
    ));
}

#[test]
fn cli_parses_doctor_network_subreport() {
    let cli =
        Cli::try_parse_from(["roko", "doctor", "network", "--workdir", "/tmp/project"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Doctor {
            subject: Some(DoctorSubject::Network),
            workdir: Some(_),
            ..
        })
    ));
}

#[test]
fn cli_cache_prune_is_dry_run_by_default() {
    let cli = Cli::try_parse_from(["roko", "cache", "prune"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Cache {
            cmd: CacheCmd::Prune { apply: false, .. }
        })
    ));
}

#[test]
fn cli_cache_prune_requires_explicit_apply_flag_for_mutation() {
    let cli = Cli::try_parse_from([
        "roko",
        "cache",
        "prune",
        "--apply",
        "--target-budget-gb",
        "64",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Cache {
            cmd: CacheCmd::Prune {
                apply: true,
                target_budget_gb: 64,
                ..
            }
        })
    ));
}

#[test]
fn cli_parses_acp_subcommand() {
    let cli = Cli::try_parse_from([
        "roko",
        "acp",
        "--workdir",
        "/tmp/project",
        "--profile",
        "editor",
        "--config",
        "/tmp/project/roko.toml",
        "--global-config",
        "/tmp/global-roko.toml",
        "--log-file",
        ".roko/editor-acp.log",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Acp {
            workdir,
            profile,
            config: Some(config),
            global_config: Some(global_config),
            log_file,
        }) if workdir == PathBuf::from("/tmp/project")
            && profile == "editor"
            && config == PathBuf::from("/tmp/project/roko.toml")
            && global_config == PathBuf::from("/tmp/global-roko.toml")
            && log_file == PathBuf::from(".roko/editor-acp.log")
    ));
}

#[test]
fn cli_parses_agent_serve_subcommand() {
    let cli = Cli::try_parse_from([
        "roko",
        "agent",
        "serve",
        "--agent-id",
        "demo-1",
        "--bind",
        "127.0.0.1:7777",
        "--relay-url",
        "https://relay.example",
        "--chain-rpc-url",
        "https://rpc.example",
        "--identity-registry",
        "0x1234",
        "--passport-id",
        "7",
        "--wallet-key",
        "0xdeadbeef",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Agent {
            cmd: AgentCmd::Serve(agent_serve::AgentServeArgs {
                agent_id,
                bind,
                relay_url: Some(_),
                chain_rpc_url: Some(_),
                identity_registry: Some(_),
                passport_id: Some(_),
                wallet_key: Some(_),
                ..
            }),
        }) if agent_id == "demo-1" && bind == "127.0.0.1:7777"
    ));
}

#[test]
fn cli_parses_inject_subcommand() {
    let cli = Cli::try_parse_from([
        "roko",
        "inject",
        "session-1",
        "stop doing that",
        "--kind",
        "directive",
    ])
    .unwrap();
    assert!(matches!(cli.command, Some(Command::Inject { .. })));
}

// -- inject fails closed (#325, gap-f118b3) --
// With no plan run listening, a valid inject request reaches nothing: it
// exits non-zero and writes nothing, no control.json or inject.json that
// nothing reads, and no substrate (engrams.jsonl) entry.

#[tokio::test]
async fn inject_fail_closed_directive() {
    let tmp = tempfile::tempdir().unwrap();
    let roko_dir = tmp.path().join(".roko");
    std::fs::create_dir_all(&roko_dir).unwrap();
    let cli = Cli::try_parse_from(["roko", "inject", "sess-1", "do something"]).unwrap();
    let code = commands::util::cmd_inject(
        &cli,
        "sess-1".into(),
        "directive",
        "do something".into(),
        Some(tmp.path().to_path_buf()),
    )
    .await
    .unwrap();
    assert_eq!(
        code, EXIT_FAILURE,
        "no transport reaches a live executor, so a directive fails closed"
    );
    assert!(
        !roko_dir.join("state").exists(),
        "a request that delivered nothing writes no control or inject file"
    );
    // No signal log should be created.
    assert!(
        !roko_dir.join("engrams.jsonl").exists(),
        "no substrate write should occur"
    );
}

#[tokio::test]
async fn inject_fail_closed_abort() {
    let tmp = tempfile::tempdir().unwrap();
    let cli = Cli::try_parse_from(["roko", "inject", "sess-1", "", "--kind", "abort"]).unwrap();
    let code = commands::util::cmd_inject(
        &cli,
        "sess-1".into(),
        "abort",
        String::new(),
        Some(tmp.path().to_path_buf()),
    )
    .await
    .unwrap();
    assert_eq!(
        code, EXIT_FAILURE,
        "an abort that reaches no executor fails closed"
    );
    assert!(!tmp.path().join(".roko/state").exists());
}

#[tokio::test]
async fn inject_fail_closed_context() {
    let tmp = tempfile::tempdir().unwrap();
    let cli = Cli::try_parse_from(["roko", "inject", "sess-1", "ctx data"]).unwrap();
    let code = commands::util::cmd_inject(
        &cli,
        "sess-1".into(),
        "context",
        "ctx data".into(),
        Some(tmp.path().to_path_buf()),
    )
    .await
    .unwrap();
    assert_eq!(
        code, EXIT_FAILURE,
        "context that reaches no executor fails closed"
    );
    assert!(!tmp.path().join(".roko/state").exists());
}

#[tokio::test]
async fn inject_fail_closed_validation_more_specific() {
    // Malformed input (empty session) should still produce a validation
    // error, not the generic transport-unavailable error.
    let tmp = tempfile::tempdir().unwrap();
    let cli = Cli::try_parse_from(["roko", "inject", "sess-1", "payload"]).unwrap();
    let result = commands::util::cmd_inject(
        &cli,
        String::new(), // empty session
        "directive",
        "payload".into(),
        Some(tmp.path().to_path_buf()),
    )
    .await;
    assert!(
        result.is_err(),
        "empty session must produce an error, not a transport failure"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("session_id"),
        "error must mention session_id: {msg}"
    );
}

#[tokio::test]
async fn inject_fail_closed_json_output() {
    let tmp = tempfile::tempdir().unwrap();
    let cli = Cli::try_parse_from(["roko", "--json", "inject", "sess-1", "payload"]).unwrap();
    assert!(cli.json, "json flag must be set");
    let code = commands::util::cmd_inject(
        &cli,
        "sess-1".into(),
        "directive",
        "payload".into(),
        Some(tmp.path().to_path_buf()),
    )
    .await
    .unwrap();
    assert_eq!(
        code, EXIT_FAILURE,
        "inject --json reports inject_transport_unavailable and fails"
    );
}

/// gap-f118b3: success needs the addressed executor's acknowledgement.
/// With no plan run listening, every kind exits non-zero and leaves
/// `.roko/state/` as it was.
#[tokio::test]
async fn inject_fails_without_executor_ack() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = tmp.path().join(".roko/state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let cli = Cli::try_parse_from(["roko", "inject", "run-1", "stop"]).unwrap();
    for (kind, payload) in [("directive", "stop"), ("context", "ctx"), ("abort", "")] {
        let code = commands::util::cmd_inject(
            &cli,
            "run-1".into(),
            kind,
            payload.into(),
            Some(tmp.path().to_path_buf()),
        )
        .await
        .unwrap();
        assert_eq!(code, EXIT_FAILURE, "{kind} succeeded without an ack");
    }
    assert_eq!(std::fs::read_dir(&state_dir).unwrap().count(), 0);
}

/// gap-f118b3: `roko inject` exits 0 only once the plan run its session
/// names acknowledges the request; a session no listening run has fails.
#[cfg(unix)]
#[tokio::test]
async fn inject_succeeds_only_on_the_runs_acknowledgement() {
    use roko_cli::execution_control::{CommandAckStatus, ExecutionCommandSender, ack_for};
    use roko_cli::inject::{InjectLink, InjectTarget, start_inject_server};

    let tmp = tempfile::tempdir().unwrap();
    let (run_commands, mut command_rx, ack_tx, acks) =
        ExecutionCommandSender::channel("graph-engine");
    let target: InjectTarget =
        std::sync::Arc::new(|session: &str| (session == "plan-1").then(|| session.to_string()));
    let link = InjectLink {
        commands: run_commands,
        acks,
        target,
        answer_timeout: Duration::from_secs(5),
    };
    let _server = start_inject_server(tmp.path(), link).unwrap();
    let run = tokio::spawn(async move {
        let command = command_rx.recv().await.unwrap();
        let ack = ack_for(&command, CommandAckStatus::Accepted, Some("queued".into()));
        ack_tx.send(ack).await.unwrap();
    });
    let cli = Cli::try_parse_from(["roko", "inject", "plan-1", "keep going"]).unwrap();
    let workdir = || Some(tmp.path().to_path_buf());

    let accepted = commands::util::cmd_inject(
        &cli,
        "plan-1".into(),
        "directive",
        "keep going".into(),
        workdir(),
    )
    .await
    .unwrap();
    assert_eq!(accepted, EXIT_SUCCESS);
    run.await.unwrap();
    let unknown = commands::util::cmd_inject(
        &cli,
        "plan-9".into(),
        "directive",
        "keep going".into(),
        workdir(),
    )
    .await
    .unwrap();
    assert_eq!(unknown, EXIT_FAILURE, "no running plan of that name");
}

#[test]
fn cli_parses_plan_list() {
    let cli = Cli::try_parse_from(["roko", "plan", "list"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Plan {
            cmd: PlanCmd::List { .. }
        })
    ));
}

#[test]
fn read_only_plan_commands_do_not_rebuild_indexes() {
    let commands = [
        Cli::try_parse_from(["roko", "plan", "list"]).unwrap(),
        Cli::try_parse_from(["roko", "plan", "show", "P34"]).unwrap(),
        Cli::try_parse_from(["roko", "plan", "validate", "plans/"]).unwrap(),
        Cli::try_parse_from(["roko", "plan", "index", "--check"]).unwrap(),
    ];

    for cli in commands {
        let Some(Command::Plan { cmd }) = cli.command else {
            panic!("expected a plan command");
        };
        assert!(!cmd.should_rebuild_indexes());
    }
}

#[test]
fn mutating_plan_commands_rebuild_indexes_but_dry_runs_do_not() {
    let mutating = [
        Cli::try_parse_from(["roko", "plan", "create", "my-plan", "--title", "My Plan"]).unwrap(),
        Cli::try_parse_from(["roko", "plan", "generate", "fix", "the", "bug"]).unwrap(),
        Cli::try_parse_from(["roko", "plan", "regenerate", "plans/my-plan"]).unwrap(),
    ];
    for cli in mutating {
        let Some(Command::Plan { cmd }) = cli.command else {
            panic!("expected a plan command");
        };
        assert!(cmd.should_rebuild_indexes());
        assert!(should_rebuild_plan_indexes(
            cmd.should_rebuild_indexes(),
            Some(EXIT_SUCCESS)
        ));
        assert!(!should_rebuild_plan_indexes(
            cmd.should_rebuild_indexes(),
            Some(1)
        ));
        assert!(!should_rebuild_plan_indexes(
            cmd.should_rebuild_indexes(),
            None
        ));
    }

    let dry_runs = [
        Cli::try_parse_from(["roko", "plan", "run", "plans/", "--dry-run"]).unwrap(),
        Cli::try_parse_from(["roko", "plan", "regenerate", "plans/my-plan", "--dry-run"]).unwrap(),
    ];
    for cli in dry_runs {
        let Some(Command::Plan { cmd }) = cli.command else {
            panic!("expected a plan command");
        };
        assert!(!cmd.should_rebuild_indexes());
    }
}

/// 1222: a successful plan run leaves the indexes in the operator's checkout
/// alone. It changes no plan definition; its results go to the plan branch
/// and `.roko/`.
#[test]
fn plan_run_does_not_rebuild_indexes() {
    let cli = Cli::try_parse_from(["roko", "plan", "run", "plans/"]).unwrap();
    let Some(Command::Plan { cmd }) = cli.command else {
        panic!("expected a plan command");
    };
    assert!(!cmd.should_rebuild_indexes());
    assert!(!should_rebuild_plan_indexes(
        cmd.should_rebuild_indexes(),
        Some(EXIT_SUCCESS)
    ));
}

#[test]
fn successful_command_propagates_index_rebuild_failure() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join(".roko"), b"not a directory").unwrap();

    let error = finish_with_index_rebuild(Ok(EXIT_SUCCESS), tmp.path(), true).unwrap_err();

    assert!(
        error.to_string().contains("Not a directory")
            || error.to_string().contains("not a directory"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn failed_or_read_only_command_does_not_attempt_index_rebuild() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join(".roko"), b"not a directory").unwrap();

    let primary =
        finish_with_index_rebuild(Err(anyhow!("primary command failed")), tmp.path(), true)
            .unwrap_err();
    let read_only = finish_with_index_rebuild(Ok(EXIT_SUCCESS), tmp.path(), false).unwrap();

    assert_eq!(primary.to_string(), "primary command failed");
    assert_eq!(read_only, EXIT_SUCCESS);
}

#[test]
fn nonzero_primary_result_is_preserved_without_index_rebuild() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join(".roko"), b"not a directory").unwrap();

    let exit_code = finish_with_index_rebuild(Ok(7), tmp.path(), true).unwrap();

    assert_eq!(exit_code, 7);
}

#[test]
fn plan_create_rebuild_uses_the_command_workdir() {
    let cli = Cli::try_parse_from([
        "roko",
        "plan",
        "create",
        "my-plan",
        "--title",
        "My Plan",
        "--workdir",
        "selected-workspace",
    ])
    .unwrap();
    let Some(Command::Plan { ref cmd }) = cli.command else {
        panic!("expected a plan command");
    };

    assert_eq!(
        cmd.index_rebuild_workdir(&cli),
        PathBuf::from("selected-workspace")
    );
}

/// bug-17544e: `plan run --help` does not advertise the flags `plan run`
/// rejects (gap-d60281). They still parse, so `plan run` can say what to
/// use instead.
#[test]
fn plan_run_help_hides_the_flags_plan_run_rejects() {
    let help = Cli::try_parse_from(["roko", "plan", "run", "--help"])
        .expect_err("help")
        .to_string();
    assert!(help.contains("--log-file"), "{help}");
    for flag in [
        "--skip-preflight",
        "--screenshots",
        "--screenshot-interval",
        "--screenshot-dir",
        "--batch-size",
    ] {
        assert!(!help.contains(flag), "{flag} is advertised:\n{help}");
    }
    assert!(Cli::try_parse_from(["roko", "plan", "run", "plans", "--batch-size", "2"]).is_ok());
    assert!(Cli::try_parse_from(["roko", "plan", "run", "plans", "--skip-preflight"]).is_ok());
}

#[test]
fn cli_parses_plan_create() {
    let cli =
        Cli::try_parse_from(["roko", "plan", "create", "my-plan", "--title", "My Plan"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Plan {
            cmd: PlanCmd::Create { .. }
        })
    ));
}

#[test]
fn cli_parses_non_mutating_plan_index_check() {
    let cli = Cli::try_parse_from(["roko", "plan", "index", "--check", "--workdir", "."]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Plan {
            cmd: PlanCmd::Index {
                check: true,
                workdir: Some(_),
            }
        })
    ));
}

/// backlog 2118: `roko plan budget raise <plan> --to <usd>`; the amount is
/// required.
#[test]
fn cli_parses_plan_budget_raise() {
    use commands::plan::PlanBudgetCmd;

    let cli =
        Cli::try_parse_from(["roko", "plan", "budget", "raise", "p1", "--to", "0.5"]).unwrap();
    let Some(Command::Plan {
        cmd: PlanCmd::Budget { cmd },
    }) = cli.command
    else {
        panic!("expected plan budget");
    };
    let PlanBudgetCmd::Raise { plan_id, to, .. } = cmd;
    assert_eq!((plan_id.as_str(), to), ("p1", 0.5));
    let missing = Cli::try_parse_from(["roko", "plan", "budget", "raise", "p1"]);
    assert!(missing.is_err(), "--to is required");
}

#[test]
fn cli_parses_plan_resume_flag() {
    let cli = Cli::try_parse_from(["roko", "plan", "run", "plans", "--resume-plan"]).unwrap();
    let Some(Command::Plan {
        cmd: PlanCmd::Run { resume_plan, .. },
    }) = cli.command
    else {
        panic!("expected plan run");
    };
    assert_eq!(
        resume_plan,
        Some(PathBuf::from(".roko/state/state-snapshot.json"))
    );
}

#[test]
fn cli_parses_continuous_screenshot_options() {
    let cli = Cli::try_parse_from([
        "roko",
        "plan",
        "run",
        "plans",
        "--screenshots",
        "--screenshot-interval",
        "30",
        "--screenshot-dir",
        "/private/tmp/roko-evidence",
    ])
    .unwrap();
    let Some(Command::Plan {
        cmd:
            PlanCmd::Run {
                screenshots,
                screenshot_interval,
                screenshot_dir,
                ..
            },
    }) = cli.command
    else {
        panic!("expected plan run");
    };
    assert!(screenshots);
    assert_eq!(screenshot_interval, 30);
    assert_eq!(
        screenshot_dir,
        Some(PathBuf::from("/private/tmp/roko-evidence"))
    );
}

#[test]
fn cli_rejects_zero_screenshot_interval() {
    assert!(
        Cli::try_parse_from([
            "roko",
            "plan",
            "run",
            "plans",
            "--screenshots",
            "--screenshot-interval",
            "0",
        ])
        .is_err()
    );
}

#[test]
fn cli_parses_plan_resume_flag_documented_alias() {
    let cli = Cli::try_parse_from(["roko", "plan", "run", "plans", "--resume-state"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Plan {
            cmd: PlanCmd::Run {
                resume_plan: Some(_),
                ..
            }
        })
    ));
}

#[test]
fn cli_parses_plan_fresh_flag() {
    let cli = Cli::try_parse_from(["roko", "plan", "run", "plans", "--fresh"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Plan {
            cmd: PlanCmd::Run { fresh: true, .. }
        })
    ));
}

#[test]
fn cli_parses_plan_parallel_flags() {
    let cli = Cli::try_parse_from([
        "roko",
        "plan",
        "run",
        "plans",
        "--max-parallel-plans",
        "3",
        "--fail-fast",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Plan {
            cmd: PlanCmd::Run {
                max_parallel_plans: Some(3),
                fail_fast: true,
                ..
            }
        })
    ));
    let cli = Cli::try_parse_from(["roko", "plan", "run", "plans"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Plan {
            cmd: PlanCmd::Run {
                max_parallel_plans: None,
                fail_fast: false,
                ..
            }
        })
    ));
    assert!(
        Cli::try_parse_from(["roko", "plan", "run", "plans", "--max-parallel-plans", "0"]).is_err()
    );
}

/// gap-4ec59f: `--no-worktree-per-task` opts a run out of per-task
/// worktrees and conflicts with `--worktree-per-task`; `--promote` no
/// longer needs the flag (config may turn worktrees on) but conflicts with
/// the opt-out.
#[test]
fn cli_parses_the_worktree_per_task_opt_out() {
    let cli =
        Cli::try_parse_from(["roko", "plan", "run", "plans", "--no-worktree-per-task"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Plan {
            cmd: PlanCmd::Run {
                worktree_per_task: false,
                no_worktree_per_task: true,
                ..
            }
        })
    ));
    for conflicting in [
        ["--worktree-per-task", "--no-worktree-per-task"],
        ["--promote=release", "--no-worktree-per-task"],
    ] {
        let mut args = vec!["roko", "plan", "run", "plans"];
        args.extend(conflicting);
        assert!(Cli::try_parse_from(args).is_err(), "{conflicting:?}");
    }
    assert!(Cli::try_parse_from(["roko", "plan", "run", "plans", "--promote", "release"]).is_ok());
}

/// Decision 4115: `roko plan run <dir> --no-holdout` runs in maximize mode
/// for that run alone; without the flag the run follows roko.toml's
/// `[experiments] maximize`.
#[test]
fn no_holdout_flag_sets_maximize_mode() {
    use roko_cli::graph_execution::plan_runner::apply_run_switches;

    let cli = Cli::try_parse_from(["roko", "plan", "run", "plans", "--no-holdout"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Plan {
            cmd: PlanCmd::Run {
                no_holdout: true,
                ..
            }
        })
    ));
    let cli = Cli::try_parse_from(["roko", "plan", "run", "plans"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Plan {
            cmd: PlanCmd::Run {
                no_holdout: false,
                ..
            }
        })
    ));

    // The run's config: maximize with the flag, else as roko.toml says.
    let maximize = |toml: &str, no_holdout: bool| {
        let mut config = roko_core::config::schema::RokoConfig::from_toml(toml).unwrap();
        apply_run_switches(&mut config, false, no_holdout);
        config.experiments.maximize
    };
    assert!(maximize("", true));
    assert!(!maximize("", false));
    assert!(maximize("[experiments]\nmaximize = true\n", false));
}

#[test]
fn cli_parses_plan_force_resume_flag() {
    let cli = Cli::try_parse_from(["roko", "plan", "run", "plans", "--force-resume"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Plan {
            cmd: PlanCmd::Run {
                force_resume: true,
                ..
            }
        })
    ));
}

#[test]
fn cli_parses_resume_flag() {
    let cli = Cli::try_parse_from(["roko", "--resume", "sess-42"]).unwrap();
    assert_eq!(cli.resume.as_deref(), Some("sess-42"));
}

#[test]
fn apply_resume_session_override_adds_env_var() {
    let mut config = Config::default();
    apply_resume_session_override(&mut config, Some("sess-42".to_string()));
    assert_eq!(
        config
            .agent
            .env
            .iter()
            .find(|(key, _)| key == "ROKO_SESSION_ID")
            .map(|(_, value)| value.as_str()),
        Some("sess-42")
    );
}

#[test]
fn apply_resume_session_override_updates_existing_env_var() {
    let mut config = Config::default();
    config
        .agent
        .env
        .push(("ROKO_SESSION_ID".to_string(), "old".to_string()));
    apply_resume_session_override(&mut config, Some("  sess-99  ".to_string()));
    assert_eq!(
        config
            .agent
            .env
            .iter()
            .find(|(key, _)| key == "ROKO_SESSION_ID")
            .map(|(_, value)| value.as_str()),
        Some("sess-99")
    );
}

#[test]
fn effort_display() {
    assert_eq!(Effort::Low.to_string(), "low");
    assert_eq!(Effort::Medium.to_string(), "medium");
    assert_eq!(Effort::High.to_string(), "high");
    assert_eq!(Effort::Max.to_string(), "max");
}

#[test]
fn effort_value_enum_all_variants() {
    // Ensure all four variants parse.
    for name in &["low", "medium", "high", "max"] {
        let cli = Cli::try_parse_from(["roko", "--effort", name]).unwrap();
        assert!(cli.effort.is_some());
    }
}

#[test]
fn exit_code_constants() {
    assert_eq!(EXIT_SUCCESS, 0);
    assert_eq!(EXIT_AGENT_FAILURE, 1);
    assert_eq!(EXIT_SYSTEM_ERROR, 2);
}

#[test]
fn resolve_workdir_uses_repo_flag() {
    let cli = Cli::try_parse_from(["roko", "--repo", "/custom"]).unwrap();
    assert_eq!(resolve_workdir(&cli), PathBuf::from("/custom"));
}

#[test]
fn resolve_workdir_defaults_to_cwd() {
    let cli = Cli::try_parse_from(["roko"]).unwrap();
    let cwd = PathBuf::from(".").canonicalize().unwrap();
    let expected = enclosing_project_of_data_dir(&cwd).unwrap_or(cwd);
    assert_eq!(resolve_workdir(&cli), expected);
}

#[test]
fn resolve_workdir_keeps_a_workspace_nested_under_dot_roko() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().canonicalize().unwrap();
    let worktree = project.join(".roko").join("worktrees").join("x");
    std::fs::create_dir_all(worktree.join("src")).unwrap();
    // A git worktree has a `.git` file that points at the main repository.
    std::fs::write(worktree.join(".git"), "gitdir: /repo/.git/worktrees/x\n").unwrap();

    assert_eq!(enclosing_project_of_data_dir(&worktree), None);
    assert_eq!(enclosing_project_of_data_dir(&worktree.join("src")), None);
    let cli = Cli::try_parse_from(["roko", "--repo", worktree.to_str().unwrap()]).unwrap();
    assert_eq!(resolve_workdir(&cli), worktree);
}

#[test]
fn resolve_workdir_still_redirects_from_the_data_dir() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().canonicalize().unwrap();
    let data_dir = project.join(".roko");
    let state = data_dir.join("state");
    std::fs::create_dir_all(&state).unwrap();

    let expected = Some(project.clone());
    assert_eq!(enclosing_project_of_data_dir(&state), expected);
    assert_eq!(enclosing_project_of_data_dir(&data_dir), expected);
    assert_eq!(enclosing_project_of_data_dir(&project), None);
    // An explicit --repo is used as given, with a warning.
    let cli = Cli::try_parse_from(["roko", "--repo", state.to_str().unwrap()]).unwrap();
    assert_eq!(resolve_workdir(&cli), state);
}

#[test]
fn resolve_workdir_canonicalizes_existing_repo_flag() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("workspace");
    std::fs::create_dir_all(&repo).unwrap();
    let repo_arg = repo.join(".");
    let cli = Cli::try_parse_from(["roko", "--repo", repo_arg.to_str().unwrap()]).unwrap();

    assert_eq!(resolve_workdir(&cli), repo.canonicalize().unwrap());
}

#[test]
fn resolve_plans_dir_prefers_top_level_plans() {
    let tmp = tempdir().unwrap();
    let workdir = tmp.path();
    let canonical = workdir.join("plans");
    let fallback = workdir.join(".roko").join("plans");
    std::fs::create_dir_all(&canonical).unwrap();
    std::fs::create_dir_all(&fallback).unwrap();

    assert_eq!(resolve_plans_dir(workdir, None), canonical);
}

#[test]
fn resolve_plans_dir_falls_back_to_dot_roko_plans_that_hold_plans() {
    let tmp = tempdir().unwrap();
    let workdir = tmp.path();
    let fallback = workdir.join(".roko").join("plans");
    std::fs::create_dir_all(fallback.join("old-plan")).unwrap();
    std::fs::write(fallback.join("old-plan").join("tasks.toml"), "").unwrap();

    assert_eq!(resolve_plans_dir(workdir, None), fallback);
}

#[test]
fn resolve_plans_dir_ignores_an_empty_dot_roko_plans() {
    let tmp = tempdir().unwrap();
    let workdir = tmp.path();
    std::fs::create_dir_all(workdir.join(".roko").join("plans")).unwrap();

    assert_eq!(resolve_plans_dir(workdir, None), workdir.join("plans"));
}

#[test]
fn resolve_plans_dir_returns_canonical_when_neither_directory_exists() {
    let tmp = tempdir().unwrap();
    let workdir = tmp.path();
    let canonical = workdir.join("plans");

    assert_eq!(resolve_plans_dir(workdir, None), canonical);
}

#[test]
fn resolve_plans_dir_honors_explicit_path() {
    let tmp = tempdir().unwrap();
    let workdir = tmp.path();
    let explicit = workdir.join("custom-plans");

    assert_eq!(resolve_plans_dir(workdir, Some(&explicit)), explicit);
}

#[tokio::test]
async fn persist_capture_episode_records_learning_episode() {
    let dir = tempdir().unwrap();
    let workdir = dir.path();

    persist_capture_episode(
        workdir,
        "claude",
        Some("claude-sonnet-4-6"),
        "plan-generate",
        "plan:generate:demo",
        "write a plan",
        "# demo plan",
        true,
        321,
        Some("resume-123"),
    )
    .await
    .unwrap();

    let episodes_path = workdir.join(".roko").join("episodes.jsonl");
    let episodes = EpisodeLogger::read_all_lossy(&episodes_path).await.unwrap();
    assert_eq!(episodes.len(), 1);
    assert!(!workdir.join(".roko/learn/episodes.jsonl").exists());
    assert!(!workdir.join(".roko/memory/episodes.jsonl").exists());

    let episode = &episodes[0];
    assert_eq!(episode.agent_id, "claude");
    assert_eq!(episode.task_id, "plan:generate:demo");
    assert_eq!(episode.kind, "agent_turn");
    assert_eq!(episode.model, "claude-sonnet-4-6");
    assert!(episode.success);
    assert_eq!(
        episode.extra.get("task_kind"),
        Some(&serde_json::json!("plan-generate"))
    );
    assert_eq!(
        episode.extra.get("provider"),
        Some(&serde_json::json!("anthropic"))
    );
    assert_eq!(
        episode.extra.get("role"),
        Some(&serde_json::json!("Strategist"))
    );
    assert_eq!(
        episode.extra.get("task_category"),
        Some(&serde_json::json!("scaffolding"))
    );
    assert_eq!(
        episode.extra.get("complexity_band"),
        Some(&serde_json::json!("standard"))
    );
    assert_eq!(
        episode.extra.get("plan_id"),
        Some(&serde_json::json!("demo"))
    );
    assert_eq!(
        episode.extra.get("session_id"),
        Some(&serde_json::json!("resume-123"))
    );
}

#[test]
fn cli_parses_config_subcommand() {
    let cli = Cli::try_parse_from(["roko", "config", "show"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::Show {
                effective: false,
                ..
            }
        })
    ));
}

#[test]
fn cli_parses_config_show_effective() {
    let cli = Cli::try_parse_from(["roko", "config", "show", "--effective"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::Show {
                effective: true,
                ..
            }
        })
    ));
}

#[test]
fn cli_parses_config_wizard_alias() {
    let cli = Cli::try_parse_from(["roko", "config", "wizard"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::Init { .. }
        })
    ));
}

#[test]
fn cli_parses_check_secrets_subcommand() {
    let cli = Cli::try_parse_from(["roko", "config", "check-secrets"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::CheckSecrets { .. }
        })
    ));
}

#[test]
fn cli_parses_set_secret_subcommand() {
    let cli = Cli::try_parse_from(["roko", "config", "set-secret", "TOKEN", "value"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::SetSecret { .. }
        })
    ));
}

#[test]
fn cli_parses_config_secrets_subcommand() {
    let cli =
        Cli::try_parse_from(["roko", "config", "secrets", "get", "anthropic.api_key"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::Secrets { .. }
        })
    ));
}

#[test]
fn cli_parses_config_mcp_list_subcommand() {
    let cli = Cli::try_parse_from(["roko", "config", "mcp", "list"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::Mcp {
                cmd: ConfigMcpCmd::List { .. }
            }
        })
    ));
}

#[test]
fn cli_parses_config_mcp_test_subcommand() {
    let cli = Cli::try_parse_from(["roko", "config", "mcp", "test", "roko"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::Mcp {
                cmd: ConfigMcpCmd::Test { name, .. }
            }
        }) if name == "roko"
    ));
}

#[test]
fn cli_parses_config_mcp_add_subcommand() {
    let cli = Cli::try_parse_from([
        "roko",
        "config",
        "mcp",
        "add",
        "roko",
        "/bin/echo",
        "--",
        "hello",
        "world",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::Mcp {
                cmd: ConfigMcpCmd::Add {
                    name,
                    command,
                    args,
                    ..
                }
            }
        }) if name == "roko" && command == "/bin/echo" && args == vec!["hello".to_string(), "world".to_string()]
    ));
}

#[test]
fn cli_parses_replay_subcommand() {
    let cli = Cli::try_parse_from(["roko", "replay", "abcd1234"]).unwrap();
    assert!(matches!(cli.command, Some(Command::Replay { .. })));
}

#[test]
fn cli_parses_replay_from_event() {
    let cli = Cli::try_parse_from(["roko", "replay", "abcd1234", "--from-event", "3"]).unwrap();
    match cli.command {
        Some(Command::Replay {
            from_event, as_of, ..
        }) => {
            assert_eq!(from_event.as_deref(), Some("3"));
            assert!(as_of.is_none());
        }
        other => panic!("expected Replay, got {other:?}"),
    }
}

#[test]
fn cli_parses_replay_as_of_hidden() {
    let cli = Cli::try_parse_from(["roko", "replay", "abcd1234", "--as-of", "5"]).unwrap();
    match cli.command {
        Some(Command::Replay {
            from_event, as_of, ..
        }) => {
            assert!(from_event.is_none());
            assert_eq!(as_of.as_deref(), Some("5"));
        }
        other => panic!("expected Replay, got {other:?}"),
    }
}

#[test]
fn cli_replay_from_event_conflicts_with_as_of() {
    let result = Cli::try_parse_from([
        "roko",
        "replay",
        "abcd1234",
        "--from-event",
        "3",
        "--as-of",
        "5",
    ]);
    assert!(result.is_err(), "--from-event and --as-of must conflict");
}

#[test]
fn cli_parses_replay_with_workdir() {
    let cli =
        Cli::try_parse_from(["roko", "replay", "abcd1234", "--workdir", "/tmp/proj"]).unwrap();
    match cli.command {
        Some(Command::Replay { workdir, .. }) => {
            assert_eq!(workdir, Some(PathBuf::from("/tmp/proj")));
        }
        other => panic!("expected Replay, got {other:?}"),
    }
}

#[test]
fn cli_parses_replay_global_json() {
    let cli = Cli::try_parse_from(["roko", "--json", "replay", "abcd1234"]).unwrap();
    assert!(cli.json);
    assert!(matches!(cli.command, Some(Command::Replay { .. })));
}

#[test]
fn cli_parses_replay_global_repo() {
    let cli = Cli::try_parse_from(["roko", "--repo", "/tmp/proj", "replay", "abcd1234"]).unwrap();
    assert_eq!(cli.repo, Some(PathBuf::from("/tmp/proj")));
    assert!(matches!(cli.command, Some(Command::Replay { .. })));
}

#[test]
fn cli_replay_workdir_overrides_repo() {
    let cli = Cli::try_parse_from([
        "roko",
        "--repo",
        "/tmp/global",
        "replay",
        "abcd1234",
        "--workdir",
        "/tmp/local",
    ])
    .unwrap();
    // Global --repo is /tmp/global, but --workdir is /tmp/local.
    // cmd_replay uses workdir.unwrap_or_else(resolve_workdir), so
    // --workdir takes precedence.
    assert_eq!(cli.repo, Some(PathBuf::from("/tmp/global")));
    match cli.command {
        Some(Command::Replay { workdir, .. }) => {
            assert_eq!(workdir, Some(PathBuf::from("/tmp/local")));
        }
        other => panic!("expected Replay, got {other:?}"),
    }
}

#[test]
fn cli_parses_completions_subcommand() {
    let cli = Cli::try_parse_from(["roko", "completions", "zsh"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Completions {
            shell: CompletionShell::Zsh
        })
    ));
}

#[test]
fn cli_parses_deploy_railway_subcommand() {
    let cli = Cli::try_parse_from(["roko", "deploy", "railway"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Deploy {
            cmd: DeployCmd::Railway { .. }
        })
    ));
}

#[test]
fn cli_parses_deploy_railway_unsafe_public_flag() {
    let cli = Cli::try_parse_from(["roko", "deploy", "railway", "--unsafe-public"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Deploy {
            cmd: DeployCmd::Railway {
                unsafe_public: true,
                ..
            }
        })
    ));
}

#[test]
fn cli_parses_deploy_fly_subcommand() {
    let cli = Cli::try_parse_from(["roko", "deploy", "fly"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Deploy {
            cmd: DeployCmd::Fly { .. }
        })
    ));
}

#[test]
fn cli_parses_deploy_fly_unsafe_public_flag() {
    let cli = Cli::try_parse_from(["roko", "deploy", "fly", "--unsafe-public"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Deploy {
            cmd: DeployCmd::Fly {
                unsafe_public: true,
                ..
            }
        })
    ));
}

#[test]
fn cli_parses_deploy_docker_subcommand() {
    let cli = Cli::try_parse_from(["roko", "deploy", "docker"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Deploy {
            cmd: DeployCmd::Docker { .. }
        })
    ));
}

#[test]
fn cli_parses_deploy_docker_unsafe_public_flag() {
    let cli = Cli::try_parse_from(["roko", "deploy", "docker", "--unsafe-public"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Deploy {
            cmd: DeployCmd::Docker {
                unsafe_public: true,
                ..
            }
        })
    ));
}

#[test]
fn deploy_docker_push_flag_absent_by_default() {
    // Without --push the flag must default to false so docker push is NOT invoked.
    let cli = Cli::try_parse_from(["roko", "deploy", "docker"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Deploy {
            cmd: DeployCmd::Docker { push: false, .. }
        })
    ));
}

#[test]
fn deploy_docker_push_flag_set_when_requested() {
    // --push must be parsed and forwarded so cmd_deploy_docker can invoke
    // `docker push` after a successful build+tag.
    let cli = Cli::try_parse_from(["roko", "deploy", "docker", "--push"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Deploy {
            cmd: DeployCmd::Docker { push: true, .. }
        })
    ));
}

#[test]
fn deploy_railway_dry_run_flag() {
    let cli = Cli::try_parse_from(["roko", "deploy", "railway", "--dry-run"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Deploy {
            cmd: DeployCmd::Railway { dry_run: true, .. }
        })
    ));
}

#[test]
fn deploy_fly_dry_run_flag() {
    let cli = Cli::try_parse_from(["roko", "deploy", "fly", "--dry-run"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Deploy {
            cmd: DeployCmd::Fly { dry_run: true, .. }
        })
    ));
}

#[test]
fn deploy_fly_custom_app_and_region() {
    let cli = Cli::try_parse_from([
        "roko", "deploy", "fly", "--app", "my-app", "--region", "lhr",
    ])
    .unwrap();
    if let Some(Command::Deploy {
        cmd: DeployCmd::Fly { app, region, .. },
    }) = cli.command
    {
        assert_eq!(app, "my-app");
        assert_eq!(region, "lhr");
    } else {
        panic!("expected Deploy Fly");
    }
}

#[test]
fn deploy_fly_defaults() {
    let cli = Cli::try_parse_from(["roko", "deploy", "fly"]).unwrap();
    if let Some(Command::Deploy {
        cmd:
            DeployCmd::Fly {
                app,
                region,
                dockerfile,
                health_path,
                volume_source,
                volume_destination,
                force,
                dry_run,
                ..
            },
    }) = cli.command
    {
        assert_eq!(app, "roko-agent");
        assert_eq!(region, "iad");
        assert_eq!(dockerfile, "Dockerfile");
        assert_eq!(health_path, "/health");
        assert_eq!(volume_source, "roko_data");
        assert_eq!(volume_destination, "/data/.roko");
        assert!(!force);
        assert!(!dry_run);
    } else {
        panic!("expected Deploy Fly");
    }
}

#[test]
fn deploy_fly_force_flag() {
    let cli = Cli::try_parse_from(["roko", "deploy", "fly", "--force"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Deploy {
            cmd: DeployCmd::Fly { force: true, .. }
        })
    ));
}

#[test]
fn deploy_docker_dry_run_flag() {
    let cli = Cli::try_parse_from(["roko", "deploy", "docker", "--dry-run"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Deploy {
            cmd: DeployCmd::Docker { dry_run: true, .. }
        })
    ));
}

#[test]
fn deploy_docker_custom_dockerfile_and_target() {
    let cli = Cli::try_parse_from([
        "roko",
        "deploy",
        "docker",
        "--dockerfile",
        "docker/roko.Dockerfile",
        "--target",
        "distroless",
        "--image",
        "my-roko",
    ])
    .unwrap();
    if let Some(Command::Deploy {
        cmd:
            DeployCmd::Docker {
                dockerfile,
                target,
                image,
                ..
            },
    }) = cli.command
    {
        assert_eq!(dockerfile, "docker/roko.Dockerfile");
        assert_eq!(target.as_deref(), Some("distroless"));
        assert_eq!(image, "my-roko");
    } else {
        panic!("expected Deploy Docker");
    }
}

#[test]
fn deploy_docker_defaults() {
    let cli = Cli::try_parse_from(["roko", "deploy", "docker"]).unwrap();
    if let Some(Command::Deploy {
        cmd:
            DeployCmd::Docker {
                dockerfile,
                target,
                image,
                dry_run,
                ..
            },
    }) = cli.command
    {
        assert_eq!(dockerfile, "Dockerfile");
        assert!(target.is_none());
        assert_eq!(image, "roko");
        assert!(!dry_run);
    } else {
        panic!("expected Deploy Docker");
    }
}

#[test]
fn cli_parses_knowledge_query_subcommand() {
    let cli = Cli::try_parse_from(["roko", "knowledge", "query", "rust async"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Knowledge {
            cmd: KnowledgeCmd::Query { .. }
        })
    ));
}

#[test]
fn cli_parses_knowledge_stats_subcommand() {
    let cli = Cli::try_parse_from(["roko", "knowledge", "stats"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Knowledge {
            cmd: KnowledgeCmd::Stats { .. }
        })
    ));
}

#[test]
fn cli_parses_knowledge_gc_subcommand() {
    let cli = Cli::try_parse_from(["roko", "knowledge", "gc"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Knowledge {
            cmd: KnowledgeCmd::Gc { .. }
        })
    ));
}

#[test]
fn cli_parses_knowledge_backup_subcommand() {
    let cli = Cli::try_parse_from(["roko", "knowledge", "backup", "/tmp/neuro-backup"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Knowledge {
            cmd: KnowledgeCmd::Backup { destination, force, .. }
        }) if destination == PathBuf::from("/tmp/neuro-backup") && !force
    ));
}

#[test]
fn cli_parses_knowledge_export_and_import_with_safe_defaults() {
    let export =
        Cli::try_parse_from(["roko", "knowledge", "export", "/tmp/knowledge-export.jsonl"])
            .unwrap();
    assert!(matches!(
        export.command,
        Some(Command::Knowledge {
            cmd: KnowledgeCmd::Export {
                output,
                force: false,
                top_n: None,
                ..
            }
        }) if output == PathBuf::from("/tmp/knowledge-export.jsonl")
    ));

    let import =
        Cli::try_parse_from(["roko", "knowledge", "import", "/tmp/knowledge-export.jsonl"])
            .unwrap();
    assert!(matches!(
        import.command,
        Some(Command::Knowledge {
            cmd: KnowledgeCmd::Import {
                input,
                decay_factor,
                legacy_raw: false,
                ..
            }
        }) if input == PathBuf::from("/tmp/knowledge-export.jsonl")
            && (decay_factor - 0.8).abs() < f64::EPSILON
    ));
}

#[test]
fn cli_validates_knowledge_import_decay_factor() {
    let cli = Cli::try_parse_from([
        "roko",
        "knowledge",
        "import",
        "/tmp/knowledge-export.jsonl",
        "--decay-factor",
        "0.65",
        "--legacy-raw",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Knowledge {
            cmd: KnowledgeCmd::Import {
                decay_factor,
                legacy_raw: true,
                ..
            }
        }) if (decay_factor - 0.65).abs() < f64::EPSILON
    ));

    let error = Cli::try_parse_from([
        "roko",
        "knowledge",
        "import",
        "/tmp/knowledge-export.jsonl",
        "--decay-factor",
        "1.01",
    ])
    .unwrap_err();
    assert!(error.to_string().contains("between 0.0 and 1.0"));
}

#[test]
fn cli_parses_knowledge_restore_subcommand() {
    let cli = Cli::try_parse_from([
        "roko",
        "knowledge",
        "restore",
        "/tmp/neuro-backup",
        "--force",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Knowledge {
            cmd: KnowledgeCmd::Restore { source, force, .. }
        }) if source == PathBuf::from("/tmp/neuro-backup") && force
    ));
}

#[test]
fn cli_parses_config_experiments_subcommand() {
    let cli = Cli::try_parse_from([
        "roko",
        "config",
        "experiments",
        "model",
        "create",
        "--id",
        "glm-vs-kimi-impl",
        "--role",
        "implementer",
        "--variant",
        "glm-5-1:glm-5.1:zai",
        "--variant",
        "kimi-k2-5:kimi-k2.5:moonshot",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::Experiments { .. }
        })
    ));
}

#[test]
fn cli_parses_config_providers_list_subcommand() {
    let cli = Cli::try_parse_from(["roko", "config", "providers", "list"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::Providers {
                cmd: ConfigProviderCmd::List { .. }
            }
        })
    ));
}

#[test]
fn cli_parses_config_providers_health_subcommand() {
    let cli = Cli::try_parse_from(["roko", "config", "providers", "health"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::Providers {
                cmd: ConfigProviderCmd::Health { .. }
            }
        })
    ));
}

#[test]
fn cli_parses_config_providers_reset_health_subcommand() {
    let cli =
        Cli::try_parse_from(["roko", "config", "providers", "reset-health", "claude_cli"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::Providers {
                cmd: ConfigProviderCmd::ResetHealth { provider: Some(ref p), .. }
            }
        }) if p == "claude_cli"
    ));
}

/// gap-d90a93: `roko config providers reset-health` clears a quarantine that
/// `provider-health.json` persists. With no provider named it clears every
/// held one; the provider routes on the next load, and its lifetime counts
/// stay. A provider the file does not track is an error.
#[test]
fn reset_health_clears_a_persisted_quarantine() {
    use roko_learn::provider_health::{ErrorClass, ProviderHealthRegistry};

    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("provider-health.json");
    let registry = ProviderHealthRegistry::load_or_new(&path);
    registry.record_failure("claude-cli", ErrorClass::AuthFailure);
    registry.record_failure("zai", ErrorClass::Timeout);
    registry.save(&path).expect("save the health file");
    drop(registry);

    let cleared = reset_provider_health(&path, None).expect("reset every held provider");
    assert_eq!(cleared.len(), 1, "{cleared:?}");
    assert!(cleared[0].starts_with("claude_cli: was"), "{cleared:?}");
    assert!(cleared[0].contains("OPEN"), "{cleared:?}");
    assert!(cleared[0].contains("AuthFailure"), "{cleared:?}");
    let reloaded = ProviderHealthRegistry::load_or_new(&path);
    assert!(reloaded.is_available("claude_cli"));
    assert_eq!(reloaded.get("claude_cli").total_failures, 1);
    assert_eq!(reloaded.get("zai").consecutive_failures, 1);
    drop(reloaded);

    let none_held = reset_provider_health(&path, None).expect("nothing is held");
    assert!(none_held.is_empty(), "{none_held:?}");
    let error = reset_provider_health(&path, Some("cerebras")).expect_err("an untracked provider");
    assert!(error.to_string().contains("cerebras"), "{error}");
}

#[test]
fn cli_parses_config_providers_test_subcommand() {
    let cli = Cli::try_parse_from(["roko", "config", "providers", "test", "zai"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::Providers {
                cmd: ConfigProviderCmd::Test { provider: Some(ref p), all: false, .. }
            }
        }) if p == "zai"
    ));
}

#[test]
fn cli_parses_config_providers_test_all() {
    let cli = Cli::try_parse_from(["roko", "config", "providers", "test", "--all"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::Providers {
                cmd: ConfigProviderCmd::Test {
                    provider: None,
                    all: true,
                    ..
                }
            }
        })
    ));
}

#[test]
fn cli_parses_config_providers_available_subcommand() {
    let cli = Cli::try_parse_from(["roko", "config", "providers", "available"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::Providers {
                cmd: ConfigProviderCmd::Available
            }
        })
    ));
}

#[test]
fn cli_parses_config_models_list_subcommand() {
    let cli = Cli::try_parse_from(["roko", "config", "models", "list"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::Models {
                cmd: ConfigModelCmd::List { .. }
            }
        })
    ));
}

#[test]
fn cli_parses_config_models_route_subcommand() {
    let cli = Cli::try_parse_from([
        "roko",
        "config",
        "models",
        "route",
        "glm-5-1",
        "--explain",
        "--complexity",
        "integrative",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Config {
            cmd: ConfigCmd::Models { cmd: ConfigModelCmd::Route { model, explain: true, complexity: Some(complexity), .. } }
        }) if model == "glm-5-1" && complexity == "integrative"
    ));
}

#[test]
fn cli_agent_chat_defaults_to_canonical_serve_url() {
    let cli = Cli::try_parse_from(["roko", "agent", "chat"]).unwrap();
    assert!(matches!(cli.command, Some(Command::Agent { .. })));
}

#[test]
fn cli_daemon_start_defaults_to_canonical_port() {
    let cli = Cli::try_parse_from(["roko", "daemon", "start"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Daemon {
            cmd: DaemonCmd::Start { port, .. }
        }) if port == roko_cli::DEFAULT_SERVE_PORT
    ));
}

#[test]
fn cli_daemon_restart_defaults_to_canonical_port() {
    let cli = Cli::try_parse_from(["roko", "daemon", "restart"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Daemon {
            cmd: DaemonCmd::Restart { port }
        }) if port == roko_cli::DEFAULT_SERVE_PORT
    ));
}

#[test]
fn cli_new_requires_type_and_name() {
    // `roko new` is a subcommand requiring <TYPE> <NAME>.
    assert!(Cli::try_parse_from(["roko", "new"]).is_err());
    assert!(Cli::try_parse_from(["roko", "new", "gate", "MyGate"]).is_ok());
}

#[test]
fn cli_explain_requires_topic() {
    // `roko explain` is a subcommand requiring <TOPIC>.
    assert!(Cli::try_parse_from(["roko", "explain"]).is_err());
    assert!(Cli::try_parse_from(["roko", "explain", "gates"]).is_ok());
}

#[test]
fn select_provider_test_model_prefers_default_model() {
    let mut config = RokoConfig::default();
    config.agent.default_model = "glm-5-1".to_string();
    config.models.insert(
        "glm-5-1".to_string(),
        ModelProfile {
            provider: "zai".to_string(),
            slug: "glm-5.1".to_string(),
            context_window: 200_000,
            max_output: Some(131_072),
            supports_tools: true,
            supports_thinking: true,
            supports_vision: false,
            supports_web_search: false,
            supports_mcp_tools: false,
            supports_partial: false,
            provider_routing: None,
            tool_format: "openai_json".to_string(),
            cost_input_per_m: Some(1.40),
            cost_output_per_m: Some(4.40),
            cost_cache_read_per_m: None,
            cost_cache_write_per_m: None,
            max_tools: None,
            tokenizer_ratio: None,
            ..Default::default()
        },
    );
    config.models.insert(
        "glm-5-1-alt".to_string(),
        ModelProfile {
            provider: "zai".to_string(),
            slug: "glm-5.1-air".to_string(),
            context_window: 128_000,
            max_output: Some(8_192),
            supports_tools: true,
            supports_thinking: false,
            supports_vision: false,
            supports_web_search: false,
            supports_mcp_tools: false,
            supports_partial: false,
            provider_routing: None,
            tool_format: "openai_json".to_string(),
            cost_input_per_m: Some(1.0),
            cost_output_per_m: Some(2.0),
            cost_cache_read_per_m: None,
            cost_cache_write_per_m: None,
            max_tools: None,
            tokenizer_ratio: None,
            ..Default::default()
        },
    );

    let selected = select_provider_test_model(&config, "zai").expect("selected model");
    assert_eq!(selected.0, "glm-5-1");
    assert_eq!(selected.1.slug, "glm-5.1");
}

#[test]
fn format_provider_rows_renders_headers_and_rows() {
    let output = format_provider_rows(&[ProviderListRow {
        provider: "anthropic".to_string(),
        kind: "claude_cli".to_string(),
        base_url: "(cli: claude)".to_string(),
        status: "ok (cli found)".to_string(),
    }]);

    assert!(output.contains("Provider"));
    assert!(output.contains("Base URL"));
    assert!(output.contains("anthropic"));
    assert!(output.contains("ok (cli found)"));
}

#[test]
fn format_model_rows_renders_headers_and_rows() {
    let output = format_model_rows(&[ModelListRow {
        model: "glm-5-1".to_string(),
        provider: "zai".to_string(),
        slug: "glm-5.1".to_string(),
        context: "200K".to_string(),
        tools: "✓".to_string(),
        thinking: "✓".to_string(),
        vision: "✗".to_string(),
        cost: "$1.40/$4.40".to_string(),
        key_status: "ok".to_string(),
        aliases: String::new(),
    }]);

    assert!(output.contains("Model"));
    assert!(output.contains("Cost (in/out)"));
    assert!(output.contains("glm-5-1"));
    assert!(output.contains("$1.40/$4.40"));
}

#[test]
fn build_model_list_row_formats_capabilities_and_costs() {
    let row = build_model_list_row(
        "kimi-k2-5",
        &ModelProfile {
            provider: "moonshot".to_string(),
            slug: "kimi-k2.5".to_string(),
            context_window: 256_000,
            max_output: Some(128_000),
            supports_tools: true,
            supports_thinking: true,
            supports_vision: true,
            supports_web_search: false,
            supports_mcp_tools: false,
            supports_partial: false,
            provider_routing: None,
            tool_format: "openai_json".to_string(),
            cost_input_per_m: Some(0.60),
            cost_output_per_m: Some(3.00),
            cost_cache_read_per_m: None,
            cost_cache_write_per_m: None,
            max_tools: None,
            tokenizer_ratio: None,
            ..Default::default()
        },
    );

    assert_eq!(row.model, "kimi-k2-5");
    assert_eq!(row.provider, "moonshot");
    assert_eq!(row.slug, "kimi-k2.5");
    assert_eq!(row.context, "256K");
    assert_eq!(row.tools, "✓");
    assert_eq!(row.thinking, "✓");
    assert_eq!(row.vision, "✓");
    assert_eq!(row.cost, "$0.60/$3.00");
}

#[test]
fn build_provider_health_row_formats_state_latency_and_error_rate() {
    let health = ProviderHealth {
        provider_id: "zai".to_string(),
        state: CircuitState::Open,
        consecutive_failures: 3,
        total_requests: 20,
        total_failures: 3,
        last_failure_at: Some(90_000),
        last_success_at: Some(95_000),
        cooldown_until: Some(108_000),
        failure_window: std::collections::VecDeque::new(),
        recent_outcomes: std::collections::VecDeque::new(),
        wasted_cost_usd: 0.0,
    };
    let latency = ProviderLatencySummary {
        recent_latencies: vec![800.0, 1_200.0, 600.0],
        weighted_latency_ms: 0.0,
        observations: 0,
    };

    let row = build_provider_health_row(
        "zai",
        Some(&health),
        Some(&latency),
        100_000,
        Some(95_000),
        Some(99_000),
    );

    assert_eq!(row.provider, "zai");
    assert_eq!(row.state, "OPEN");
    assert_eq!(row.fails, "3/3");
    assert_eq!(row.cooldown, "8s left");
    assert_eq!(row.latency_p50, "0.8s");
    assert_eq!(row.error_rate, "15.0%");
    assert_eq!(row.last_check, "1s ago");
}

#[test]
fn format_provider_health_rows_renders_headers_and_rows() {
    let output = format_provider_health_rows(&[ProviderHealthRow {
        provider: "openrouter".to_string(),
        state: "CLOSED".to_string(),
        fails: "0/3".to_string(),
        cooldown: "—".to_string(),
        latency_p50: "0.8s".to_string(),
        error_rate: "0.0%".to_string(),
        last_check: "5m ago".to_string(),
    }]);

    assert!(output.contains("Provider"));
    assert!(output.contains("Latency p50"));
    assert!(output.contains("Error Rate"));
    assert!(output.contains("openrouter"));
    assert!(output.contains("0.8s"));
}

#[test]
fn cli_parses_dashboard_subcommand() {
    let cli =
        Cli::try_parse_from(["roko", "dashboard", "--page", "plan-view", "--list-pages"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Dashboard {
            page: Some(_),
            list_pages: true,
            text: false,
            ..
        })
    ));
}

#[test]
fn cli_parses_dashboard_text_flag() {
    let cli = Cli::try_parse_from(["roko", "dashboard", "--text"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Dashboard { text: true, .. })
    ));
}

#[test]
fn cli_parses_dashboard_snapshot_flag() {
    let cli = Cli::try_parse_from(["roko", "dashboard", "--snapshot", "/tmp/snap"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Dashboard {
            snapshot: Some(_),
            ..
        })
    ));
}

#[test]
fn parse_dashboard_page_accepts_known_slugs() {
    assert_eq!(parse_dashboard_page("health"), Some(PageId::Health));
    assert_eq!(
        parse_dashboard_page("agent status"),
        Some(PageId::AgentStatus)
    );
    assert_eq!(
        parse_dashboard_page("agent activity"),
        Some(PageId::AgentStatus)
    );
    assert_eq!(parse_dashboard_page("plan_view"), Some(PageId::PlanView));
    assert_eq!(parse_dashboard_page("learning"), Some(PageId::Learning));
    assert_eq!(
        parse_dashboard_page("provider health"),
        Some(PageId::ProviderHealth)
    );
    assert_eq!(
        parse_dashboard_page("model comparison"),
        Some(PageId::ModelComparison)
    );
}

#[test]
fn parse_dashboard_page_rejects_unknown_slugs() {
    assert_eq!(parse_dashboard_page("unknown"), None);
}

async fn seed_dashboard_snapshot(workdir: &Path) {
    let memory_dir = workdir.join(".roko").join("memory");
    fs::create_dir_all(&memory_dir).await.unwrap();
    let learn_dir = workdir.join(".roko").join("learn");
    fs::create_dir_all(&learn_dir).await.unwrap();

    let mut ep1 = Episode::new("agent-a", "task-a");
    ep1.success = true;
    ep1.usage.cost_usd = 1.25;
    ep1.usage.wall_ms = 125;
    ep1.usage.input_tokens = 100;
    ep1.usage.cache_read_tokens = 25;

    let mut ep2 = Episode::new("agent-b", "task-b");
    ep2.success = false;
    ep2.usage.cost_usd = 2.75;
    ep2.usage.wall_ms = 225;
    ep2.usage.input_tokens = 200;
    ep2.usage.cache_read_tokens = 50;

    let episodes_path = workdir.join(".roko").join("episodes.jsonl");
    let episodes = [
        serde_json::to_string(&ep1).unwrap(),
        serde_json::to_string(&ep2).unwrap(),
    ]
    .join("\n")
        + "\n";
    fs::write(&episodes_path, episodes).await.unwrap();

    // The headline numbers come from the runs' attempt ledgers (backlog 2126).
    {
        use roko_learn::telemetry::{
            AttemptIdentity, AttemptKey, AttemptOutcome, AttemptVerdictRecord, TelemetryWriter,
            TelemetryWriterConfig,
        };

        let run_dir = workdir.join(".roko").join("runs").join("run-1");
        let writer = TelemetryWriter::spawn(&run_dir, TelemetryWriterConfig::default()).unwrap();
        for (plan, task, model, outcome, cost_usd, tokens_in) in [
            (
                "plan-a",
                "task-a",
                "claude-haiku",
                AttemptOutcome::Passed,
                1.0,
                100,
            ),
            (
                "plan-b",
                "task-b",
                "claude-sonnet",
                AttemptOutcome::GateFailed,
                3.0,
                200,
            ),
        ] {
            let key = AttemptKey::new("run-1", plan, task, 1);
            let identity = AttemptIdentity::new(&key);
            let mut verdict = AttemptVerdictRecord::settle(identity, outcome, true);
            verdict.executed.model_dispatched = Some(model.to_string());
            verdict.cost.billed_usd = Some(cost_usd);
            verdict.usage.tokens_in = Some(tokens_in);
            assert!(writer.submit(verdict));
        }
        assert_eq!(writer.close().written, 2);
    }

    let cfactor_path = learn_dir.join("c-factor.jsonl");
    let mut cf1 = CFactor::default();
    cf1.overall = 0.48;
    cf1.computed_at = chrono::Utc::now() - chrono::Duration::days(6);

    let mut cf2 = CFactor::default();
    cf2.overall = 0.53;
    cf2.computed_at = chrono::Utc::now() - chrono::Duration::days(3);

    let mut cf3 = CFactor::default();
    cf3.overall = 0.67;
    cf3.components = roko_learn::cfactor::CFactorComponents {
        gate_pass_rate: 0.82,
        cost_efficiency: 0.76,
        speed: 0.71,
        information_flow_rate: 0.89,
        first_try_rate: 0.64,
        knowledge_growth: 0.18,
        knowledge_integration_rate: 0.57,
        hdc_diversity: 0.73,
        convergence_velocity: 0.66,
        turn_taking_equality: 0.74,
        social_perceptiveness: 0.68,
    };
    cf3.computed_at = chrono::Utc::now();

    let cfactor_history = [
        serde_json::to_string(&cf1).unwrap(),
        serde_json::to_string(&cf2).unwrap(),
        serde_json::to_string(&cf3).unwrap(),
    ]
    .join("\n")
        + "\n";
    fs::write(&cfactor_path, cfactor_history).await.unwrap();

    let provider_health_path = learn_dir.join("provider-health.json");
    let provider_health = serde_json::json!({
        "providers": {
            "anthropic": {
                "provider_id": "anthropic",
                "state": "Closed",
                "consecutive_failures": 0,
                "total_requests": 12,
                "total_failures": 1,
                "last_failure_at": null,
                "cooldown_until": null,
                "failure_window": []
            },
            "zai": {
                "provider_id": "zai",
                "state": "HalfOpen",
                "consecutive_failures": 3,
                "total_requests": 8,
                "total_failures": 2,
                "last_failure_at": 1710000000000i64,
                "cooldown_until": 1710000005000i64,
                "failure_window": []
            }
        }
    });
    fs::write(
        &provider_health_path,
        serde_json::to_string_pretty(&provider_health).unwrap(),
    )
    .await
    .unwrap();

    let latency_stats_path = learn_dir.join("latency-stats.json");
    let latency_stats = serde_json::json!({
        "entries": [
            {
                "provider": "anthropic",
                "stats": {
                    "model_slug": "claude-opus-4-6",
                    "provider_id": "anthropic",
                    "ttft_ema_ms": 0.0,
                    "total_latency_ema_ms": 0.0,
                    "tokens_per_second_ema": 0.0,
                    "observations": 3,
                    "recent_latencies": [800.0, 1200.0, 600.0]
                }
            }
        ]
    });
    fs::write(
        &latency_stats_path,
        serde_json::to_string_pretty(&latency_stats).unwrap(),
    )
    .await
    .unwrap();

    let cascade_router_path = learn_dir.join("cascade-router.json");
    let cascade_router = serde_json::json!({
        "model_slugs": ["kimi-k2.5", "glm-5.1", "claude-sonnet-4-6", "claude-opus-4-6"],
        "confidence_stats": {
            "kimi-k2.5": { "trials": 145, "successes": 113 },
            "glm-5.1": { "trials": 203, "successes": 166 },
            "claude-sonnet-4-6": { "trials": 312, "successes": 250 },
            "claude-opus-4-6": { "trials": 47, "successes": 44 }
        }
    });
    fs::write(
        &cascade_router_path,
        serde_json::to_string_pretty(&cascade_router).unwrap(),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn dashboard_output_renders_snapshot_for_health_and_falls_back_for_other_pages() {
    let dir = tempdir().unwrap();
    seed_dashboard_snapshot(dir.path()).await;

    let cli = Cli::try_parse_from(["roko", "--quiet"]).unwrap();
    let health = dashboard_output(
        &cli,
        Some(dir.path().to_path_buf()),
        Some("health".to_string()),
        false,
    )
    .await
    .unwrap();
    assert!(health.contains("Health (health)"));
    assert!(health.contains("episodes: 2"));
    assert!(health.contains("success rate: 50.0%"));
    assert!(health.contains("avg cost / episode: $2.0000"));
    assert!(health.contains("cache hit rate: 25.0%"));
    assert!(health.contains("current c-factor: 0.67 ↑"));
    assert!(health.contains("gate pass rate: 82.0%"));
    assert!(health.contains("information flow rate: 89.0%"));
    assert!(health.contains("knowledge growth: 18.0%"));

    let trends = dashboard_output(
        &cli,
        Some(dir.path().to_path_buf()),
        Some("trends".to_string()),
        false,
    )
    .await
    .unwrap();
    assert!(trends.contains("Trends (trends)"));
    assert!(trends.contains("first-attempt pass rate: 50.0%"));
    assert!(trends.contains("avg iterations per plan: 1.00"));
    assert!(trends.contains("avg cost per plan: $2.0000"));
    assert!(trends.contains("haiku share: 50.0%"));

    let provider_health = dashboard_output(
        &cli,
        Some(dir.path().to_path_buf()),
        Some("provider-health".to_string()),
        false,
    )
    .await
    .unwrap();
    assert!(provider_health.contains("Provider Health (provider-health)"));
    assert!(provider_health.contains("anthropic"));
    assert!(provider_health.contains("● CLOSED"));
    assert!(provider_health.contains("p50: 0.8s"));
    assert!(provider_health.contains("summary: 20 requests, 3 failures"));

    let model_comparison = dashboard_output(
        &cli,
        Some(dir.path().to_path_buf()),
        Some("model-comparison".to_string()),
        false,
    )
    .await
    .unwrap();
    assert!(model_comparison.contains("Model Comparison (model-comparison)"));
    assert!(model_comparison.contains("Pareto frontier:"));
    assert!(model_comparison.contains("claude-sonnet-4-6 dominated by glm-5.1"));

    let fallback = dashboard_output(
        &cli,
        Some(dir.path().to_path_buf()),
        Some("plan-view".to_string()),
        false,
    )
    .await
    .unwrap();
    assert!(fallback.contains("Plan View (plan-view)"));
    // render_plan_view_page now returns Some("source: missing") instead
    // of None, so the scaffold widget list fallback is no longer reached.
    assert!(fallback.contains("source: missing") || fallback.contains("widgets (2):"));
}

#[test]
fn bootstrap_observability_dirs_creates_expected_paths() {
    let tmp = tempfile::tempdir().unwrap();
    // The guard requires roko.toml or .roko/ to exist before creating dirs.
    std::fs::write(tmp.path().join("roko.toml"), b"").unwrap();
    bootstrap_observability_dirs(tmp.path()).unwrap();
    let roko = tmp.path().join(".roko");
    assert!(roko.join("traces").is_dir());
    // Tool calls keep no metrics file (backlog 2123).
    assert!(!roko.join("metrics").exists());
    assert!(roko.join("runtime").is_dir());
    assert!(roko.join("runs").is_dir());
}

#[test]
fn bootstrap_observability_dirs_skips_without_intent() {
    let tmp = tempfile::tempdir().unwrap();
    // No roko.toml or .roko/ — guard should skip creation.
    bootstrap_observability_dirs(tmp.path()).unwrap();
    assert!(!tmp.path().join(".roko").exists());
}

#[test]
fn backup_neuro_store_writes_canonical_secret_safe_snapshot() {
    let workdir = tempdir().unwrap();
    let backup_dir = tempdir().unwrap();
    let neuro_dir = workdir.path().join(".roko").join("neuro");
    std::fs::create_dir_all(&neuro_dir).unwrap();
    std::fs::write(neuro_dir.join(NEURO_KNOWLEDGE_FILE), b"{\"id\":\"k1\"}\n").unwrap();
    std::fs::write(
        neuro_dir.join(NEURO_CONFIRMATIONS_FILE),
        b"{\"id\":\"c1\"}\n",
    )
    .unwrap();

    let report = backup_neuro_store(workdir.path(), backup_dir.path(), false, None).unwrap();

    let backup = std::fs::read_to_string(&report.snapshot.knowledge).unwrap();
    let mut lines = backup.lines();
    let header: serde_json::Value =
        serde_json::from_str(lines.next().expect("backup header")).unwrap();
    assert_eq!(header["version"], 2);
    assert_eq!(header["entry_count"], 1);
    assert!(
        header["merkle_root"]
            .as_str()
            .is_some_and(|root| !root.is_empty())
    );
    let entry: serde_json::Value =
        serde_json::from_str(lines.next().expect("knowledge entry")).unwrap();
    assert_eq!(entry["id"], "k1");
    assert!(lines.next().is_none());

    let restored = KnowledgeStore::new(backup_dir.path().join("roundtrip.jsonl"));
    let import = restored
        .import(&report.snapshot.knowledge, &ImportOptions::default())
        .unwrap();
    assert_eq!(import.imported, 1);
    assert_eq!(restored.read_all().unwrap()[0].id, "k1");
    assert_eq!(
        std::fs::read(report.snapshot.confirmations).unwrap(),
        b"{\"id\":\"c1\"}\n"
    );
    assert!(report.confirmations_present);
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&report.manifest).expect("read generated manifest"))
            .expect("parse generated manifest");
    assert_eq!(manifest["version"], 2);
    assert_eq!(manifest["knowledge_format_version"], 2);
    assert_eq!(manifest["entry_count"], 1);
    assert_eq!(manifest["confirmations_present"], true);
    assert_eq!(manifest["confirmations"]["bytes"], 12);
    assert!(
        manifest["confirmations"]["sha256"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
}

#[test]
fn restore_neuro_store_requires_force_for_existing_target_and_removes_stale_optional_file() {
    let workdir = tempdir().unwrap();
    let backup_dir = tempdir().unwrap();
    let neuro_dir = workdir.path().join(".roko").join("neuro");
    std::fs::create_dir_all(&neuro_dir).unwrap();
    std::fs::write(
        neuro_dir.join(NEURO_KNOWLEDGE_FILE),
        b"{\"id\":\"old\",\"content\":\"old data\",\"confidence\":0.5}\n",
    )
    .unwrap();
    std::fs::write(neuro_dir.join(NEURO_CONFIRMATIONS_FILE), b"stale\n").unwrap();
    std::fs::write(
        backup_dir.path().join(NEURO_KNOWLEDGE_FILE),
        b"{\"id\":\"new\",\"content\":\"new data\",\"confidence\":0.9}\n",
    )
    .unwrap();

    let err = restore_neuro_store(
        workdir.path(),
        backup_dir.path(),
        false,
        1,
        0.8,
        None,
        None,
        true,
    )
    .unwrap_err();
    assert!(err.to_string().contains("Re-run with --force"));

    let report = restore_neuro_store(
        workdir.path(),
        backup_dir.path(),
        true,
        1,
        0.8,
        None,
        None,
        true,
    )
    .unwrap();
    let restored = std::fs::read_to_string(&report.live.knowledge).unwrap();
    assert!(
        restored.contains("\"new\""),
        "restored store should contain the new entry"
    );
    // The backup has no confirmations file, so the report should note it as absent.
    assert!(!report.confirmations_present);
    assert!(
        !report.live.confirmations.exists(),
        "restore must remove confirmations that are absent from the backup"
    );
}

#[test]
fn backup_preflight_and_alias_checks_preserve_existing_state() {
    let workdir = tempdir().unwrap();
    let backup_parent = tempdir().unwrap();
    let neuro_dir = workdir.path().join(".roko").join("neuro");
    std::fs::create_dir_all(&neuro_dir).unwrap();
    let live_knowledge = neuro_dir.join(NEURO_KNOWLEDGE_FILE);
    std::fs::write(&live_knowledge, b"{\"id\":\"live\"}\n").unwrap();

    let destination = backup_parent.path().join("snapshot");
    std::fs::create_dir_all(&destination).unwrap();
    let marker = destination.join("keep.txt");
    std::fs::write(&marker, b"unchanged").unwrap();
    let error = backup_neuro_store(workdir.path(), &destination, false, None)
        .expect_err("populated destination must require force");
    assert!(error.to_string().contains("--force"));
    assert_eq!(std::fs::read(&marker).unwrap(), b"unchanged");
    assert!(!destination.join(NEURO_KNOWLEDGE_FILE).exists());

    let forced = backup_neuro_store(workdir.path(), &destination, true, None)
        .expect("forced staged replacement");
    assert!(forced.snapshot.knowledge.exists());
    assert!(forced.manifest.exists());
    assert!(
        !marker.exists(),
        "forced replacement must not retain stale files"
    );

    let before = std::fs::read(&live_knowledge).unwrap();
    let error = backup_neuro_store(workdir.path(), &neuro_dir, true, None)
        .expect_err("backup must reject the live neuro directory");
    assert!(error.to_string().contains("live neuro store"));
    assert_eq!(std::fs::read(&live_knowledge).unwrap(), before);

    let absent_workdir = tempdir().unwrap();
    let nonexistent_ancestor = absent_workdir.path().join(".roko");
    let error = backup_neuro_store(absent_workdir.path(), &nonexistent_ancestor, true, None)
        .expect_err("nonexistent ancestor of live store must still be rejected");
    assert!(error.to_string().contains("live neuro store"));
    assert!(
        !nonexistent_ancestor.exists(),
        "alias preflight must not create a missing destination"
    );
}

#[test]
fn restore_verifies_confirmation_digest_before_creating_live_state() {
    let source = tempdir().unwrap();
    let destination = tempdir().unwrap();
    let backup = tempdir().unwrap();
    let source_neuro = source.path().join(".roko").join("neuro");
    std::fs::create_dir_all(&source_neuro).unwrap();
    std::fs::write(
        source_neuro.join(NEURO_KNOWLEDGE_FILE),
        b"{\"id\":\"source\",\"content\":\"source knowledge\"}\n",
    )
    .unwrap();
    std::fs::write(
        source_neuro.join(NEURO_CONFIRMATIONS_FILE),
        b"original confirmations\n",
    )
    .unwrap();
    backup_neuro_store(source.path(), backup.path(), false, None).unwrap();
    std::fs::write(
        backup.path().join(NEURO_CONFIRMATIONS_FILE),
        b"tampered confirmations\n",
    )
    .unwrap();

    let error = restore_neuro_store(
        destination.path(),
        backup.path(),
        false,
        1,
        0.8,
        None,
        None,
        false,
    )
    .expect_err("confirmation tampering must fail");
    assert!(error.to_string().contains("integrity verification failed"));
    assert!(!neuro_live_files(destination.path()).knowledge.exists());
}

#[test]
fn restore_rejects_manifest_count_mismatch_without_live_writes() {
    let source = tempdir().unwrap();
    let destination = tempdir().unwrap();
    let backup = tempdir().unwrap();
    let source_neuro = source.path().join(".roko").join("neuro");
    std::fs::create_dir_all(&source_neuro).unwrap();
    std::fs::write(
        source_neuro.join(NEURO_KNOWLEDGE_FILE),
        b"{\"id\":\"source\",\"content\":\"source knowledge\"}\n",
    )
    .unwrap();
    let report = backup_neuro_store(source.path(), backup.path(), false, None).unwrap();
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&report.manifest).unwrap()).unwrap();
    manifest["entry_count"] = serde_json::json!(2);
    std::fs::write(
        &report.manifest,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let error = restore_neuro_store(
        destination.path(),
        backup.path(),
        false,
        1,
        0.8,
        None,
        None,
        false,
    )
    .expect_err("manifest count mismatch must fail");
    assert!(error.to_string().contains("entry_count mismatch"));
    assert!(!neuro_live_files(destination.path()).knowledge.exists());
}

#[test]
fn restore_requires_explicit_legacy_for_missing_or_v1_manifest() {
    for legacy_case in ["missing", "v1"] {
        let source = tempdir().unwrap();
        let backup = tempdir().unwrap();
        let destination = tempdir().unwrap();
        let source_neuro = source.path().join(".roko").join("neuro");
        std::fs::create_dir_all(&source_neuro).unwrap();
        std::fs::write(
            source_neuro.join(NEURO_KNOWLEDGE_FILE),
            format!("{{\"id\":\"{legacy_case}\",\"content\":\"legacy case\"}}\n"),
        )
        .unwrap();
        let report = backup_neuro_store(source.path(), backup.path(), false, None).unwrap();
        if legacy_case == "missing" {
            std::fs::remove_file(&report.manifest).unwrap();
        } else {
            let mut manifest: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&report.manifest).unwrap()).unwrap();
            manifest["version"] = serde_json::json!(1);
            std::fs::write(
                &report.manifest,
                serde_json::to_vec_pretty(&manifest).unwrap(),
            )
            .unwrap();
        }

        let error = restore_neuro_store(
            destination.path(),
            backup.path(),
            false,
            1,
            0.8,
            None,
            None,
            false,
        )
        .expect_err("legacy manifest must require opt-in");
        assert!(error.to_string().contains("legacy") || error.to_string().contains("manifest"));
        assert!(!neuro_live_files(destination.path()).knowledge.exists());

        let restored = restore_neuro_store(
            destination.path(),
            backup.path(),
            false,
            1,
            0.8,
            None,
            None,
            true,
        )
        .expect("explicit legacy restore");
        assert!(restored.legacy_input);
        assert_eq!(restored.entries_restored, 1);
    }
}

#[test]
fn restore_rejects_corrupt_live_store_without_partial_knowledge_or_confirmation_changes() {
    let source = tempdir().unwrap();
    let backup = tempdir().unwrap();
    let destination = tempdir().unwrap();
    let source_neuro = source.path().join(".roko").join("neuro");
    std::fs::create_dir_all(&source_neuro).unwrap();
    std::fs::write(
        source_neuro.join(NEURO_KNOWLEDGE_FILE),
        b"{\"id\":\"new\",\"content\":\"new knowledge\"}\n",
    )
    .unwrap();
    backup_neuro_store(source.path(), backup.path(), false, None).unwrap();

    let live = neuro_live_files(destination.path());
    std::fs::create_dir_all(live.knowledge.parent().unwrap()).unwrap();
    let knowledge_before = b"{\"id\":\"old\",\"content\":\"valid\"}\nnot-json\n";
    let confirmations_before = b"old confirmations\n";
    std::fs::write(&live.knowledge, knowledge_before).unwrap();
    std::fs::write(&live.confirmations, confirmations_before).unwrap();

    let error = restore_neuro_store(
        destination.path(),
        backup.path(),
        true,
        1,
        0.8,
        None,
        None,
        false,
    )
    .expect_err("corrupt live store must fail closed");
    assert!(format!("{error:#}").contains("decode knowledge line 2"));
    assert_eq!(std::fs::read(&live.knowledge).unwrap(), knowledge_before);
    assert_eq!(
        std::fs::read(&live.confirmations).unwrap(),
        confirmations_before
    );
}

#[test]
fn subcommand_workdir_reads_the_deepest_workdir_flag() {
    let workdir = |args: &[&str]| {
        let matches = Cli::command()
            .try_get_matches_from(args)
            .expect("valid invocation");
        subcommand_workdir(&matches)
    };
    assert_eq!(
        workdir(&["roko", "serve", "--workdir", "/ws/serve"]),
        Some(PathBuf::from("/ws/serve"))
    );
    assert_eq!(
        workdir(&["roko", "plan", "run", "plans", "--workdir", "/ws/plan"]),
        Some(PathBuf::from("/ws/plan"))
    );
    assert_eq!(workdir(&["roko", "--repo", "/ws/repo", "status"]), None);
    assert_eq!(workdir(&["roko", "init"]), None);
}

#[test]
fn crash_report_dir_uses_subcommand_workdir() {
    let matches = Cli::command()
        .try_get_matches_from(["roko", "plan", "run", "plans", "--workdir", "/ws/plan"])
        .expect("valid invocation");
    let workdir = subcommand_workdir(&matches);
    assert_eq!(
        crash_report_dir(workdir.as_deref()),
        PathBuf::from("/ws/plan").join(".roko")
    );
}

#[test]
fn redacting_format_scrubs_api_keys() {
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::layer::SubscriberExt;

    // Capture output into a shared buffer.
    #[derive(Clone)]
    struct BufWriter(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for BufWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for BufWriter {
        type Writer = BufWriter;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    let buffer = Arc::new(Mutex::new(Vec::new()));
    let writer = BufWriter(Arc::clone(&buffer));

    let scrubber = roko_fs::observability::RunScrubber::build(&[]);
    let fmt_layer = tracing_subscriber::fmt::layer()
        .event_format(RedactingFormat::new(
            tracing_subscriber::fmt::format(),
            scrubber,
        ))
        .with_writer(writer)
        .with_ansi(false);

    let subscriber = tracing_subscriber::registry().with(fmt_layer);

    // Use `with_default` so the subscriber is scoped to this test — does
    // not conflict with the global subscriber from other tests.
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(
            "connecting with key sk-ant-api03-AAABBBCCCDDDEEEFFFGGGHHHIIIJJJ and token ghp_ABCDEFGHIJKLMNOPqrstuvwxyz1234567890"
        );
        tracing::warn!("Bearer eyJhbGciOiJIUzI1NiJ9.payload.signature in header");
        tracing::info!("ANTHROPIC_API_KEY=sk-ant-secret-value-99999 leaked");
    });

    let output = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();

    // API keys must be scrubbed.
    assert!(
        !output.contains("sk-ant-api03-AAABBBCCC"),
        "Anthropic key should be scrubbed, got: {output}"
    );
    assert!(
        !output.contains("ghp_ABCDEFGHIJKLMNOP"),
        "GitHub PAT should be scrubbed, got: {output}"
    );
    assert!(
        !output.contains("eyJhbGciOiJIUzI1NiJ9"),
        "Bearer token should be scrubbed, got: {output}"
    );
    assert!(
        !output.contains("sk-ant-secret-value"),
        "env-var key value should be scrubbed, got: {output}"
    );

    // Redaction markers must be present.
    assert!(
        output.contains("[REDACTED"),
        "redaction markers should appear, got: {output}"
    );

    // Non-secret context text must survive.
    assert!(
        output.contains("connecting with key"),
        "context text should survive, got: {output}"
    );
}

#[test]
fn dotenv_entries_record_which_values_the_files_supplied() {
    let shell = |name: &str| {
        matches!(name, "SHELL_KEY" | "SAME_KEY").then(|| std::ffi::OsString::from("shell"))
    };
    let entries = |pairs: &[(&str, &str)]| {
        pairs
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect::<Vec<_>>()
    };
    let mut names = roko_core::child_env::DotenvNames::new();
    // ~/.roko/.env never overrides: a variable already set keeps its value.
    record_dotenv_entries(
        &mut names,
        &entries(&[("NEW_KEY", "file"), ("SHELL_KEY", "file")]),
        false,
        shell,
    );
    // ./.roko/.env overrides, but an identical value changes nothing.
    record_dotenv_entries(
        &mut names,
        &entries(&[("SAME_KEY", "shell"), ("OTHER_KEY", "file")]),
        true,
        shell,
    );
    for name in ["NEW_KEY", "SHELL_KEY", "SAME_KEY", "OTHER_KEY"] {
        assert!(names.is_listed(name), "{name} is listed");
    }
    assert!(names.supplied_value("NEW_KEY"));
    assert!(!names.supplied_value("SHELL_KEY"));
    assert!(!names.supplied_value("SAME_KEY"));
    assert!(names.supplied_value("OTHER_KEY"));
}

#[test]
fn log_scrubber_adds_env_redactions() {
    let scrubber = roko_fs::observability::RunScrubber::build(&[("MY_TOKEN", "super-secret-42")]);
    let output = scrubber.scrub("leaked super-secret-42 in logs");
    assert!(
        !output.contains("super-secret-42"),
        "env redaction should scrub literal value, got: {output}"
    );
    assert!(
        output.contains("[REDACTED:MY_TOKEN]"),
        "should use named redaction, got: {output}"
    );
}

#[test]
fn tracing_log_directive_prefers_roko_log() {
    // ROKO_LOG is authoritative; when both are set, ROKO_LOG wins.
    let directive = tracing_log_directive_from(Some("roko=debug".into()), Some("info".into()));
    assert_eq!(directive, "info");
}

#[test]
fn tracing_log_directive_falls_back_to_rust_log_and_default() {
    // Falls back to RUST_LOG when ROKO_LOG is absent.
    let directive = tracing_log_directive_from(Some("roko=trace".into()), None);
    assert_eq!(directive, "roko=trace");

    // Falls back to ROKO_LOG when present and RUST_LOG absent.
    let directive2 = tracing_log_directive_from(None, Some("roko=trace".into()));
    assert_eq!(directive2, "roko=trace");

    let default_directive = tracing_log_directive_from(None, None);
    assert_eq!(default_directive, "roko=info");
}

#[test]
fn cli_parses_feed_list() {
    let cli = Cli::try_parse_from(["roko", "feed", "list"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Feed {
            cmd: commands::feed::FeedCmd::List,
        })
    ));
}

#[test]
fn cli_parses_feed_status() {
    let cli = Cli::try_parse_from(["roko", "feed", "status", "file-watch-roko-dir"]).unwrap();
    match cli.command {
        Some(Command::Feed {
            cmd: commands::feed::FeedCmd::Status { id },
        }) => assert_eq!(id, "file-watch-roko-dir"),
        other => panic!("unexpected command variant: {other:?}"),
    }
}

#[test]
fn cli_parses_feed_lifecycle_and_recipe_run() {
    let feed = Cli::try_parse_from(["roko", "feed", "start", "provider-health-feed"]).unwrap();
    assert!(matches!(
        feed.command,
        Some(Command::Feed {
            cmd: commands::feed::FeedCmd::Start { .. }
        })
    ));

    let recipe = Cli::try_parse_from([
        "roko", "recipe", "run", "blend", "--input", "left=2", "--input", "right=6",
    ])
    .unwrap();
    match recipe.command {
        Some(Command::Recipe {
            cmd: commands::recipe::RecipeCmd::Run { id, inputs },
        }) => {
            assert_eq!(id, "blend");
            assert_eq!(inputs, vec!["left=2", "right=6"]);
        }
        other => panic!("unexpected command variant: {other:?}"),
    }
}

// ── E15-T7: roko-mcp-github auto-discovery tests ─────────────────────────

/// Helper: create a fake executable at `dir/roko-mcp-github` and return its
/// path. Uses only `std::fs` — no env var manipulation required.
fn create_fake_github_binary(dir: &std::path::Path) -> std::path::PathBuf {
    let fake = dir.join("roko-mcp-github");
    std::fs::write(&fake, b"#!/bin/sh\necho ok").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&fake).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake, perms).unwrap();
    }
    fake
}

/// `find_binary_in_target_dirs` must locate a binary placed in a
/// `target/debug/` ancestor of the given search root.
#[test]
fn mcp_github_discover_finds_binary_in_target_debug() {
    let tmp = tempdir().unwrap();
    let target_debug = tmp.path().join("target").join("debug");
    std::fs::create_dir_all(&target_debug).unwrap();
    let fake = create_fake_github_binary(&target_debug);

    let found = commands::mcp::find_binary_in_target_dirs(tmp.path(), "roko-mcp-github");
    assert!(found.is_some(), "should find binary in target/debug/");
    assert_eq!(found.unwrap(), fake);
}

/// `find_binary_in_target_dirs` must return `None` when no matching binary
/// exists anywhere in the ancestor tree.
#[test]
fn mcp_github_discover_returns_none_when_no_target_binary() {
    let tmp = tempdir().unwrap();
    // No target/ subdirectory — nothing to find.
    let found = commands::mcp::find_binary_in_target_dirs(tmp.path(), "roko-mcp-github");
    assert!(found.is_none(), "should return None when binary is absent");
}

/// `add_github_mcp_server` must add a `github` entry with the supplied
/// command. Does not touch environment variables.
#[test]
fn mcp_github_discover_adds_entry_with_explicit_command() {
    let mut config = roko_agent::mcp::McpConfig { servers: vec![] };
    let cmd = "/fake/path/roko-mcp-github".to_string();
    commands::mcp::add_github_mcp_server(&mut config, cmd.clone());

    assert_eq!(config.servers.len(), 1, "expected exactly one server entry");
    let s = &config.servers[0];
    assert_eq!(s.name, "github");
    assert_eq!(s.command, cmd);
    assert!(
        s.args.is_empty(),
        "auto-discovered entry should have no args"
    );
}

/// When the user already configured a `github` server, auto-discovery
/// must not add a duplicate entry.
#[test]
fn mcp_github_discover_respects_user_configured_github_server() {
    let tmp = tempdir().unwrap();

    // Place a fake binary so discovery would succeed if it weren't for the
    // existing user-configured server.
    let target_debug = tmp.path().join("target").join("debug");
    std::fs::create_dir_all(&target_debug).unwrap();
    create_fake_github_binary(&target_debug);

    // User-configured entry.
    let user_entry = roko_agent::mcp::McpServerConfig {
        name: "github".to_string(),
        transport: roko_agent::mcp::McpTransportConfig::Stdio,
        command: "/usr/local/bin/my-custom-github-mcp".to_string(),
        args: vec![],
        env: std::collections::HashMap::new(),
        endpoint: None,
        auth_token: None,
        tier: Default::default(),
    };

    let mut config = roko_agent::mcp::McpConfig {
        servers: vec![user_entry],
    };
    commands::mcp::augment_mcp_config_with_github(&mut config, tmp.path());

    assert_eq!(
        config.servers.len(),
        1,
        "user-configured server must not be duplicated"
    );
    assert_eq!(
        config.servers[0].command, "/usr/local/bin/my-custom-github-mcp",
        "user-configured command must be preserved"
    );
}

/// `augment_mcp_config_with_github` on a workdir without a target/ tree
/// must not panic and must leave the config unchanged when the binary is
/// absent from target/.  (If the binary is genuinely on PATH the test
/// correctly adds one entry — that is valid behaviour.)
#[test]
fn mcp_github_discover_skips_when_binary_absent_from_target() {
    let tmp = tempdir().unwrap();
    let mut config = roko_agent::mcp::McpConfig { servers: vec![] };
    commands::mcp::augment_mcp_config_with_github(&mut config, tmp.path());
    // Must not panic.  Binary count may be 0 (absent) or 1 (on real PATH).
    let _ = config.servers.len();
}

/// `resolve_mcp_config_with_autodiscovery` writes `mcp-auto.json` when the
/// github binary is found in `target/debug/` and no pre-existing MCP config
/// is present.
#[test]
fn mcp_github_discover_writes_auto_config_file() {
    let tmp = tempdir().unwrap();
    let roko_dir = tmp.path().join(".roko");
    std::fs::create_dir_all(&roko_dir).unwrap();

    // Place a fake binary inside target/debug relative to tmp so the
    // ancestor walk succeeds without touching PATH.
    let target_debug = tmp.path().join("target").join("debug");
    std::fs::create_dir_all(&target_debug).unwrap();
    create_fake_github_binary(&target_debug);

    let result = resolve_mcp_config_with_autodiscovery(tmp.path(), &roko_dir);

    // A path must be returned (binary was found in target/debug/).
    let path = result.expect("expected a config path when github binary is in target/");

    // The returned path should be the auto-generated file.
    assert_eq!(
        path,
        roko_dir.join("mcp-auto.json"),
        "expected mcp-auto.json, got: {}",
        path.display()
    );

    // The file must exist and contain the github server.
    let content = std::fs::read_to_string(&path).unwrap();
    let parsed: roko_agent::mcp::McpConfig = serde_json::from_str(&content).unwrap();
    assert!(
        parsed.servers.iter().any(|s| s.name == "github"),
        "mcp-auto.json must contain a 'github' server entry"
    );
}

// ── E31-T06: trigger subcommand parsing ────────────────────────────────

#[test]
fn cli_parses_trigger_list() {
    let cli = Cli::try_parse_from(["roko", "trigger", "list"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Trigger {
            cmd: commands::trigger::TriggerCmd::List { .. },
        })
    ));
}

#[test]
fn cli_parses_trigger_show() {
    let cli = Cli::try_parse_from(["roko", "trigger", "show", "my-hook"]).unwrap();
    match cli.command {
        Some(Command::Trigger {
            cmd: commands::trigger::TriggerCmd::Show { name, .. },
        }) => assert_eq!(name, "my-hook"),
        other => panic!("unexpected command variant: {other:?}"),
    }
}

#[test]
fn cli_parses_trigger_create() {
    let cli = Cli::try_parse_from([
        "roko",
        "trigger",
        "create",
        "deploy-hook",
        "--kind",
        "webhook",
        "--graph",
        "plans/deploy.toml",
    ])
    .unwrap();
    match cli.command {
        Some(Command::Trigger {
            cmd:
                commands::trigger::TriggerCmd::Create {
                    name, kind, graph, ..
                },
        }) => {
            assert_eq!(name, "deploy-hook");
            assert_eq!(kind, "webhook");
            assert_eq!(graph, "plans/deploy.toml");
        }
        other => panic!("unexpected command variant: {other:?}"),
    }
}

#[test]
fn cli_parses_trigger_fire() {
    let cli = Cli::try_parse_from([
        "roko",
        "trigger",
        "fire",
        "my-hook",
        "--payload",
        "{\"key\":\"value\"}",
    ])
    .unwrap();
    match cli.command {
        Some(Command::Trigger {
            cmd: commands::trigger::TriggerCmd::Fire { name, payload, .. },
        }) => {
            assert_eq!(name, "my-hook");
            assert_eq!(payload, "{\"key\":\"value\"}");
        }
        other => panic!("unexpected command variant: {other:?}"),
    }
}

#[test]
fn cli_parses_trigger_fire_default_payload() {
    let cli = Cli::try_parse_from(["roko", "trigger", "fire", "my-hook"]).unwrap();
    match cli.command {
        Some(Command::Trigger {
            cmd: commands::trigger::TriggerCmd::Fire { name, payload, .. },
        }) => {
            assert_eq!(name, "my-hook");
            assert_eq!(payload, "{}");
        }
        other => panic!("unexpected command variant: {other:?}"),
    }
}

// ── #262: CLI flag resolution contract tests ────────────────────

use roko_cli::resolved_overrides::{
    CascadePolicy, ConfigEditTarget, ConfigSetInput, DryRunPolicy, InteractionMode, LearnTuneInput,
    PlanRunInput, PresentationMode, ResolvedExecutionOverrides, RunInput, ServePolicy,
};

#[test]
fn cli_flags_model_alias_equivalence() {
    let cli_model = Cli::try_parse_from(["roko", "--model", "sonnet", "status"]).unwrap();
    let cli_force = Cli::try_parse_from(["roko", "--force-model", "sonnet", "status"]).unwrap();
    assert_eq!(cli_model.model, cli_force.model);
    assert_eq!(cli_model.model.as_deref(), Some("sonnet"));
}

#[test]
fn cli_flags_headless_resolves() {
    let cli = Cli::try_parse_from(["roko", "--headless", "status"]).unwrap();
    let flags = global_cli_flags(&cli);
    let overrides = ResolvedExecutionOverrides::for_run(&flags, &RunInput::default());
    assert_eq!(overrides.interaction_mode, InteractionMode::Headless);
}

#[test]
fn cli_flags_force_backend_alias_resolves_to_model() {
    let cli = Cli::try_parse_from(["roko", "--force-backend", "sonnet", "status"]).unwrap();
    assert_eq!(
        cli.model.as_deref(),
        Some("sonnet"),
        "--force-backend must resolve to the unified model field"
    );
}

#[test]
fn cli_flags_no_cascade_resolves() {
    let cli = Cli::try_parse_from(["roko", "status"]).unwrap();
    let flags = global_cli_flags(&cli);
    let input = RunInput {
        no_cascade: true,
        ..RunInput::default()
    };
    let overrides = ResolvedExecutionOverrides::for_run(&flags, &input);
    assert_eq!(overrides.cascade_policy, CascadePolicy::DisabledByUser);
}

#[test]
fn cli_flags_serve_required() {
    let cli = Cli::try_parse_from(["roko", "status"]).unwrap();
    let flags = global_cli_flags(&cli);
    let input = RunInput {
        serve_required: true,
        ..RunInput::default()
    };
    let overrides = ResolvedExecutionOverrides::for_run(&flags, &input);
    assert_eq!(overrides.serve_policy, ServePolicy::Required);
}

#[test]
fn cli_flags_no_serve_disabled() {
    let cli = Cli::try_parse_from(["roko", "--no-serve", "status"]).unwrap();
    let flags = global_cli_flags(&cli);
    let overrides = ResolvedExecutionOverrides::for_run(&flags, &RunInput::default());
    assert_eq!(overrides.serve_policy, ServePolicy::Disabled);
}

#[test]
fn cli_flags_plan_run_tui_presentation() {
    let cli = Cli::try_parse_from(["roko", "status"]).unwrap();
    let flags = global_cli_flags(&cli);
    let plan_tui = PlanRunInput {
        approval: true,
        ..PlanRunInput::default()
    };
    let plan_no_tui = PlanRunInput {
        no_tui: true,
        ..PlanRunInput::default()
    };
    let plan_auto = PlanRunInput::default();

    assert_eq!(
        ResolvedExecutionOverrides::for_plan_run(&flags, &plan_tui).presentation,
        PresentationMode::Tui,
    );
    assert_eq!(
        ResolvedExecutionOverrides::for_plan_run(&flags, &plan_no_tui).presentation,
        PresentationMode::Text,
    );
    assert_eq!(
        ResolvedExecutionOverrides::for_plan_run(&flags, &plan_auto).presentation,
        PresentationMode::Auto,
    );
}

#[test]
fn cli_flags_config_set_targets() {
    assert_eq!(
        ResolvedExecutionOverrides::resolve_config_edit_target(&ConfigSetInput {
            global: false,
            project: false,
        }),
        ConfigEditTarget::Global,
        "no flags defaults to Global for config set"
    );
    assert_eq!(
        ResolvedExecutionOverrides::resolve_config_edit_target(&ConfigSetInput {
            global: false,
            project: true,
        }),
        ConfigEditTarget::Project,
    );
}

#[test]
fn cli_flags_learn_tune_dry_run() {
    assert_eq!(
        ResolvedExecutionOverrides::resolve_tune_dry_run(&LearnTuneInput { dry_run: true }),
        DryRunPolicy::ReadOnlyNoMutation,
    );
    assert_eq!(
        ResolvedExecutionOverrides::resolve_tune_dry_run(&LearnTuneInput { dry_run: false }),
        DryRunPolicy::Execute,
    );
}

#[test]
fn cli_flags_global_flags_helper_roundtrip() {
    let cli = Cli::try_parse_from([
        "roko",
        "--model",
        "opus",
        "--role",
        "architect",
        "--effort",
        "high",
        "--json",
        "--quiet",
        "--headless",
        "--no-serve",
        "status",
    ])
    .unwrap();
    let flags = global_cli_flags(&cli);
    assert_eq!(flags.model, Some("opus"));
    assert_eq!(flags.role, Some("architect"));
    assert_eq!(flags.effort, Some("high"));
    assert!(flags.json);
    assert!(flags.quiet);
    assert!(flags.headless);
    assert!(flags.no_serve);
}

#[test]
fn cli_flags_plan_run_fresh_and_force_resume() {
    let cli = Cli::try_parse_from(["roko", "status"]).unwrap();
    let flags = global_cli_flags(&cli);
    let plan = PlanRunInput {
        fresh: true,
        force_resume: true,
        ..PlanRunInput::default()
    };
    let overrides = ResolvedExecutionOverrides::for_plan_run(&flags, &plan);
    assert!(overrides.fresh);
    assert!(overrides.force_resume);
}

// ── P2-FLG-1: global --json wires through to subcommands ────────

#[test]
fn cli_flags_json_parses_globally_for_plan_list() {
    // `roko --json plan list` must set cli.json = true.
    let cli = Cli::try_parse_from(["roko", "--json", "plan", "list"]).unwrap();
    assert!(cli.json, "--json must be set when passed before subcommand");
}

#[test]
fn cli_flags_json_parses_globally_for_history() {
    let cli = Cli::try_parse_from(["roko", "--json", "history"]).unwrap();
    assert!(
        cli.json,
        "--json must be set when passed before history subcommand"
    );
}

// ── P2-FLG-2: --role override for plan subcommands ────────────

#[test]
fn cli_flags_role_propagates_to_plan_context() {
    // Verify that --role appears in cli.role for plan subcommands.
    let cli = Cli::try_parse_from(["roko", "--role", "custom-writer", "plan", "list"]).unwrap();
    assert_eq!(
        cli.role.as_deref(),
        Some("custom-writer"),
        "--role must propagate to cli.role for plan subcommands"
    );
}

// ── #326: error_hint accuracy tests ────────────────────────────

#[test]
fn error_hint_state_recovery_required() {
    let hint = error_hint("state recovery required: snapshot corrupt");
    assert!(hint.is_some());
    assert!(hint.unwrap().contains("--fresh"));
}

#[test]
fn error_hint_state_snapshot_corrupt() {
    let hint = error_hint("state snapshot corrupt; trying backup");
    assert!(hint.is_some());
    assert!(hint.unwrap().contains("--fresh"));
}

#[test]
fn error_hint_authoritative_with_state_recovery() {
    // The word "authoritative" inside a StateRecoveryRequired message
    // must get the state recovery hint, not the API key hint.
    let hint = error_hint(
        "state recovery required: the snapshot at /tmp/state-snapshot.json is corrupt \
             (validate authoritative snapshot failed)",
    );
    assert!(hint.is_some());
    let h = hint.unwrap();
    assert!(
        !h.contains("API key"),
        "authoritative must not trigger API key hint, got: {h}"
    );
    assert!(h.contains("--fresh"), "should recommend --fresh, got: {h}");
}

#[test]
fn error_hint_authoritative_alone_no_api_key() {
    // "authoritative" alone must not produce an API key hint.
    let hint = error_hint(
        "resume validation failed: validate authoritative snapshot /tmp/state-snapshot.json",
    );
    assert!(
        hint.is_none() || !hint.unwrap().contains("API key"),
        "authoritative alone must not trigger API key hint"
    );
}

#[test]
fn error_hint_401_triggers_api_key() {
    let hint = error_hint("HTTP 401: unauthorized");
    assert!(hint.is_some());
    assert!(hint.unwrap().contains("API key"));
}

#[test]
fn error_hint_unauthorized_triggers_api_key() {
    let hint = error_hint("request failed: unauthorized");
    assert!(hint.is_some());
    assert!(hint.unwrap().contains("API key"));
}

#[test]
fn error_hint_invalid_api_key_triggers() {
    let hint = error_hint("invalid_api_key");
    assert!(hint.is_some());
    assert!(hint.unwrap().contains("API key"));
}

#[test]
fn error_hint_authentication_failed_triggers() {
    let hint = error_hint("authentication failed for provider");
    assert!(hint.is_some());
    assert!(hint.unwrap().contains("API key"));
}

#[test]
fn error_hint_authorization_policy_no_api_key() {
    let hint = error_hint("authorization policy denied tool execution");
    assert!(
        hint.is_none() || !hint.unwrap().contains("API key"),
        "authorization policy must not trigger API key hint"
    );
}

#[test]
fn error_hint_unrelated_returns_none() {
    assert!(error_hint("something completely unrelated went wrong").is_none());
}

#[test]
fn error_hint_ignores_401_outside_an_http_status() {
    for msg in [
        "/tmp/run-1401/checkpoint.json: No such file or directory",
        "worktree for gap-e4019c already exists",
        "wrote 14015 bytes to the snapshot",
        "task 401 of plan p1 failed verification",
    ] {
        let hint = error_hint(msg);
        assert!(
            hint.is_none() || !hint.unwrap().contains("API key"),
            "a 401 outside an HTTP status must not blame the API key: {msg}"
        );
    }
    for msg in [
        "HTTP 401",
        "request returned status 401",
        "server returned HTTP 401.",
    ] {
        let hint = error_hint(msg);
        assert!(
            hint.is_some_and(|h| h.contains("API key")),
            "an HTTP 401 must get the API key hint: {msg}"
        );
    }
}

#[test]
fn error_hint_points_provider_auth_at_provider_keys() {
    let msg = "API key invalid for provider 'openai' (HTTP 401). Check $OPENAI_API_KEY";
    let hint = error_hint(msg).expect("a provider 401 gets a hint");
    assert!(hint.contains("roko config check-secrets"), "got: {hint}");
    assert!(
        !hint.contains("ROKO_API_KEY"),
        "ROKO_API_KEY is the serve key, got: {hint}"
    );
}

#[test]
fn error_hint_points_serve_auth_at_roko_api_key() {
    let msg = "the workspace server rejected the request (401): server returned HTTP 401";
    let hint = error_hint(msg).expect("a serve 401 gets a hint");
    assert!(
        hint.contains("ROKO_API_KEY") && hint.contains("roko login"),
        "got: {hint}"
    );
}

// ─── Impact CLI parsing ─────────────────────────────────────────────

#[test]
fn cli_parses_impact_defaults() {
    let cli = Cli::try_parse_from(["roko", "impact"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Impact {
            ref base,
            ref files,
            json: false,
            workdir: None,
        }) if base == "HEAD" && files.is_empty()
    ));
}

#[test]
fn cli_parses_impact_with_base() {
    let cli = Cli::try_parse_from(["roko", "impact", "--base", "main"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Impact {
            ref base,
            ..
        }) if base == "main"
    ));
}

#[test]
fn cli_parses_impact_with_json_flag() {
    let cli = Cli::try_parse_from(["roko", "impact", "--json"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Impact { json: true, .. })
    ));
}

#[test]
fn cli_parses_impact_with_explicit_files() {
    let cli = Cli::try_parse_from([
        "roko",
        "impact",
        "--files",
        "crates/roko-core/src/lib.rs",
        "crates/roko-gate/src/lib.rs",
    ])
    .unwrap();
    match &cli.command {
        Some(Command::Impact { files, .. }) => {
            assert_eq!(files.len(), 2);
            assert_eq!(files[0], "crates/roko-core/src/lib.rs");
            assert_eq!(files[1], "crates/roko-gate/src/lib.rs");
        }
        other => panic!("expected Impact, got {other:?}"),
    }
}

#[test]
fn cli_parses_impact_with_workdir() {
    let cli = Cli::try_parse_from(["roko", "impact", "--workdir", "/some/path"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Command::Impact {
            workdir: Some(ref p),
            ..
        }) if p == Path::new("/some/path")
    ));
}
