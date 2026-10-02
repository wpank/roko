//! OpenAPI surface for the roko HTTP server.
//!
//! The document is assembled here so the route handlers can stay focused on
//! behavior while this module tracks the public HTTP surface.
// These functions are OpenAPI annotation stubs referenced by #[utoipa::path]
// macros. They are intentionally empty and never called at runtime — utoipa
// reads them at compile time to build the OpenAPI document. The blanket
// dead_code allow is correct and intentional for this file only.
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(clippy::needless_for_each)]

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::routing::get;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{OpenApi, ToSchema};

use crate::agent_lifecycle::{
    AgentObservationCommit, AgentRuntimeObservation, AgentSlotObservation, CompletedAgentTick,
    ObservedAgentMode, ObservedAgentRegime, ObservedLifecycleState, ObservedSlotState,
    ObservedVitalityPhase,
};
use crate::state::AppState;
use crate::subscription_relay::{
    ReconciliationRecord, RelayStreamBinding, ServeRelayConnectionStatus, SubscriptionRelayStatus,
};

/// Build the OpenAPI routes served under `/api`.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/openapi.json", get(openapi_json))
}

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "roko-serve API",
        version = env!("CARGO_PKG_VERSION"),
        description = "HTTP API exposed by roko-serve."
    ),
    servers((url = "/api")),
    tags(
        (name = "status", description = "Health, metrics, and dashboard endpoints"),
        (name = "plans", description = "Plan CRUD and execution"),
        (name = "run", description = "Single prompt execution endpoints"),
        (name = "run-observability", description = "Bounded read-only run evidence and event indexes"),
        (name = "templates", description = "Template CRUD and deploy endpoints"),
        (name = "deployments", description = "Cloud deployment endpoints"),
        (name = "agents", description = "Agent registration and lifecycle endpoints"),
        (name = "research", description = "Research and enhancement endpoints"),
        (name = "config", description = "Configuration endpoints"),
        (name = "subscriptions", description = "Subscription endpoints"),
        (name = "prds", description = "PRD endpoints"),
        (name = "webhooks", description = "Webhook ingress endpoints"),
        (name = "providers", description = "Provider and routing endpoints"),
        (name = "learning", description = "Learning and cascade endpoints"),
        (name = "aggregator", description = "Aggregation and knowledge endpoints"),
        (name = "diagnosis", description = "Diagnosis endpoints"),
        (name = "extensions", description = "Loaded extension metadata and health"),
        (name = "surfaces", description = "Typed StateHub-backed surface projections"),
        (name = "arenas", description = "Arena lifecycle, attempt execution, and settlement"),
        (name = "registries", description = "Passport and knowledge registry endpoints"),
        (name = "meta", description = "Meta-agent lineage proposal and activation"),
        (name = "connectors", description = "Authenticated connector transport lifecycle"),
        (name = "projections", description = "StateHub-backed projection and telemetry routes"),
        (name = "feeds", description = "Feed descriptor CRUD and runtime status"),
        (name = "recipes", description = "Recipe persistence and pure evaluation"),
        (name = "triggers", description = "Trigger binding CRUD and manual fire"),
        (name = "dreams", description = "Dream consolidation cycle and journal"),
        (name = "groups", description = "Persistent group membership, knowledge, and event APIs"),
        (name = "secrets", description = "Secret management endpoints"),
        (name = "jobs", description = "Marketplace job lifecycle and matching"),
        (name = "neuro", description = "Neuro knowledge query endpoint")
    ),
    paths(
        health,
        session_status,
        metrics_summary,
        dashboard,
        episodes,
        signals,
        operation_status,
        list_plans,
        get_plan,
        create_plan,
        execute_plan,
        plan_status,
        generate_plan,
        execute_plans,
        cancel_plan,
        revise_plan,
        plan_chat,
        get_plan_source,
        update_plan_source,
        start_run,
        run_status,
        run_observability_detail,
        run_observability_events,
        run_observability_event_stream,
        run_observability_tasks,
        run_observability_attempts,
        run_observability_gates,
        run_observability_logs,
        run_observability_metrics,
        run_observability_artifacts,
        run_observability_screenshots,
        run_observability_bundle,
        list_templates,
        create_template,
        get_template,
        delete_template,
        deploy_template,
        list_deployments,
        get_deployment,
        get_deployment_logs,
        proxy_task,
        receive_callback,
        list_managed_agents,
        register_agent,
        observe_agent_lifecycle,
        get_agent,
        stop_agent,
        agent_episodes,
        proxy_agent_logs,
        send_message,
        token_status,
        issue_token,
        list_research,
        research_topic,
        enhance_prd,
        enhance_plan,
        enhance_tasks,
        analyze,
        get_config,
        update_config,
        reload_config,
        list_subscriptions,
        relay_subscription_status,
        create_subscription,
        update_subscription,
        delete_subscription,
        enable_subscription,
        disable_subscription,
        list_prds,
        post_idea,
        get_prd,
        draft_prd,
        promote_prd,
        plan_from_prd,
        prds_coverage,
        consolidate_prds,
        github_webhook,
        slack_webhook,
        generic_webhook,
        list_providers,
        provider_health,
        test_provider,
        list_models,
        explain_routing,
        diagnosis_recent,
        learning_efficiency,
        learning_cascade_router,
        learning_cascade,
        learning_cost_tiers,
        learning_experiments,
        learning_adaptive_thresholds,
        learning_gate_thresholds,
        list_agents,
        agent_topology,
        agent_stats,
        agent_skills,
        agent_heartbeat,
        agent_trace,
        list_prediction_sessions,
        get_prediction_session,
        list_prediction_claims,
        list_knowledge_entries,
        list_knowledge_edges,
        search_knowledge,
        list_knowledge_kinds,
        list_tasks,
        task_stats,
        get_task,
        list_extensions,
        get_extension,
        workbench_surface,
        inbox_surface,
        canvas_surface,
        minimap_surface,
        autonomy_surface,
        list_arenas,
        create_arena,
        get_arena,
        transition_arena,
        get_arena_leaderboard,
        list_arena_attempts,
        start_arena_attempt,
        get_arena_attempt,
        submit_arena_attempt,
        settle_arena_attempt,
        list_passports,
        mint_passport,
        get_passport,
        passport_history,
        transfer_passport,
        update_passport_metadata,
        add_delegation,
        revoke_delegation,
        list_registry_knowledge,
        publish_registry_knowledge,
        get_registry_knowledge,
        validate_registry_knowledge,
        challenge_registry_knowledge,
        resolve_knowledge_challenge,
        list_registry_events,
        registry_stats,
        sync_indexer,
        rebuild_indexer,
        list_meta_agents,
        propose_meta_agent,
        get_meta_agent,
        validate_meta_agent,
        morph_meta_agent,
        rollback_meta_morph,
        deactivate_meta_agent,
        list_connectors,
        create_connector,
        delete_connector,
        connector_health,
        restart_connector,
        query_connector,
        execute_connector,
        projections_catalog,
        get_telemetry,
        stream_telemetry,
        get_named_projection,
        stream_named_projection,
        get_lens_runtimes,
        get_lens_runtime,
        reset_lens_runtime,
        enable_lens_runtime,
        disable_lens_runtime,
        get_statehub_projection,
        get_statehub_projection_history,
        list_feeds,
        create_feed,
        get_feed_catalog,
        list_runtime_feeds,
        get_runtime_feed_status,
        discover_feeds,
        search_feeds,
        feed_health,
        start_feed,
        stop_feed,
        get_feed,
        delete_feed,
        list_recipes,
        save_recipe,
        get_recipe,
        delete_recipe,
        evaluate_recipe,
        list_triggers,
        create_trigger,
        get_trigger,
        delete_trigger,
        trigger_history,
        fire_trigger,
        dream_run,
        dream_journal,
        list_groups,
        create_group,
        get_group,
        update_group,
        delete_group,
        invite_agent_to_group,
        list_invitations,
        accept_invitation,
        reject_invitation,
        list_group_members,
        update_group_member,
        remove_group_member,
        list_group_knowledge,
        publish_group_knowledge,
        list_pheromones,
        deposit_pheromone,
        publish_group_message,
        list_group_events,
        list_secrets,
        set_secret,
        delete_secret,
        test_secret,
        list_jobs,
        create_job,
        job_stats,
        match_jobs,
        get_job,
        update_job,
        cancel_job,
        assign_job,
        start_job,
        submit_job,
        evaluate_job,
        execute_job,
        cancel_job_endpoint,
        neuro_query,
        knowledge_query,
        retrieval_stats,
        retrieval_query,
        create_auth_session,
        end_auth_session,
        affect_state_handler,
        list_agent_tokens,
        issue_agent_token,
        revoke_agent_token,
        create_agent,
        get_agent_config,
        get_agent_profile,
        restart_agent,
        start_agent,
        list_api_keys,
        create_api_key,
        revoke_api_key,
        rotate_api_key,
        query_auth_audit,
        cost_summary,
        bench_events_sse,
        export_bench_run,
        start_matrix_run,
        bench_list_models,
        pareto_frontier,
        provider_status,
        start_bench_run,
        delete_bench_run,
        get_bench_run,
        bench_run_status,
        list_bench_runs,
        bench_start_bench_run,
        compare_bench_runs,
        bench_delete_bench_run,
        bench_get_bench_run,
        cancel_bench_run,
        list_suites,
        upload_suite,
        get_suite,
        list_swe_datasets,
        start_swe_run,
        list_swe_runs,
        get_swe_run,
        cfactor_trend,
        cache_prune,
        cache_status,
        chain_agents,
        chain_blocks,
        chain_bounties,
        chain_events,
        chain_status,
        chain_txs,
        chain_watcher_status,
        apply_preset,
        get_config_toml,
        get_dashboard_runs,
        post_defi_bonds,
        get_defi_bonds_id,
        get_defi_indices,
        get_defi_instruments,
        post_defi_insurance,
        post_defi_insurance_id_claims,
        post_defi_options_price,
        get_defi_risk_portfolio,
        create_deployment,
        teardown_deployment,
        doctor_report,
        sse_handler,
        ingest_event,
        ingest_event_batch,
        executor_state,
        gates_history,
        gate_summary,
        gate_history,
        pipeline_batch_flush,
        pipeline_batch_result,
        pipeline_batch_submit,
        pipeline_inference,
        gateway_models,
        gateway_stats,
        list_heartbeats,
        receive_heartbeat,
        list_history,
        get_history_session,
        batch_submit
    ),
    components(schemas(
        ApiErrorResponse,
        HealthResponse,
        SessionStatusResponse,
        ReloadResponse,
        IdResponse,
        NameResponse,
        OperationResponse,
        DeploymentCreateRequest,
        DeploymentCreateResponse,
        PlanCreateRequest,
        PlanCreateTask,
        RunRequest,
        TemplateCreateRequest,
        TemplateDeployRequest,
        AgentRegisterRequest,
        AgentRuntimeObservation,
        AgentObservationCommit,
        AgentSlotObservation,
        CompletedAgentTick,
        ObservedAgentMode,
        ObservedAgentRegime,
        ObservedLifecycleState,
        ObservedSlotState,
        ObservedVitalityPhase,
        AgentMessageRequest,
        TopicRequest,
        ConfigUpdateRequest,
        SubscriptionCreateRequest,
        SubscriptionUpdateRequest,
        ReconciliationRecord,
        RelayStreamBinding,
        ServeRelayConnectionStatus,
        SubscriptionRelayStatus,
        PrdIdeaRequest,
        DeploymentCallbackRequest,
        WebhookPayload,
        SearchQueryRequest
    ))
)]
struct ApiDoc;

