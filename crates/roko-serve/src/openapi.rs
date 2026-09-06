//! OpenAPI surface for the roko HTTP server.
//!
//! The document is assembled here so the route handlers can stay focused on
//! behavior while this module tracks the public HTTP surface.
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
        neuro_query
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
doc_get_param!(delete_template, "/templates/{name}", "templates", "name");
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
doc_get_param!(get_arena_leaderboard, "/arenas/{id}/leaderboard", "arenas", "id");
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
doc_get_param!(get_passport, "/registries/passports/{id}", "registries", "id");
doc_get_param!(passport_history, "/registries/passports/{id}/history", "registries", "id");
doc_post_value!(transfer_passport, "/registries/passports/{id}/transfer", "registries");
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
doc_post_value!(add_delegation, "/registries/passports/{id}/delegations", "registries");
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
doc_get!(list_registry_knowledge, "/registries/knowledge", "registries");
doc_post_value!(publish_registry_knowledge, "/registries/knowledge", "registries");
doc_get_param!(get_registry_knowledge, "/registries/knowledge/{id}", "registries", "id");
doc_post_value!(validate_registry_knowledge, "/registries/knowledge/{id}/validate", "registries");
doc_post_value!(challenge_registry_knowledge, "/registries/knowledge/{id}/challenge", "registries");
doc_post_value!(resolve_knowledge_challenge, "/registries/knowledge/challenges/{id}/resolve", "registries");
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
doc_post_value!(rollback_meta_morph, "/meta/agents/{id}/morph/rollback", "meta");
doc_post_value!(deactivate_meta_agent, "/meta/agents/{id}/deactivate", "meta");

// ── Connectors ────────────────────────────────────────────────────────────
doc_get!(list_connectors, "/connectors", "connectors");
doc_post_value!(create_connector, "/connectors", "connectors");
doc_delete!(delete_connector, "/connectors/{name}", "connectors");
doc_get_param!(connector_health, "/connectors/{name}/health", "connectors", "name");
doc_post_value!(restart_connector, "/connectors/{name}/restart", "connectors");
doc_post_value!(query_connector, "/connectors/{name}/query", "connectors");
doc_post_value!(execute_connector, "/connectors/{name}/execute", "connectors");

// ── Projections / StateHub ────────────────────────────────────────────────
doc_get!(projections_catalog, "/projections/catalog", "projections");
doc_get!(get_telemetry, "/projections/telemetry", "projections");
doc_get!(stream_telemetry, "/projections/telemetry/stream", "projections");
doc_get_param!(get_named_projection, "/projections/{name}", "projections", "name");
doc_get_param!(stream_named_projection, "/projections/{name}/stream", "projections", "name");
doc_get!(get_lens_runtimes, "/statehub/lens-runtimes", "projections");
doc_get_param!(get_lens_runtime, "/statehub/lens-runtimes/{runtime_id}", "projections", "runtime_id");
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
doc_get_param!(get_statehub_projection, "/statehub/{projection_id}", "projections", "projection_id");
doc_get_param!(get_statehub_projection_history, "/statehub/{projection_id}/history", "projections", "projection_id");

// ── Feeds ─────────────────────────────────────────────────────────────────
doc_get!(list_feeds, "/feeds", "feeds");
doc_post_value!(create_feed, "/feeds", "feeds");
doc_get!(get_feed_catalog, "/feeds/catalog", "feeds");
doc_get!(list_runtime_feeds, "/feeds/runtime", "feeds");
doc_get_param!(get_runtime_feed_status, "/feeds/runtime/{id}", "feeds", "id");
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
doc_get_param!(trigger_history, "/triggers/{name}/history", "triggers", "name");
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
doc_post_value!(accept_invitation, "/invitations/{invitation_id}/accept", "groups");
doc_post_value!(reject_invitation, "/invitations/{invitation_id}/reject", "groups");
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
doc_get_param!(list_group_knowledge, "/groups/{id}/knowledge", "groups", "id");
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
}
