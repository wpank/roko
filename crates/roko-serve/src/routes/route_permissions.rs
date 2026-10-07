//! Canonical HTTP route-to-RBAC permission mapping.
//!
//! Authentication scopes answer whether a credential may address a broad
//! class of routes. This table is the second, typed authorization boundary:
//! it maps each request to the workspace permission required to perform it.

use axum::http::Method;

use crate::rbac::Permission;

/// An ordered route-prefix permission rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RoutePermission {
    /// HTTP path prefix. More-specific entries must precede broader ones.
    pub prefix: &'static str,
    /// Permission required for matching requests.
    pub permission: Permission,
}

/// Explicit rules for security-sensitive and commonly mutated route groups.
///
/// Prefixes are externally visible paths. A row outside `/api` (such as
/// `/relay`) belongs to a router merged at the root and matches the path its
/// middleware sees unchanged.
pub(crate) const ROUTE_PERMISSION_MANIFEST: &[RoutePermission] = &[
    RoutePermission {
        prefix: "/api/auth/audit",
        permission: Permission::SecretsRead,
    },
    RoutePermission {
        prefix: "/api/api-keys",
        permission: Permission::ApiKeyCreate,
    },
    RoutePermission {
        prefix: "/api/agent-tokens",
        permission: Permission::TokenIssue,
    },
    RoutePermission {
        prefix: "/api/relay-tokens",
        permission: Permission::TokenIssue,
    },
    RoutePermission {
        prefix: "/api/team/join",
        permission: Permission::ViewDashboard,
    },
    RoutePermission {
        prefix: "/api/team",
        permission: Permission::TeamManage,
    },
    RoutePermission {
        prefix: "/api/secrets",
        permission: Permission::SecretsWrite,
    },
    RoutePermission {
        prefix: "/api/config",
        permission: Permission::ConfigEdit,
    },
    RoutePermission {
        prefix: "/api/registries",
        permission: Permission::ConfigEdit,
    },
    RoutePermission {
        prefix: "/api/plans",
        permission: Permission::PlanCreate,
    },
    RoutePermission {
        prefix: "/api/agents",
        permission: Permission::AgentSpawn,
    },
    RoutePermission {
        prefix: "/api/meta",
        permission: Permission::AgentSpawn,
    },
    RoutePermission {
        prefix: "/api/groups",
        permission: Permission::PlanCreate,
    },
    RoutePermission {
        prefix: "/api/arenas",
        permission: Permission::PlanCreate,
    },
    RoutePermission {
        prefix: "/api/defi",
        permission: Permission::PlanExecute,
    },
    RoutePermission {
        prefix: "/api/marketplace",
        permission: Permission::PlanCreate,
    },
    RoutePermission {
        prefix: "/api/invitations",
        permission: Permission::PlanCreate,
    },
    RoutePermission {
        prefix: "/api/events/ingest",
        permission: Permission::ConfigEdit,
    },
    RoutePermission {
        prefix: "/api/terminal",
        permission: Permission::AgentSpawn,
    },
    RoutePermission {
        prefix: "/api/workspaces",
        permission: Permission::ConfigEdit,
    },
    RoutePermission {
        prefix: "/api/jobs",
        permission: Permission::PlanExecute,
    },
    RoutePermission {
        prefix: "/api/run",
        permission: Permission::PlanExecute,
    },
    RoutePermission {
        prefix: "/api/research",
        permission: Permission::PlanExecute,
    },
    RoutePermission {
        prefix: "/api/dream",
        permission: Permission::PlanExecute,
    },
    RoutePermission {
        prefix: "/api/deployments",
        permission: Permission::ConfigEdit,
    },
    RoutePermission {
        prefix: "/api/subscriptions",
        permission: Permission::ConfigEdit,
    },
    RoutePermission {
        prefix: "/api/templates",
        permission: Permission::PlanCreate,
    },
    RoutePermission {
        prefix: "/api/heartbeats",
        permission: Permission::ConfigEdit,
    },
    RoutePermission {
        prefix: "/api/neuro",
        permission: Permission::ConfigEdit,
    },
    RoutePermission {
        prefix: "/api/inference",
        permission: Permission::PlanExecute,
    },
    RoutePermission {
        prefix: "/api/gateway",
        permission: Permission::PlanExecute,
    },
    RoutePermission {
        prefix: "/api/bench",
        permission: Permission::PlanExecute,
    },
    RoutePermission {
        prefix: "/api/connectors",
        permission: Permission::ConfigEdit,
    },
    RoutePermission {
        prefix: "/api/feeds",
        permission: Permission::ConfigEdit,
    },
    RoutePermission {
        prefix: "/api/recipes",
        permission: Permission::ConfigEdit,
    },
    RoutePermission {
        prefix: "/api/rpc",
        permission: Permission::PlanExecute,
    },
    RoutePermission {
        prefix: "/api/vision-loop",
        permission: Permission::PlanExecute,
    },
    RoutePermission {
        prefix: "/api/webhooks",
        permission: Permission::ConfigEdit,
    },
    RoutePermission {
        prefix: "/api/providers",
        permission: Permission::ConfigEdit,
    },
    // Releasing an immune isolation control lifts a security block.
    RoutePermission {
        prefix: "/api/safety",
        permission: Permission::ConfigEdit,
    },
    // Approving a staged outbound effect runs a plan agent's held tool call
    // (9133).
    RoutePermission {
        prefix: "/api/effects",
        permission: Permission::PlanExecute,
    },
    RoutePermission {
        prefix: "/relay",
        permission: Permission::AgentSpawn,
    },
    // Every MCP call is a POST; its tools only read so far (9114), and
    // `tools/call` checks each tool's own scope.
    RoutePermission {
        prefix: "/mcp",
        permission: Permission::ViewDashboard,
    },
    RoutePermission {
        prefix: "/ws/terminal",
        permission: Permission::AgentSpawn,
    },
];