macro_rules! doc_get {
    ($name:ident, $path:literal, $tag:literal) => {
        #[utoipa::path(
                    get,
                    path = $path,
                    tag = $tag,
                    responses(
                        (status = 200, description = "Successful response", body = Value),
                        (status = 400, description = "Bad request", body = ApiErrorResponse),
                        (status = 404, description = "Not found", body = ApiErrorResponse),
                        (status = 500, description = "Internal error", body = ApiErrorResponse)
                    )
                )]
        fn $name() {}
    };
}

macro_rules! doc_get_param {
    ($name:ident, $path:literal, $tag:literal, $param:literal) => {
        #[utoipa::path(
                    get,
                    path = $path,
                    tag = $tag,
                    params(($param = String, Path, description = "Path parameter")),
                    responses(
                        (status = 200, description = "Successful response", body = Value),
                        (status = 400, description = "Bad request", body = ApiErrorResponse),
                        (status = 404, description = "Not found", body = ApiErrorResponse),
                        (status = 500, description = "Internal error", body = ApiErrorResponse)
                    )
                )]
        fn $name() {}
    };
}

macro_rules! doc_post_value {
    ($name:ident, $path:literal, $tag:literal) => {
        #[utoipa::path(
                    post,
                    path = $path,
                    tag = $tag,
                    request_body = Value,
                    responses(
                        (status = 200, description = "Successful response", body = Value),
                        (status = 201, description = "Created", body = Value),
                        (status = 202, description = "Accepted", body = Value),
                        (status = 400, description = "Bad request", body = ApiErrorResponse),
                        (status = 401, description = "Unauthorized", body = ApiErrorResponse),
                        (status = 404, description = "Not found", body = ApiErrorResponse),
                        (status = 409, description = "Conflict", body = ApiErrorResponse),
                        (status = 500, description = "Internal error", body = ApiErrorResponse)
                    )
                )]
        fn $name() {}
    };
}

macro_rules! doc_put_value {
    ($name:ident, $path:literal, $tag:literal) => {
        #[utoipa::path(
                    put,
                    path = $path,
                    tag = $tag,
                    request_body = Value,
                    responses(
                        (status = 200, description = "Successful response", body = Value),
                        (status = 400, description = "Bad request", body = ApiErrorResponse),
                        (status = 404, description = "Not found", body = ApiErrorResponse),
                        (status = 500, description = "Internal error", body = ApiErrorResponse)
                    )
                )]
        fn $name() {}
    };
}

macro_rules! doc_delete {
    ($name:ident, $path:literal, $tag:literal) => {
        #[utoipa::path(
                    delete,
                    path = $path,
                    tag = $tag,
                    params(("id" = String, Path, description = "Path parameter")),
                    responses(
                        (status = 200, description = "Successful response", body = Value),
                        (status = 400, description = "Bad request", body = ApiErrorResponse),
                        (status = 404, description = "Not found", body = ApiErrorResponse),
                        (status = 500, description = "Internal error", body = ApiErrorResponse)
                    )
                )]
        fn $name() {}
    };
}

macro_rules! doc_delete_param {
    ($name:ident, $path:literal, $tag:literal, $param:literal) => {
        #[utoipa::path(
                    delete,
                    path = $path,
                    tag = $tag,
                    params(($param = String, Path, description = "Path parameter")),
                    responses(
                        (status = 200, description = "Successful response", body = Value),
                        (status = 400, description = "Bad request", body = ApiErrorResponse),
                        (status = 404, description = "Not found", body = ApiErrorResponse),
                        (status = 500, description = "Internal error", body = ApiErrorResponse)
                    )
                )]
        fn $name() {}
    };
}

doc_get!(health, "/health", "status");
doc_get!(session_status, "/status", "status");
doc_get!(metrics_summary, "/metrics/summary", "status");
doc_get!(dashboard, "/dashboard", "status");
doc_get!(episodes, "/episodes", "status");
doc_get!(signals, "/signals", "status");
doc_get_param!(operation_status, "/operations/{id}", "status", "id");
doc_get!(workbench_surface, "/projections/workbench", "surfaces");
doc_get!(inbox_surface, "/projections/inbox", "surfaces");
doc_get!(canvas_surface, "/projections/canvas", "surfaces");
doc_get!(minimap_surface, "/projections/minimap", "surfaces");
doc_get!(autonomy_surface, "/projections/autonomy", "surfaces");

