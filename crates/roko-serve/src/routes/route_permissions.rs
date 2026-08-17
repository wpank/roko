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
        prefix: "/api/prd",
        permission: Permission::PlanCreate,
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
    RoutePermission {
        prefix: "/relay",
        permission: Permission::AgentSpawn,
    },
];

/// Resolve the typed permission required for a request.
///
/// Read-only requests normally rely on authentication plus their route's
/// existing scope. Audit and secret reads are intentionally stronger. Every
/// mutation receives a typed permission: unmatched mutations fail closed to
/// [`Permission::ConfigEdit`] rather than bypassing RBAC.
pub(crate) fn required_permission_for(method: &Method, path: &str) -> Option<Permission> {
    // Axum strips the `/api` nest prefix before invoking middleware attached
    // to the nested router. Accept both that runtime form and the canonical
    // externally visible path used by tests, logs, and documentation.
    let nested_path;
    let path = if path.starts_with("/api/") {
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

    if matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS) {
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
    fn unclassified_mutations_fail_closed() {
        assert_eq!(
            required_permission_for(&Method::PATCH, "/api/new-feature"),
            Some(Permission::ConfigEdit)
        );
    }
}