/// Routes whose GET opens an interactive session: a WebSocket upgrade into a
/// shell. A GET there is not a read, so it takes the route's write scope and
/// RBAC permission like a mutation.
const SESSION_OPENING_ROUTES: &[&str] = &["/ws/terminal"];

/// Whether a request to `path` opens an interactive session (see
/// [`SESSION_OPENING_ROUTES`]).
pub(crate) fn opens_interactive_session(path: &str) -> bool {
    SESSION_OPENING_ROUTES
        .iter()
        .any(|prefix| path_has_segment_prefix(path, prefix))
}

/// Whether `path` is `prefix` itself or lies below it.
///
/// A prefix matches only at a path-segment boundary, so `/relay` covers
/// `/relay/agents` but not `/relay-tokens`. A prefix that ends in `/` is
/// already at a boundary.
pub(crate) fn path_has_segment_prefix(path: &str, prefix: &str) -> bool {
    path.strip_prefix(prefix)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/') || prefix.ends_with('/'))
}

/// Resolve the typed permission required for a request.
///
/// Read-only requests normally rely on authentication plus their route's
/// existing scope. Audit and secret reads are intentionally stronger, and a
/// GET that opens an interactive session is treated as a mutation. Every
/// mutation receives a typed permission: unmatched mutations fail closed to
/// [`Permission::ConfigEdit`] rather than bypassing RBAC.
pub(crate) fn required_permission_for(method: &Method, path: &str) -> Option<Permission> {
    // Axum strips the `/api` nest prefix before invoking middleware attached
    // to the nested router. Accept both that runtime form and the canonical
    // externally visible path used by tests, logs, and documentation. Routers
    // merged at the root keep their own prefix, so a path in one of their
    // families is matched as it arrives instead of being moved under `/api`.
    let nested_path;
    let path = if path.starts_with("/api/") || is_root_mounted(path) {
        path
    } else {
        nested_path = format!("/api{path}");
        &nested_path
    };

    if path.starts_with("/api/auth/audit") {
        return Some(Permission::SecretsRead);
    }
    if path.starts_with("/api/secrets") && matches!(*method, Method::GET | Method::HEAD) {
        return Some(Permission::SecretsRead);
    }

    if matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
        && !opens_interactive_session(path)
    {
        return None;
    }

    // A running plan and an agent stop are narrower than their parent route
    // groups, so classify them before consulting the prefix manifest.
    if path.starts_with("/api/plans/") && (path.ends_with("/run") || path.ends_with("/execute")) {
        return Some(Permission::PlanExecute);
    }
    if path.starts_with("/api/agents/") && (*method == Method::DELETE || path.ends_with("/stop")) {
        return Some(Permission::AgentStop);
    }
    if path.starts_with("/api/arenas/") && path.contains("/attempts") {
        return Some(Permission::PlanExecute);
    }

    ROUTE_PERMISSION_MANIFEST
        .iter()
        .find(|entry| path.starts_with(entry.prefix))
        .map(|entry| entry.permission)
        .or(Some(Permission::ConfigEdit))
}