doc_get!(list_plans, "/plans", "plans");
doc_get_param!(get_plan, "/plans/{id}", "plans", "id");
doc_post_value!(create_plan, "/plans", "plans");
doc_post_value!(execute_plan, "/plans/{id}/execute", "plans");
doc_get_param!(plan_status, "/plans/{id}/status", "plans", "id");
doc_post_value!(generate_plan, "/plans/generate", "plans");
doc_post_value!(execute_plans, "/plans/execute", "plans");
doc_post_value!(cancel_plan, "/plans/{id}/cancel", "plans");
doc_post_value!(revise_plan, "/plans/{id}/revise", "plans");
doc_post_value!(plan_chat, "/plans/{id}/chat", "plans");
doc_get_param!(get_plan_source, "/plans/{id}/source", "plans", "id");
doc_put_value!(update_plan_source, "/plans/{id}/source", "plans");

doc_post_value!(start_run, "/run", "run");
doc_get_param!(run_status, "/run/{id}/status", "run", "id");
doc_get_param!(
    run_observability_detail,
    "/runs/{run_id}",
    "run-observability",
    "run_id"
);
doc_get_param!(
    run_observability_events,
    "/runs/{run_id}/events",
    "run-observability",
    "run_id"
);
doc_get_param!(
    run_observability_event_stream,
    "/runs/{run_id}/events/stream",
    "run-observability",
    "run_id"
);
doc_get_param!(
    run_observability_tasks,
    "/runs/{run_id}/tasks",
    "run-observability",
    "run_id"
);
#[utoipa::path(
    get,
    path = "/runs/{run_id}/tasks/{task_id}/attempts",
    tag = "run-observability",
    params(
        ("run_id" = String, Path, description = "Validated run identifier"),
        ("task_id" = String, Path, description = "Validated task identifier")
    ),
    responses(
        (status = 200, description = "Bounded attempt events", body = Value),
        (status = 400, description = "Invalid identifier", body = ApiErrorResponse),
        (status = 403, description = "Requires loopback or authentication", body = ApiErrorResponse),
        (status = 404, description = "Run or task not found", body = ApiErrorResponse)
    )
)]
fn run_observability_attempts() {}
doc_get_param!(
    run_observability_gates,
    "/runs/{run_id}/gates",
    "run-observability",
    "run_id"
);
doc_get_param!(
    run_observability_logs,
    "/runs/{run_id}/logs",
    "run-observability",
    "run_id"
);
doc_get_param!(
    run_observability_metrics,
    "/runs/{run_id}/metrics",
    "run-observability",
    "run_id"
);
doc_get_param!(
    run_observability_artifacts,
    "/runs/{run_id}/artifacts",
    "run-observability",
    "run_id"
);
doc_get_param!(
    run_observability_screenshots,
    "/runs/{run_id}/screenshots",
    "run-observability",
    "run_id"
);
doc_get_param!(
    run_observability_bundle,
    "/runs/{run_id}/bundle",
    "run-observability",
    "run_id"
);

doc_get!(list_templates, "/templates", "templates");
doc_post_value!(create_template, "/templates", "templates");
doc_get_param!(get_template, "/templates/{name}", "templates", "name");
#[utoipa::path(
    delete,
    path = "/templates/{name}",
    tag = "templates",
    params(("name" = String, Path, description = "Template name")),
    responses(
        (status = 200, description = "Template removed", body = Value),
        (status = 500, description = "Internal error", body = ApiErrorResponse)
    )
)]
fn delete_template() {}
doc_post_value!(deploy_template, "/templates/{name}/deploy", "templates");

doc_get!(list_deployments, "/deployments", "deployments");
doc_get_param!(get_deployment, "/deployments/{id}", "deployments", "id");
doc_get_param!(
    get_deployment_logs,
    "/deployments/{id}/logs",
    "deployments",
    "id"
);
doc_post_value!(proxy_task, "/deployments/{id}/task", "deployments");
doc_post_value!(
    receive_callback,
    "/deployments/{id}/callback",
    "deployments"
);

doc_get!(list_managed_agents, "/managed-agents", "agents");
doc_post_value!(register_agent, "/agents/register", "agents");
#[utoipa::path(
    post,
    path = "/agents/{id}/observation",
    tag = "agents",
    params(("id" = String, Path, description = "Registered agent identifier")),
    request_body = AgentRuntimeObservation,
    responses(
        (status = 200, description = "Observation committed", body = AgentObservationCommit),
        (status = 400, description = "Invalid observation", body = ApiErrorResponse),
        (status = 404, description = "Agent not found", body = ApiErrorResponse),
        (status = 409, description = "Stale or conflicting sequence", body = ApiErrorResponse),
        (status = 500, description = "Durable commit failed", body = ApiErrorResponse)
    )
)]
fn observe_agent_lifecycle() {}
doc_get_param!(get_agent, "/agents/{id}", "agents", "id");
doc_post_value!(stop_agent, "/agents/{id}/stop", "agents");
doc_get_param!(agent_episodes, "/agents/{id}/episodes", "agents", "id");
doc_get_param!(proxy_agent_logs, "/agents/{id}/logs", "agents", "id");
doc_post_value!(send_message, "/agents/{id}/message", "agents");
doc_get_param!(token_status, "/agents/{id}/token", "agents", "id");
doc_post_value!(issue_token, "/agents/{id}/token", "agents");

doc_get!(list_research, "/research", "research");
doc_post_value!(research_topic, "/research/topic", "research");
doc_post_value!(enhance_prd, "/research/enhance-prd/{slug}", "research");
doc_post_value!(enhance_plan, "/research/enhance-plan/{plan}", "research");
doc_post_value!(enhance_tasks, "/research/enhance-tasks/{plan}", "research");
doc_post_value!(analyze, "/research/analyze", "research");

doc_get!(get_config, "/config", "config");
doc_put_value!(update_config, "/config", "config");
doc_post_value!(reload_config, "/config/reload", "config");

doc_get!(list_subscriptions, "/subscriptions", "subscriptions");
#[utoipa::path(
    get,
    path = "/subscriptions/relay/status",
    tag = "subscriptions",
    responses(
        (status = 200, description = "Durable relay subscription consumer status", body = SubscriptionRelayStatus),
        (status = 401, description = "Unauthorized", body = ApiErrorResponse),
        (status = 403, description = "Insufficient scope", body = ApiErrorResponse)
    )
)]
fn relay_subscription_status() {}
doc_post_value!(create_subscription, "/subscriptions", "subscriptions");
doc_put_value!(update_subscription, "/subscriptions/{id}", "subscriptions");
doc_delete!(delete_subscription, "/subscriptions/{id}", "subscriptions");
doc_post_value!(
    enable_subscription,
    "/subscriptions/{id}/enable",
    "subscriptions"
);
doc_post_value!(
    disable_subscription,
    "/subscriptions/{id}/disable",
    "subscriptions"
);

doc_get!(list_prds, "/prds", "prds");
doc_post_value!(post_idea, "/prds/ideas", "prds");
doc_get_param!(get_prd, "/prds/{slug}", "prds", "slug");
doc_post_value!(draft_prd, "/prds/{slug}/draft", "prds");
doc_post_value!(promote_prd, "/prds/{slug}/promote", "prds");
doc_post_value!(plan_from_prd, "/prds/{slug}/plan", "prds");
doc_get!(prds_coverage, "/prds/status", "prds");
doc_post_value!(consolidate_prds, "/prds/consolidate", "prds");

doc_post_value!(github_webhook, "/webhooks/github", "webhooks");
doc_post_value!(slack_webhook, "/webhooks/slack", "webhooks");
doc_post_value!(generic_webhook, "/webhooks/generic", "webhooks");

doc_get!(list_providers, "/providers", "providers");
doc_get_param!(provider_health, "/providers/{id}/health", "providers", "id");
doc_post_value!(test_provider, "/providers/{id}/test", "providers");
doc_get!(list_models, "/models", "providers");
doc_get!(explain_routing, "/routing/explain", "providers");

doc_get!(diagnosis_recent, "/diagnosis/recent", "diagnosis");

doc_get!(learning_efficiency, "/learning/efficiency", "learning");
doc_get!(
    learning_cascade_router,
    "/learning/cascade-router",
    "learning"
);
doc_get!(learning_cascade, "/learning/cascade", "learning");
doc_get!(learning_cost_tiers, "/learning/cost-tiers", "learning");
doc_get!(learning_experiments, "/learning/experiments", "learning");
doc_get!(
    learning_adaptive_thresholds,
    "/learning/adaptive-thresholds",
    "learning"
);
doc_get!(
    learning_gate_thresholds,
    "/learning/gate-thresholds",
    "learning"
);

doc_get!(list_agents, "/agents", "aggregator");
doc_get!(agent_topology, "/agents/topology", "aggregator");
doc_get_param!(agent_stats, "/agents/{id}/stats", "aggregator", "id");
doc_get_param!(agent_skills, "/agents/{id}/skills", "aggregator", "id");
doc_get_param!(
    agent_heartbeat,
    "/agents/{id}/heartbeat",
    "aggregator",
    "id"
);
doc_get_param!(agent_trace, "/agents/{id}/trace", "aggregator", "id");
doc_get!(
    list_prediction_sessions,
    "/predictions/sessions",
    "aggregator"
);
doc_get_param!(
    get_prediction_session,
    "/predictions/sessions/{id}",
    "aggregator",
    "id"
);
doc_get!(list_prediction_claims, "/predictions/claims", "aggregator");
doc_get!(list_knowledge_entries, "/knowledge/entries", "aggregator");
doc_get!(list_knowledge_edges, "/knowledge/edges", "aggregator");
doc_get!(search_knowledge, "/knowledge/search", "aggregator");
doc_get!(list_knowledge_kinds, "/knowledge/kinds", "aggregator");
doc_get!(list_tasks, "/tasks", "aggregator");
doc_get!(task_stats, "/tasks/stats", "aggregator");
doc_get_param!(get_task, "/tasks/{id}", "aggregator", "id");
doc_get!(list_extensions, "/extensions", "extensions");
doc_get_param!(get_extension, "/extensions/{name}", "extensions", "name");

// ── Arenas ────────────────────────────────────────────────────────────────
doc_get!(list_arenas, "/arenas", "arenas");
doc_post_value!(create_arena, "/arenas", "arenas");
doc_get_param!(get_arena, "/arenas/{id}", "arenas", "id");
#[utoipa::path(
    patch,
    path = "/arenas/{id}",
    tag = "arenas",
    params(("id" = String, Path, description = "Arena identifier (hex-encoded 32-byte hash)")),
    request_body = Value,
    responses(
        (status = 200, description = "Arena state after transition", body = Value),
        (status = 400, description = "Invalid action or arena id", body = ApiErrorResponse),
        (status = 401, description = "Unauthorized", body = ApiErrorResponse),
        (status = 403, description = "Not arena owner", body = ApiErrorResponse),
        (status = 404, description = "Arena not found", body = ApiErrorResponse),
        (status = 409, description = "Invalid state transition", body = ApiErrorResponse),
        (status = 500, description = "Internal error", body = ApiErrorResponse)
    )
)]
fn transition_arena() {}
doc_get_param!(
    get_arena_leaderboard,
    "/arenas/{id}/leaderboard",
    "arenas",
    "id"
);
doc_get_param!(list_arena_attempts, "/arenas/{id}/attempts", "arenas", "id");
doc_post_value!(start_arena_attempt, "/arenas/{id}/attempts", "arenas");
#[utoipa::path(
    get,
    path = "/arenas/{id}/attempts/{attempt_id}",
    tag = "arenas",
    params(
        ("id" = String, Path, description = "Arena identifier"),
        ("attempt_id" = String, Path, description = "Attempt identifier")
    ),
    responses(
        (status = 200, description = "Attempt detail", body = Value),
        (status = 404, description = "Arena or attempt not found", body = ApiErrorResponse),
        (status = 500, description = "Internal error", body = ApiErrorResponse)
    )
)]
fn get_arena_attempt() {}
#[utoipa::path(
    post,
    path = "/arenas/{id}/attempts/{attempt_id}/submit",
    tag = "arenas",
    params(
        ("id" = String, Path, description = "Arena identifier"),
        ("attempt_id" = String, Path, description = "Attempt identifier")
    ),
    request_body = Value,
    responses(
        (status = 200, description = "Updated attempt after submission", body = Value),
        (status = 400, description = "Invalid hashes", body = ApiErrorResponse),
        (status = 401, description = "Unauthorized", body = ApiErrorResponse),
        (status = 403, description = "Not attempt owner", body = ApiErrorResponse),
        (status = 404, description = "Arena or attempt not found", body = ApiErrorResponse),
        (status = 409, description = "Invalid attempt state", body = ApiErrorResponse),
        (status = 500, description = "Internal error", body = ApiErrorResponse)
    )
)]
fn submit_arena_attempt() {}
#[utoipa::path(
    post,
    path = "/arenas/{id}/attempts/{attempt_id}/settle",
    tag = "arenas",
    params(
        ("id" = String, Path, description = "Arena identifier"),
        ("attempt_id" = String, Path, description = "Attempt identifier")
    ),
    request_body = Value,
    responses(
        (status = 200, description = "Settled attempt with scoring evidence", body = Value),
        (status = 400, description = "Invalid evidence", body = ApiErrorResponse),
        (status = 401, description = "Unauthorized", body = ApiErrorResponse),
        (status = 403, description = "Not arena owner", body = ApiErrorResponse),
        (status = 404, description = "Arena or attempt not found", body = ApiErrorResponse),
        (status = 409, description = "Invalid attempt state", body = ApiErrorResponse),
        (status = 500, description = "Internal error", body = ApiErrorResponse)
    )
)]
fn settle_arena_attempt() {}

// ── Registries ────────────────────────────────────────────────────────────
doc_get!(list_passports, "/registries/passports", "registries");
doc_post_value!(mint_passport, "/registries/passports", "registries");
doc_get_param!(
    get_passport,
    "/registries/passports/{id}",
    "registries",
    "id"
);
doc_get_param!(
    passport_history,
    "/registries/passports/{id}/history",
    "registries",
    "id"
);
doc_post_value!(
    transfer_passport,
    "/registries/passports/{id}/transfer",
    "registries"
);
#[utoipa::path(
    put,
    path = "/registries/passports/{id}/metadata",
    tag = "registries",
    params(("id" = String, Path, description = "Passport identifier")),
    request_body = Value,
    responses(
        (status = 200, description = "Updated passport", body = Value),
        (status = 400, description = "Bad request", body = ApiErrorResponse),
        (status = 404, description = "Passport not found", body = ApiErrorResponse),
        (status = 500, description = "Internal error", body = ApiErrorResponse)
    )
)]
fn update_passport_metadata() {}
doc_post_value!(
    add_delegation,
    "/registries/passports/{id}/delegations",
    "registries"
);
#[utoipa::path(
    delete,
    path = "/registries/passports/{id}/delegations/{delegatee}",
    tag = "registries",
    params(
        ("id" = String, Path, description = "Passport identifier"),
        ("delegatee" = String, Path, description = "Delegatee identifier")
    ),
    responses(
        (status = 200, description = "Delegation revoked", body = Value),
        (status = 404, description = "Passport or delegation not found", body = ApiErrorResponse),
        (status = 500, description = "Internal error", body = ApiErrorResponse)
    )
)]
fn revoke_delegation() {}
doc_get!(
    list_registry_knowledge,
    "/registries/knowledge",
    "registries"
);
doc_post_value!(
    publish_registry_knowledge,
    "/registries/knowledge",
    "registries"
);
doc_get_param!(
    get_registry_knowledge,
    "/registries/knowledge/{id}",
    "registries",
    "id"
);
doc_post_value!(
    validate_registry_knowledge,
    "/registries/knowledge/{id}/validate",
    "registries"
);
doc_post_value!(
    challenge_registry_knowledge,
    "/registries/knowledge/{id}/challenge",
    "registries"
);
doc_post_value!(
    resolve_knowledge_challenge,
    "/registries/knowledge/challenges/{id}/resolve",
    "registries"
);
doc_get!(list_registry_events, "/registries/events", "registries");
doc_get!(registry_stats, "/registries/stats", "registries");
doc_post_value!(sync_indexer, "/registries/indexer/sync", "registries");
doc_post_value!(rebuild_indexer, "/registries/indexer/rebuild", "registries");