/// Whether `path` falls under a manifest row declared outside `/api`.
///
/// The match is per path segment, so `/relay-tokens` (the nest-stripped form
/// of `/api/relay-tokens`) is not mistaken for the root-mounted `/relay`
/// family and keeps its own row.
fn is_root_mounted(path: &str) -> bool {
    ROUTE_PERMISSION_MANIFEST
        .iter()
        .filter(|entry| !entry.prefix.starts_with("/api/"))
        .any(|entry| path_has_segment_prefix(path, entry.prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_are_unrestricted_except_sensitive_metadata() {
        assert_eq!(required_permission_for(&Method::GET, "/api/jobs"), None);
        assert_eq!(
            required_permission_for(&Method::GET, "/api/auth/audit"),
            Some(Permission::SecretsRead)
        );
        assert_eq!(
            required_permission_for(&Method::GET, "/api/secrets"),
            Some(Permission::SecretsRead)
        );
    }

    #[test]
    fn specialized_mutations_override_parent_prefixes() {
        assert_eq!(
            required_permission_for(&Method::POST, "/api/plans/demo/run"),
            Some(Permission::PlanExecute)
        );
        assert_eq!(
            required_permission_for(&Method::POST, "/api/agents/demo/stop"),
            Some(Permission::AgentStop)
        );
    }

    #[test]
    fn group_mutations_require_member_level_create_permission() {
        assert_eq!(
            required_permission_for(&Method::POST, "/api/groups"),
            Some(Permission::PlanCreate)
        );
        assert_eq!(
            required_permission_for(&Method::POST, "/api/invitations/inv-1/accept"),
            Some(Permission::PlanCreate)
        );
        assert_eq!(required_permission_for(&Method::GET, "/api/groups"), None);
    }

    #[test]
    fn arena_creation_and_attempts_have_distinct_permissions() {
        assert_eq!(
            required_permission_for(&Method::POST, "/api/arenas"),
            Some(Permission::PlanCreate)
        );
        assert_eq!(
            required_permission_for(&Method::POST, "/api/arenas/demo/attempts"),
            Some(Permission::PlanExecute)
        );
        assert_eq!(
            required_permission_for(&Method::POST, "/api/arenas/demo/attempts/attempt-1/settle"),
            Some(Permission::PlanExecute)
        );
        assert_eq!(required_permission_for(&Method::GET, "/api/arenas"), None);
    }

    #[test]
    fn meta_mutations_require_agent_spawn_permission() {
        for path in [
            "/api/meta/agents",
            "/api/meta/agents/demo/validate",
            "/api/meta/agents/demo/morph",
            "/api/meta/agents/demo/morph/rollback",
            "/api/meta/agents/demo/deactivate",
        ] {
            assert_eq!(
                required_permission_for(&Method::POST, path),
                Some(Permission::AgentSpawn)
            );
        }
        assert_eq!(
            required_permission_for(&Method::GET, "/api/meta/agents"),
            None
        );
    }

    #[test]
    fn defi_reads_are_read_only_and_mutations_require_execution_permission() {
        assert_eq!(
            required_permission_for(&Method::POST, "/api/defi/bonds"),
            Some(Permission::PlanExecute)
        );
        assert_eq!(
            required_permission_for(&Method::POST, "/api/defi/insurance/policy-1/claims"),
            Some(Permission::PlanExecute)
        );
        assert_eq!(
            required_permission_for(&Method::GET, "/api/defi/instruments"),
            None
        );
    }

    #[test]
    fn registry_reads_are_read_only_and_mutations_require_config_permission() {
        assert_eq!(
            required_permission_for(&Method::GET, "/api/registries/passports"),
            None
        );
        for (method, path) in [
            (Method::POST, "/api/registries/passports"),
            (Method::POST, "/api/registries/passports/1/transfer"),
            (Method::PUT, "/api/registries/passports/1/metadata"),
            (Method::POST, "/api/registries/passports/1/delegations"),
            (Method::DELETE, "/api/registries/passports/1/delegations/2"),
            (Method::POST, "/api/registries/knowledge"),
            (Method::POST, "/api/registries/knowledge/abc/validate"),
            (Method::POST, "/api/registries/knowledge/abc/challenge"),
            (
                Method::POST,
                "/api/registries/knowledge/challenges/abc/resolve",
            ),
            (Method::POST, "/api/registries/indexer/sync"),
            (Method::POST, "/api/registries/indexer/rebuild"),
        ] {
            assert_eq!(
                required_permission_for(&method, path),
                Some(Permission::ConfigEdit),
                "{method} {path}"
            );
        }
    }

    #[test]
    fn marketplace_reads_are_read_only_and_mutations_require_create_permission() {
        assert_eq!(
            required_permission_for(&Method::GET, "/api/marketplace/browse"),
            None
        );
        assert_eq!(
            required_permission_for(&Method::POST, "/api/marketplace/publish"),
            Some(Permission::PlanCreate)
        );
        assert_eq!(
            required_permission_for(&Method::POST, "/api/marketplace/fork"),
            Some(Permission::PlanCreate)
        );
    }

    #[test]
    fn team_join_is_available_to_any_authenticated_workspace_role() {
        assert_eq!(
            required_permission_for(&Method::POST, "/api/team/join"),
            Some(Permission::ViewDashboard)
        );
        assert_eq!(
            required_permission_for(&Method::POST, "/api/team/invite"),
            Some(Permission::TeamManage)
        );
    }

    #[test]
    fn relay_path_requires_agent_spawn() {
        // The relay proxy is merged at the root, so its middleware sees
        // `/relay/...` itself; that path must hit the `/relay` row rather
        // than fall through to the `ConfigEdit` catch-all.
        for path in ["/relay", "/relay/agents", "/relay/agents/123"] {
            for method in [Method::POST, Method::PUT, Method::DELETE] {
                assert_eq!(
                    required_permission_for(&method, path),
                    Some(Permission::AgentSpawn),
                    "{method} {path}"
                );
            }
        }
        assert_eq!(required_permission_for(&Method::GET, "/relay/agents"), None);
        // `/relay-tokens` is the nest-stripped form of `/api/relay-tokens`;
        // sharing the `/relay` spelling must not move it off its own row.
        for path in ["/relay-tokens", "/api/relay-tokens", "/relay-tokens/tok-1"] {
            assert_eq!(
                required_permission_for(&Method::POST, path),
                Some(Permission::TokenIssue),
                "{path}"
            );
        }
        assert_eq!(
            required_permission_for(&Method::POST, "/relayed"),
            Some(Permission::ConfigEdit)
        );
    }

    #[test]
    fn opening_a_terminal_websocket_requires_agent_spawn() {
        for method in [Method::GET, Method::HEAD, Method::OPTIONS] {
            assert_eq!(
                required_permission_for(&method, "/ws/terminal/abc-123"),
                Some(Permission::AgentSpawn),
                "{method}"
            );
        }
        // Only the session-opening route changes; other reads stay open.
        assert_eq!(required_permission_for(&Method::GET, "/ws/events"), None);
        assert_eq!(required_permission_for(&Method::GET, "/ws/terminals"), None);
    }

    #[test]
    fn releasing_an_isolation_control_requires_config_edit() {
        assert_eq!(
            required_permission_for(&Method::POST, "/api/safety/controls/plan%2Ftask/release"),
            Some(Permission::ConfigEdit)
        );
        assert_eq!(
            required_permission_for(&Method::POST, "/safety/controls/plan%2Ftask/release"),
            Some(Permission::ConfigEdit)
        );
        assert_eq!(
            required_permission_for(&Method::GET, "/api/safety/controls"),
            None
        );
    }

    #[test]
    fn deciding_a_staged_effect_requires_plan_execute() {
        assert_eq!(
            required_permission_for(&Method::POST, "/api/effects/effect-1/decision"),
            Some(Permission::PlanExecute)
        );
        assert_eq!(required_permission_for(&Method::GET, "/api/effects"), None);
    }

    #[test]
    fn segment_prefixes_stop_at_path_boundaries() {
        assert!(path_has_segment_prefix("/relay", "/relay"));
        assert!(path_has_segment_prefix("/relay/agents", "/relay"));
        assert!(!path_has_segment_prefix("/relay-tokens", "/relay"));
        assert!(!path_has_segment_prefix("/relayed/x", "/relay"));
        assert!(path_has_segment_prefix("/hooks/plugin/x", "/hooks/"));
        assert!(!path_has_segment_prefix("/api", "/api/"));
    }

    #[test]
    fn unclassified_mutations_fail_closed() {
        assert_eq!(
            required_permission_for(&Method::PATCH, "/api/new-feature"),
            Some(Permission::ConfigEdit)
        );
    }
}