// ── Meta-agents ───────────────────────────────────────────────────────────
doc_get!(list_meta_agents, "/meta/agents", "meta");
doc_post_value!(propose_meta_agent, "/meta/agents", "meta");
doc_get_param!(get_meta_agent, "/meta/agents/{id}", "meta", "id");
doc_post_value!(validate_meta_agent, "/meta/agents/{id}/validate", "meta");
doc_post_value!(morph_meta_agent, "/meta/agents/{id}/morph", "meta");
doc_post_value!(
    rollback_meta_morph,
    "/meta/agents/{id}/morph/rollback",
    "meta"
);
doc_post_value!(
    deactivate_meta_agent,
    "/meta/agents/{id}/deactivate",
    "meta"
);

// ── Connectors ────────────────────────────────────────────────────────────
doc_get!(list_connectors, "/connectors", "connectors");
doc_post_value!(create_connector, "/connectors", "connectors");
doc_delete!(delete_connector, "/connectors/{name}", "connectors");
doc_get_param!(
    connector_health,
    "/connectors/{name}/health",
    "connectors",
    "name"
);
doc_post_value!(
    restart_connector,
    "/connectors/{name}/restart",
    "connectors"
);
doc_post_value!(query_connector, "/connectors/{name}/query", "connectors");
doc_post_value!(
    execute_connector,
    "/connectors/{name}/execute",
    "connectors"
);

// ── Projections / StateHub ────────────────────────────────────────────────
doc_get!(projections_catalog, "/projections/catalog", "projections");
doc_get!(get_telemetry, "/projections/telemetry", "projections");
doc_get!(
    stream_telemetry,
    "/projections/telemetry/stream",
    "projections"
);
doc_get_param!(
    get_named_projection,
    "/projections/{name}",
    "projections",
    "name"
);
doc_get_param!(
    stream_named_projection,
    "/projections/{name}/stream",
    "projections",
    "name"
);
doc_get!(get_lens_runtimes, "/statehub/lens-runtimes", "projections");
doc_get_param!(
    get_lens_runtime,
    "/statehub/lens-runtimes/{runtime_id}",
    "projections",
    "runtime_id"
);
#[utoipa::path(
    post,
    path = "/statehub/lens-runtimes/{runtime_id}/{lens}/reset",
    tag = "projections",
    params(
        ("runtime_id" = String, Path, description = "Lens runtime identifier"),
        ("lens" = String, Path, description = "Lens name")
    ),
    responses(
        (status = 200, description = "Lens reset", body = Value),
        (status = 404, description = "Runtime or lens not found", body = ApiErrorResponse),
        (status = 500, description = "Internal error", body = ApiErrorResponse)
    )
)]
fn reset_lens_runtime() {}
#[utoipa::path(
    post,
    path = "/statehub/lens-runtimes/{runtime_id}/{lens}/enable",
    tag = "projections",
    params(
        ("runtime_id" = String, Path, description = "Lens runtime identifier"),
        ("lens" = String, Path, description = "Lens name")
    ),
    responses(
        (status = 200, description = "Lens enabled", body = Value),
        (status = 404, description = "Runtime or lens not found", body = ApiErrorResponse),
        (status = 500, description = "Internal error", body = ApiErrorResponse)
    )
)]
fn enable_lens_runtime() {}
#[utoipa::path(
    post,
    path = "/statehub/lens-runtimes/{runtime_id}/{lens}/disable",
    tag = "projections",
    params(
        ("runtime_id" = String, Path, description = "Lens runtime identifier"),
        ("lens" = String, Path, description = "Lens name")
    ),
    responses(
        (status = 200, description = "Lens disabled", body = Value),
        (status = 404, description = "Runtime or lens not found", body = ApiErrorResponse),
        (status = 500, description = "Internal error", body = ApiErrorResponse)
    )
)]
fn disable_lens_runtime() {}
doc_get_param!(
    get_statehub_projection,
    "/statehub/{projection_id}",
    "projections",
    "projection_id"
);
doc_get_param!(
    get_statehub_projection_history,
    "/statehub/{projection_id}/history",
    "projections",
    "projection_id"
);

// ── Feeds ─────────────────────────────────────────────────────────────────
doc_get!(list_feeds, "/feeds", "feeds");
doc_post_value!(create_feed, "/feeds", "feeds");
doc_get!(get_feed_catalog, "/feeds/catalog", "feeds");
doc_get!(list_runtime_feeds, "/feeds/runtime", "feeds");
doc_get_param!(
    get_runtime_feed_status,
    "/feeds/runtime/{id}",
    "feeds",
    "id"
);
doc_get!(discover_feeds, "/feeds/discover", "feeds");
doc_get!(search_feeds, "/feeds/search", "feeds");
doc_get!(feed_health, "/feeds/health", "feeds");
doc_post_value!(start_feed, "/feeds/start/{id}", "feeds");
doc_post_value!(stop_feed, "/feeds/stop/{id}", "feeds");
doc_get_param!(get_feed, "/feeds/{id}", "feeds", "id");
doc_delete!(delete_feed, "/feeds/{id}", "feeds");

// ── Recipes ───────────────────────────────────────────────────────────────
doc_get!(list_recipes, "/recipes", "recipes");
doc_post_value!(save_recipe, "/recipes", "recipes");
doc_get_param!(get_recipe, "/recipes/{id}", "recipes", "id");
doc_delete!(delete_recipe, "/recipes/{id}", "recipes");
doc_post_value!(evaluate_recipe, "/recipes/{id}/evaluate", "recipes");

// ── Triggers ──────────────────────────────────────────────────────────────
doc_get!(list_triggers, "/triggers", "triggers");
doc_post_value!(create_trigger, "/triggers", "triggers");
doc_get_param!(get_trigger, "/triggers/{name}", "triggers", "name");
doc_delete!(delete_trigger, "/triggers/{name}", "triggers");
doc_get_param!(
    trigger_history,
    "/triggers/{name}/history",
    "triggers",
    "name"
);
doc_post_value!(fire_trigger, "/triggers/{name}/fire", "triggers");

// ── Dreams ────────────────────────────────────────────────────────────────
doc_post_value!(dream_run, "/dream/run", "dreams");
doc_get!(dream_journal, "/dream/journal", "dreams");

// ── Groups ────────────────────────────────────────────────────────────────
doc_get!(list_groups, "/groups", "groups");
doc_post_value!(create_group, "/groups", "groups");
doc_get_param!(get_group, "/groups/{id}", "groups", "id");
#[utoipa::path(
    patch,
    path = "/groups/{id}",
    tag = "groups",
    params(("id" = String, Path, description = "Group identifier")),
    request_body = Value,
    responses(
        (status = 200, description = "Updated group", body = Value),
        (status = 400, description = "Bad request", body = ApiErrorResponse),
        (status = 404, description = "Group not found", body = ApiErrorResponse),
        (status = 500, description = "Internal error", body = ApiErrorResponse)
    )
)]
fn update_group() {}
doc_delete!(delete_group, "/groups/{id}", "groups");
doc_post_value!(invite_agent_to_group, "/groups/{id}/invite", "groups");
doc_get_param!(list_invitations, "/groups/{id}/invitations", "groups", "id");
doc_post_value!(
    accept_invitation,
    "/invitations/{invitation_id}/accept",
    "groups"
);
doc_post_value!(
    reject_invitation,
    "/invitations/{invitation_id}/reject",
    "groups"
);
doc_get_param!(list_group_members, "/groups/{id}/members", "groups", "id");
#[utoipa::path(
    patch,
    path = "/groups/{id}/members/{agent_id}",
    tag = "groups",
    params(
        ("id" = String, Path, description = "Group identifier"),
        ("agent_id" = String, Path, description = "Agent identifier")
    ),
    request_body = Value,
    responses(
        (status = 200, description = "Updated member", body = Value),
        (status = 400, description = "Bad request", body = ApiErrorResponse),
        (status = 404, description = "Group or member not found", body = ApiErrorResponse),
        (status = 500, description = "Internal error", body = ApiErrorResponse)
    )
)]
fn update_group_member() {}
#[utoipa::path(
    delete,
    path = "/groups/{id}/members/{agent_id}",
    tag = "groups",
    params(
        ("id" = String, Path, description = "Group identifier"),
        ("agent_id" = String, Path, description = "Agent identifier")
    ),
    responses(
        (status = 200, description = "Member removed", body = Value),
        (status = 404, description = "Group or member not found", body = ApiErrorResponse),
        (status = 500, description = "Internal error", body = ApiErrorResponse)
    )
)]
fn remove_group_member() {}
doc_get_param!(
    list_group_knowledge,
    "/groups/{id}/knowledge",
    "groups",
    "id"
);
doc_post_value!(publish_group_knowledge, "/groups/{id}/knowledge", "groups");
doc_get_param!(list_pheromones, "/groups/{id}/pheromones", "groups", "id");
doc_post_value!(deposit_pheromone, "/groups/{id}/pheromones", "groups");
doc_post_value!(publish_group_message, "/groups/{id}/message", "groups");
doc_get_param!(list_group_events, "/groups/{id}/events", "groups", "id");

// ── Secrets ───────────────────────────────────────────────────────────────
doc_get!(list_secrets, "/secrets", "secrets");
#[utoipa::path(
    post,
    path = "/secrets/{namespace}/{key}",
    tag = "secrets",
    params(
        ("namespace" = String, Path, description = "Secret namespace"),
        ("key" = String, Path, description = "Secret key")
    ),
    request_body = Value,
    responses(
        (status = 200, description = "Secret stored", body = Value),
        (status = 400, description = "Bad request", body = ApiErrorResponse),
        (status = 500, description = "Internal error", body = ApiErrorResponse)
    )
)]
fn set_secret() {}
#[utoipa::path(
    delete,
    path = "/secrets/{namespace}/{key}",
    tag = "secrets",
    params(
        ("namespace" = String, Path, description = "Secret namespace"),
        ("key" = String, Path, description = "Secret key")
    ),
    responses(
        (status = 200, description = "Secret deleted", body = Value),
        (status = 404, description = "Secret not found", body = ApiErrorResponse),
        (status = 500, description = "Internal error", body = ApiErrorResponse)
    )
)]
fn delete_secret() {}
#[utoipa::path(
    post,
    path = "/secrets/{namespace}/{key}/test",
    tag = "secrets",
    params(
        ("namespace" = String, Path, description = "Secret namespace"),
        ("key" = String, Path, description = "Secret key")
    ),
    responses(
        (status = 200, description = "Secret connectivity test result", body = Value),
        (status = 404, description = "Secret not found", body = ApiErrorResponse),
        (status = 500, description = "Internal error", body = ApiErrorResponse)
    )
)]
fn test_secret() {}

// ── Jobs ──────────────────────────────────────────────────────────────────
doc_get!(list_jobs, "/jobs", "jobs");
doc_post_value!(create_job, "/jobs", "jobs");
doc_get!(job_stats, "/jobs/stats", "jobs");
doc_post_value!(match_jobs, "/jobs/match", "jobs");
doc_get_param!(get_job, "/jobs/{id}", "jobs", "id");
#[utoipa::path(
    patch,
    path = "/jobs/{id}",
    tag = "jobs",
    params(("id" = String, Path, description = "Job identifier")),
    request_body = Value,
    responses(
        (status = 200, description = "Updated job", body = Value),
        (status = 400, description = "Bad request", body = ApiErrorResponse),
        (status = 404, description = "Job not found", body = ApiErrorResponse),
        (status = 500, description = "Internal error", body = ApiErrorResponse)
    )
)]
fn update_job() {}
doc_delete!(cancel_job, "/jobs/{id}", "jobs");
doc_post_value!(assign_job, "/jobs/{id}/assign", "jobs");
doc_post_value!(start_job, "/jobs/{id}/start", "jobs");
doc_post_value!(submit_job, "/jobs/{id}/submit", "jobs");
doc_post_value!(evaluate_job, "/jobs/{id}/evaluate", "jobs");
doc_post_value!(execute_job, "/jobs/{id}/execute", "jobs");
doc_post_value!(cancel_job_endpoint, "/jobs/{id}/cancel", "jobs");

// ── Neuro ─────────────────────────────────────────────────────────────────
doc_post_value!(neuro_query, "/neuro/query", "neuro");
doc_get!(knowledge_query, "/knowledge", "neuro");
doc_get!(retrieval_stats, "/retrieval/stats", "neuro");
doc_get!(retrieval_query, "/retrieval/query", "neuro");

// ── Session exchange (mounted at `/api/auth/session`, outside the API key layer) ──
#[utoipa::path(
    post,
    path = "/auth/session",
    tag = "auth",
    responses(
        (status = 204, description = "Session created; `Set-Cookie` carries `roko_session`"),
        (status = 401, description = "Missing or invalid credential", body = ApiErrorResponse)
    )
)]
fn create_auth_session() {}
#[utoipa::path(
    delete,
    path = "/auth/session",
    tag = "auth",
    responses((status = 204, description = "Session ended and its cookie cleared"))
)]
fn end_auth_session() {}

// ── affect (gap-c50b85) ────────────────────────────────────────────────────────────
doc_get!(affect_state_handler, "/affect/state", "affect");

// ── agents (gap-c50b85) ────────────────────────────────────────────────────────────
doc_post_value!(create_agent, "/agents/create", "agents");
doc_get_param!(get_agent_config, "/agents/{id}/config", "agents", "id");
doc_get_param!(get_agent_profile, "/agents/{id}/profile", "agents", "id");
doc_post_value!(restart_agent, "/agents/{id}/restart", "agents");
doc_post_value!(start_agent, "/agents/{id}/start", "agents");

// ── auth (gap-c50b85) ──────────────────────────────────────────────────────────────
doc_get!(list_agent_tokens, "/agent-tokens", "auth");
doc_post_value!(issue_agent_token, "/agent-tokens", "auth");
doc_delete_param!(
    revoke_agent_token,
    "/agent-tokens/{token_id}",
    "auth",
    "token_id"
);
doc_get!(list_api_keys, "/api-keys", "auth");
doc_post_value!(create_api_key, "/api-keys", "auth");
doc_delete_param!(revoke_api_key, "/api-keys/{name}", "auth", "name");
doc_post_value!(rotate_api_key, "/api-keys/{name}/rotate", "auth");
doc_get!(query_auth_audit, "/auth/audit", "auth");

// ── bench (gap-c50b85) ─────────────────────────────────────────────────────────────
doc_get!(cost_summary, "/bench/cost-summary", "bench");
doc_get!(bench_events_sse, "/bench/events", "bench");
doc_get_param!(export_bench_run, "/bench/export/{id}", "bench", "id");
doc_post_value!(start_matrix_run, "/bench/matrix", "bench");
doc_get!(bench_list_models, "/bench/models", "bench");
doc_get!(pareto_frontier, "/bench/pareto", "bench");
doc_get!(provider_status, "/bench/provider-status", "bench");
doc_post_value!(start_bench_run, "/bench/run", "bench");
doc_delete!(delete_bench_run, "/bench/run/{id}", "bench");
doc_get_param!(get_bench_run, "/bench/run/{id}", "bench", "id");
doc_get_param!(bench_run_status, "/bench/run/{id}/status", "bench", "id");
doc_get!(list_bench_runs, "/bench/runs", "bench");
doc_post_value!(bench_start_bench_run, "/bench/runs", "bench");
doc_get!(compare_bench_runs, "/bench/runs/compare", "bench");
doc_delete!(bench_delete_bench_run, "/bench/runs/{id}", "bench");
doc_get_param!(bench_get_bench_run, "/bench/runs/{id}", "bench", "id");
doc_post_value!(cancel_bench_run, "/bench/runs/{id}/cancel", "bench");
doc_get!(list_suites, "/bench/suites", "bench");
doc_post_value!(upload_suite, "/bench/suites", "bench");
doc_get_param!(get_suite, "/bench/suites/{id}", "bench", "id");

// ── cache (gap-c50b85) ─────────────────────────────────────────────────────────────
doc_post_value!(cache_prune, "/cache/prune", "cache");

// ── learning (gap-c50b85) ──────────────────────────────────────────────────────────
doc_get!(cfactor_trend, "/c-factor/trend", "learning");

// ── swe_bench (gap-c50b85) ─────────────────────────────────────────────────────────
doc_get!(list_swe_datasets, "/bench/swe/datasets", "swe_bench");
doc_post_value!(start_swe_run, "/bench/swe/run", "swe_bench");
doc_get!(list_swe_runs, "/bench/swe/runs", "swe_bench");
doc_get_param!(get_swe_run, "/bench/swe/runs/{id}", "swe_bench", "id");

// ── cache (gap-c50b85) ─────────────────────────────────────────────────────────────
doc_get!(cache_status, "/cache/status", "cache");

// ── chain (gap-c50b85) ─────────────────────────────────────────────────────────────
doc_get!(chain_agents, "/chain/agents", "chain");
doc_get!(chain_blocks, "/chain/blocks", "chain");
doc_get!(chain_bounties, "/chain/bounties", "chain");
doc_get!(chain_events, "/chain/events", "chain");
doc_get!(chain_status, "/chain/status", "chain");
doc_get!(chain_txs, "/chain/transactions", "chain");
doc_get!(chain_watcher_status, "/chain/watcher", "chain");

// ── config (gap-c50b85) ────────────────────────────────────────────────────────────
doc_post_value!(apply_preset, "/config/preset", "config");
doc_get!(get_config_toml, "/config/toml", "config");

// ── defi (gap-c50b85) ──────────────────────────────────────────────────────────────
doc_post_value!(post_defi_bonds, "/defi/bonds", "defi");
doc_get_param!(get_defi_bonds_id, "/defi/bonds/{id}", "defi", "id");
doc_get!(get_defi_indices, "/defi/indices", "defi");
doc_get!(get_defi_instruments, "/defi/instruments", "defi");
doc_post_value!(post_defi_insurance, "/defi/insurance", "defi");
doc_post_value!(
    post_defi_insurance_id_claims,
    "/defi/insurance/{id}/claims",
    "defi"
);
doc_post_value!(post_defi_options_price, "/defi/options/price", "defi");
doc_get!(get_defi_risk_portfolio, "/defi/risk/portfolio", "defi");

// ── deployments (gap-c50b85) ───────────────────────────────────────────────────────
doc_post_value!(create_deployment, "/deployments", "deployments");
doc_delete!(teardown_deployment, "/deployments/{id}", "deployments");

// ── doctor (gap-c50b85) ────────────────────────────────────────────────────────────
doc_get!(doctor_report, "/doctor", "doctor");

// ── event_ingest (gap-c50b85) ──────────────────────────────────────────────────────
doc_post_value!(ingest_event, "/events/ingest", "event_ingest");
doc_post_value!(ingest_event_batch, "/events/ingest/batch", "event_ingest");

// ── gateway (gap-c50b85) ───────────────────────────────────────────────────────────
doc_post_value!(pipeline_batch_flush, "/gateway/batch/flush", "gateway");
doc_get_param!(
    pipeline_batch_result,
    "/gateway/batch/result/{id}",
    "gateway",
    "id"
);
doc_post_value!(pipeline_batch_submit, "/gateway/batch/submit", "gateway");
doc_post_value!(pipeline_inference, "/gateway/inference", "gateway");
doc_get!(gateway_models, "/gateway/models", "gateway");
doc_get!(gateway_stats, "/gateway/stats", "gateway");
doc_post_value!(batch_submit, "/inference/batch/submit", "gateway");

// ── heartbeats (gap-c50b85) ────────────────────────────────────────────────────────
doc_get!(list_heartbeats, "/heartbeats", "heartbeats");
doc_post_value!(receive_heartbeat, "/heartbeats", "heartbeats");

// ── history (gap-c50b85) ───────────────────────────────────────────────────────────
doc_get!(list_history, "/history", "history");
doc_get_param!(get_history_session, "/history/{id}", "history", "id");

// ── learning (gap-c50b85) ──────────────────────────────────────────────────────────
doc_get!(executor_state, "/executor/state", "learning");

// ── runs (gap-c50b85) ──────────────────────────────────────────────────────────────
doc_get!(get_dashboard_runs, "/dashboard/runs", "runs");

// ── sse (gap-c50b85) ───────────────────────────────────────────────────────────────
doc_get!(sse_handler, "/events", "sse");

// ── status (gap-c50b85) ────────────────────────────────────────────────────────────
doc_get!(gates_history, "/gates/history", "status");
doc_get!(gate_summary, "/gates/summary", "status");
doc_get_param!(
    gate_history,
    "/gates/{gate_name}/history",
    "status",
    "gate_name"
);

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiErrorResponse {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IdResponse {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NameResponse {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub active_plans: usize,
    pub active_agents: usize,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SessionStatusResponse {
    pub session_id: Option<String>,
    pub workdir: String,
    pub daemon_running: bool,
    pub signal_count: usize,
    pub episode_count: usize,
    pub last_episode_passed: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReloadResponse {
    pub success: bool,
    pub warnings: Vec<String>,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OperationResponse {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PlanCreateRequest {
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub tasks: Vec<PlanCreateTask>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PlanCreateTask {
    pub id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RunRequest {
    pub prompt: String,
    #[serde(default)]
    pub workdir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TemplateCreateRequest {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TemplateDeployRequest {
    #[serde(default)]
    pub params: Value,
    #[serde(default)]
    pub backend: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeploymentCreateRequest {
    pub template: String,
    #[serde(default)]
    pub params: Value,
    #[serde(default)]
    pub backend: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeploymentCreateResponse {
    pub id: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AgentRegisterRequest {
    pub agent_id: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AgentMessageRequest {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TopicRequest {
    pub topic: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ConfigUpdateRequest {
    #[serde(flatten)]
    pub value: Value,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SubscriptionCreateRequest {
    #[serde(flatten)]
    pub value: Value,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SubscriptionUpdateRequest {
    #[serde(flatten)]
    pub value: Value,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PrdIdeaRequest {
    pub idea: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeploymentCallbackRequest {
    #[serde(flatten)]
    pub value: Value,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WebhookPayload {
    #[serde(flatten)]
    pub value: Value,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SearchQueryRequest {
    #[serde(default)]
    pub q: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use roko_core::config::ServeAuthConfig;
    use tempfile::tempdir;
    use tower::ServiceExt;

    use crate::deploy::create_backend;
    use crate::routes::build_router;
    use crate::runtime::NoOpRuntime;
    use crate::state::AppState;

    #[tokio::test]
    async fn openapi_endpoint_is_served_under_api() {
        let dir = tempdir().expect("tempdir");
        let deploy_backend =
            Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(NoOpRuntime),
                roko_core::config::schema::RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        );

        let app = build_router(
            Arc::clone(&state),
            &[],
            ServeAuthConfig {
                enabled: false,
                ..ServeAuthConfig::default()
            },
        );
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/openapi.json")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let payload: Value = serde_json::from_slice(&body).expect("parse response body");
        assert_eq!(payload["openapi"], "3.1.0");
        assert!(payload["paths"]["/plans"].is_object());
        assert!(payload["paths"]["/agents/{id}/observation"]["post"].is_object());
        assert!(payload["paths"]["/subscriptions/relay/status"]["get"].is_object());
        for surface in ["workbench", "inbox", "canvas", "minimap", "autonomy"] {
            assert!(
                payload["paths"][format!("/projections/{surface}")]["get"].is_object(),
                "missing OpenAPI surface path {surface}"
            );
        }
        assert!(payload["components"]["schemas"]["AgentRuntimeObservation"].is_object());
        assert!(payload["components"]["schemas"]["AgentObservationCommit"].is_object());
        assert!(payload["components"]["schemas"]["SubscriptionRelayStatus"].is_object());
    }

    /// Registered routes the document does not describe yet (gap-c50b85).
    const UNDOCUMENTED_ROUTES: &str = include_str!("openapi_undocumented.txt");

    /// Methods a route registration can name. `any(...)` proxies are left out:
    /// they have no single method to document.
    const ROUTE_METHODS: [&str; 8] = [
        "get", "post", "put", "patch", "delete", "head", "options", "trace",
    ];

    /// Routers nested under a prefix inside `/api` (`routes/mod.rs`).
    const NESTED_ROUTERS: [(&str, &str, &str); 3] = [
        ("routes/providers.rs", "router", "/providers"),
        ("routes/providers.rs", "models_router", "/models"),
        ("routes/providers.rs", "routing_router", "/routing"),
    ];

    /// Routers `routes::build_router` mounts at the server root, not under
    /// `/api`: their paths outside `/api/` are not part of this document.
    const ROOT_ROUTERS: [(&str, &str); 7] = [
        ("routes/ws.rs", "routes"),
        ("routes/relay_proxy.rs", "routes"),
        ("terminal.rs", "routes"),
        ("routes/shared_runs.rs", "public_routes"),
        ("routes/webhooks.rs", "public_routes"),
        ("routes/triggers.rs", "public_routes"),
        ("routes/auth_session.rs", "routes"),
    ];

    /// Probes `routes::build_router` registers on the root router itself.
    const ROOT_PATHS: [&str; 3] = ["/health", "/ready", "/metrics"];

    #[test]
    fn openapi_documents_every_registered_route() {
        let registered = registered_routes();
        let documented = documented_routes();
        let listed: BTreeSet<&str> = UNDOCUMENTED_ROUTES
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();

        let missing: Vec<&str> = registered
            .iter()
            .map(String::as_str)
            .filter(|route| !documented.contains(*route) && !listed.contains(route))
            .collect();
        assert!(
            missing.is_empty(),
            "registered routes missing from the OpenAPI document; document them in openapi.rs \
             (or, failing that, list them in openapi_undocumented.txt): {missing:#?}"
        );

        let stale: Vec<&str> = listed
            .iter()
            .copied()
            .filter(|route| documented.contains(*route) || !registered.contains(*route))
            .collect();
        assert!(
            stale.is_empty(),
            "openapi_undocumented.txt lists routes that are documented or no longer registered; \
             delete these lines: {stale:#?}"
        );
    }

    /// `METHOD /path` for every operation in the document, parameters written `{}`.
    fn documented_routes() -> BTreeSet<String> {
        let doc = serde_json::to_value(ApiDoc::openapi()).expect("serialize OpenAPI document");
        let mut routes = BTreeSet::new();
        for (path, item) in doc["paths"].as_object().expect("paths object") {
            for method in ROUTE_METHODS {
                if item.get(method).is_some() {
                    let method = method.to_ascii_uppercase();
                    routes.insert(format!("{method} {}", normalize_params(path)));
                }
            }
        }
        routes
    }

    /// `METHOD /path` for every `.route("<literal>", <methods>)` registration in
    /// this crate's sources outside test modules, as served under `/api`, with
    /// parameters written `{}`. Root-mounted `/api/...` literals lose their
    /// prefix; other root-mounted routes are left out.
    fn registered_routes() -> BTreeSet<String> {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        collect_rust_files(&src, &mut files);
        let mut routes = BTreeSet::new();
        for file in files {
            if file.file_name().is_some_and(|name| name == "tests.rs") {
                continue;
            }
            let rel = file.strip_prefix(&src).expect("file under src");
            let rel = rel.to_string_lossy().replace('\\', "/");
            let source = std::fs::read_to_string(&file).expect("read source file");
            let text = production_part(&source);
            let mut from = 0;
            while let Some(found) = text[from..].find(".route(") {
                let open = from + found + ".route".len();
                from = open;
                let Some(close) = matching_paren(text, open) else {
                    continue;
                };
                let Some(literal) = text[open + 1..close].trim_start().strip_prefix('"') else {
                    continue;
                };
                let Some(end) = literal.find('"') else {
                    continue;
                };
                let Some(methods) = literal[end + 1..].trim_start().strip_prefix(',') else {
                    continue;
                };
                let path = &literal[..end];
                let router_fn = enclosing_fn(&text[..open]);
                let nest = NESTED_ROUTERS
                    .iter()
                    .find(|(nest_file, nest_fn, _)| *nest_file == rel && *nest_fn == router_fn);
                let root_router = ROOT_ROUTERS
                    .iter()
                    .any(|(root_file, root_fn)| *root_file == rel && *root_fn == router_fn);
                let root_probe = rel == "routes/mod.rs" && ROOT_PATHS.contains(&path);
                let api_path = match (path.strip_prefix("/api/"), nest) {
                    (Some(rest), _) => format!("/{rest}"),
                    (None, _) if root_router || root_probe => continue,
                    (None, Some((_, _, prefix))) if path == "/" => (*prefix).to_string(),
                    (None, Some((_, _, prefix))) => format!("{prefix}{path}"),
                    (None, None) => path.to_string(),
                };
                for method in method_names(methods) {
                    routes.insert(format!("{method} {}", normalize_params(&api_path)));
                }
            }
        }
        routes
    }

    fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read source dir") {
            let path = entry.expect("source dir entry").path();
            if path.is_dir() {
                collect_rust_files(&path, files);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }

    /// The source before its first `#[cfg(test)]` module.
    fn production_part(text: &str) -> &str {
        let mut from = 0;
        while let Some(found) = text[from..].find("#[cfg(test)]") {
            let at = from + found;
            let rest = text[at + "#[cfg(test)]".len()..].trim_start();
            let rest = rest
                .strip_prefix("pub(crate)")
                .or_else(|| rest.strip_prefix("pub"))
                .map_or(rest, str::trim_start);
            let is_module = rest
                .strip_prefix("mod")
                .is_some_and(|after| after.starts_with(char::is_whitespace));
            if is_module {
                return &text[..at];
            }
            from = at + 1;
        }
        text
    }

    /// Byte index of the bracket closing the one at `open`, skipping string literals.
    fn matching_paren(text: &str, open: usize) -> Option<usize> {
        let bytes = text.as_bytes();
        let mut depth = 0usize;
        let mut in_string = false;
        let mut i = open;
        while i < bytes.len() {
            match bytes[i] {
                b'\\' if in_string => i += 1,
                b'"' => in_string = !in_string,
                b'(' | b'[' | b'{' if !in_string => depth += 1,
                b')' | b']' | b'}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
            i += 1;
        }
        None
    }

    /// Name of the last `fn <name>` in `text`.
    fn enclosing_fn(text: &str) -> &str {
        let is_ident = |c: char| c.is_alphanumeric() || c == '_';
        let mut name = "";
        let mut from = 0;
        while let Some(found) = text[from..].find("fn ") {
            let at = from + found;
            from = at + "fn ".len();
            if text[..at].chars().next_back().is_some_and(is_ident) {
                continue;
            }
            let rest = text[from..].trim_start();
            let len = rest.find(|c: char| !is_ident(c)).unwrap_or(rest.len());
            if len > 0 {
                name = &rest[..len];
            }
        }
        name
    }

    /// Upper-cased HTTP methods named by a method router such as
    /// `get(list).post(create)`.
    fn method_names(expr: &str) -> Vec<String> {
        let mut methods = Vec::new();
        let mut rest = expr;
        loop {
            rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '.' || c == ',');
            let len = rest
                .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':'))
                .unwrap_or(rest.len());
            let callee = &rest[..len];
            let open = rest.len() - rest[len..].trim_start().len();
            if callee.is_empty() || !rest[open..].starts_with('(') {
                break;
            }
            let Some(close) = matching_paren(rest, open) else {
                break;
            };
            let name = callee.rsplit("::").next().unwrap_or(callee);
            if ROUTE_METHODS.contains(&name) {
                methods.push(name.to_ascii_uppercase());
            }
            rest = &rest[close + 1..];
        }
        methods
    }

    /// `path` with every `{param}` written `{}`.
    fn normalize_params(path: &str) -> String {
        let mut out = String::with_capacity(path.len());
        let mut in_param = false;
        for c in path.chars() {
            match c {
                '{' => {
                    in_param = true;
                    out.push_str("{}");
                }
                '}' => in_param = false,
                _ if in_param => {}
                _ => out.push(c),
            }
        }
        out
    }
}
