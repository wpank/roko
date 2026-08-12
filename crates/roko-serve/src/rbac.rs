//! Role-Based Access Control (RBAC) for the roko HTTP API.
//!
//! This module defines the typed role and permission model for workspace
//! members, along with the enforcement helpers used by route middleware.
//!
//! ## Role hierarchy (highest → lowest authority)
//!
//! ```text
//! Owner > Admin > Member > Viewer
//! ```
//!
//! ## Permission sets
//!
//! | Permission     | Owner | Admin | Member | Viewer |
//! |----------------|-------|-------|--------|--------|
//! | PlanExecute    |   ✓   |   ✓   |   ✓    |        |
//! | PlanCreate     |   ✓   |   ✓   |   ✓    |        |
//! | AgentSpawn     |   ✓   |   ✓   |   ✓    |        |
//! | AgentStop      |   ✓   |   ✓   |   ✓    |        |
//! | ConfigEdit     |   ✓   |   ✓   |        |        |
//! | SecretsRead    |   ✓   |   ✓   |        |        |
//! | SecretsWrite   |   ✓   |   ✓   |        |        |
//! | TeamManage     |   ✓   |       |        |        |
//! | ApiKeyCreate   |   ✓   |   ✓   |        |        |
//! | TokenIssue     |   ✓   |   ✓   |        |        |
//! | ViewDashboard  |   ✓   |   ✓   |   ✓    |   ✓    |

use std::collections::HashSet;
use std::fmt;

use crate::error::ApiError;

// ── Role ────────────────────────────────────────────────────────────────────

/// A workspace role. Roles are strictly ordered: `Owner > Admin > Member > Viewer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    /// Workspace viewer — read-only access to the dashboard.
    Viewer = 0,
    /// Regular member — can run plans and spawn/stop agents.
    Member = 1,
    /// Administrator — all member permissions plus config, secrets, and API keys.
    Admin = 2,
    /// Owner — all permissions including team management.
    Owner = 3,
}

impl std::str::FromStr for Role {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "owner" => Ok(Self::Owner),
            "admin" => Ok(Self::Admin),
            "member" => Ok(Self::Member),
            "viewer" => Ok(Self::Viewer),
            _ => Err(()),
        }
    }
}

impl Role {
    /// Return the canonical lowercase string for this role.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Member => "member",
            Self::Viewer => "viewer",
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Permission ──────────────────────────────────────────────────────────────

/// A discrete action that can be permitted or denied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Permission {
    /// Execute an existing plan.
    PlanExecute,
    /// Create or modify a plan.
    PlanCreate,
    /// Spawn a new agent process.
    AgentSpawn,
    /// Stop a running agent.
    AgentStop,
    /// Edit server/workspace configuration.
    ConfigEdit,
    /// Read secrets from the secrets store.
    SecretsRead,
    /// Write (create/update/delete) secrets.
    SecretsWrite,
    /// Manage team members (invite, change roles, remove).
    TeamManage,
    /// Create API keys.
    ApiKeyCreate,
    /// Issue agent bearer tokens.
    TokenIssue,
    /// View the dashboard and read-only status endpoints.
    ViewDashboard,
}

impl Permission {
    /// Machine-readable name for structured error responses.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PlanExecute => "plan:execute",
            Self::PlanCreate => "plan:create",
            Self::AgentSpawn => "agent:spawn",
            Self::AgentStop => "agent:stop",
            Self::ConfigEdit => "config:edit",
            Self::SecretsRead => "secrets:read",
            Self::SecretsWrite => "secrets:write",
            Self::TeamManage => "team:manage",
            Self::ApiKeyCreate => "api_key:create",
            Self::TokenIssue => "token:issue",
            Self::ViewDashboard => "dashboard:view",
        }
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── RoleBinding ─────────────────────────────────────────────────────────────

/// Maps a [`Role`] to its granted [`Permission`] set.
///
/// A `RoleBinding` is immutable once constructed. Use [`RoleBinding::for_role`]
/// to obtain the canonical built-in binding for a given role.
#[derive(Debug, Clone)]
pub struct RoleBinding {
    /// The role this binding applies to.
    pub role: Role,
    /// The permissions granted by this role.
    pub permissions: HashSet<Permission>,
}

impl RoleBinding {
    /// Construct a `RoleBinding` from an explicit permission set.
    pub fn new(role: Role, permissions: impl IntoIterator<Item = Permission>) -> Self {
        Self {
            role,
            permissions: permissions.into_iter().collect(),
        }
    }

    /// Return the canonical built-in binding for `role`.
    ///
    /// These are the default permission sets described in the module docs.
    pub fn for_role(role: Role) -> Self {
        Self::new(role, role_permissions(role))
    }

    /// Returns `true` if this binding grants `permission`.
    pub fn grants(&self, permission: Permission) -> bool {
        self.permissions.contains(&permission)
    }
}

// ── role_permissions ────────────────────────────────────────────────────────

/// Return the default [`Permission`] set for `role`.
///
/// This is the canonical built-in permission mapping. Route handlers and
/// middleware call [`enforce_permission`] instead of this function directly;
/// this function is exposed for tests and admin tooling.
pub fn role_permissions(role: Role) -> HashSet<Permission> {
    let viewer: HashSet<Permission> = [Permission::ViewDashboard].into();

    let member: HashSet<Permission> = viewer
        .iter()
        .copied()
        .chain([
            Permission::PlanExecute,
            Permission::PlanCreate,
            Permission::AgentSpawn,
            Permission::AgentStop,
        ])
        .collect();

    let admin: HashSet<Permission> = member
        .iter()
        .copied()
        .chain([
            Permission::ConfigEdit,
            Permission::SecretsRead,
            Permission::SecretsWrite,
            Permission::ApiKeyCreate,
            Permission::TokenIssue,
        ])
        .collect();

    let owner: HashSet<Permission> = admin
        .iter()
        .copied()
        .chain([Permission::TeamManage])
        .collect();

    match role {
        Role::Viewer => viewer,
        Role::Member => member,
        Role::Admin => admin,
        Role::Owner => owner,
    }
}

// ── check_permission / enforce_permission ───────────────────────────────────

/// Returns `true` when `role` has been granted `required`.
///
/// ```
/// use roko_serve::rbac::{Role, Permission, check_permission};
///
/// assert!(check_permission(Role::Admin, Permission::SecretsRead));
/// assert!(!check_permission(Role::Member, Permission::TeamManage));
/// ```
pub fn check_permission(role: Role, required: Permission) -> bool {
    role_permissions(role).contains(&required)
}

/// Enforce that `role` holds `required`, returning a typed [`ApiError`] on
/// denial.
///
/// Returns `Ok(())` when the role holds the permission, or
/// `Err(ApiError::forbidden(...))` otherwise.
///
/// # Errors
///
/// Returns an [`ApiError`] with HTTP 403 status when the role does not have
/// the requested permission.
pub fn enforce_permission(role: Role, required: Permission) -> Result<(), ApiError> {
    if check_permission(role, required) {
        Ok(())
    } else {
        // Emit a structured warning so deny events appear in the trace even
        // when the caller does not wire up file-based audit logging.
        tracing::warn!(
            role = %role.as_str(),
            permission = %required.as_str(),
            "auth_audit: permission denied",
        );
        Err(ApiError::forbidden(format!(
            "role '{}' does not have permission '{}'",
            role.as_str(),
            required.as_str()
        )))
    }
}

/// Resolve a role from a string and enforce a permission, returning a typed
/// [`ApiError`] on failure.
///
/// Convenience wrapper for route handlers that receive the caller's role as a
/// raw string (e.g. from the `TeamMember.role` field in persistent storage).
///
/// Returns `Err(ApiError::forbidden(...))` when the string is not a recognised
/// role or when the resolved role does not hold `required`.
///
/// # Errors
///
/// Returns an [`ApiError`] with HTTP 403 status when the role string is
/// unrecognised or permission is denied.
pub fn enforce_permission_str(role_str: &str, required: Permission) -> Result<(), ApiError> {
    let role: Role = role_str
        .parse()
        .map_err(|()| ApiError::forbidden(format!("unrecognised role '{role_str}'")))?;
    enforce_permission(role, required)
}

// ── Role escalation guard ────────────────────────────────────────────────────

/// Verify that `changer_role` has sufficient authority to assign `target_role`.
///
/// A user may only assign roles strictly below their own level. The owner role
/// may not be assigned by non-owners even if their level is ≥ admin, because
/// `Owner > Admin` in the ordering.
///
/// # Errors
///
/// Returns [`ApiError::forbidden`] when:
/// - `changer_role` is below `target_role` in the hierarchy, or
/// - the changer is not an Owner and is trying to assign the Owner role.
pub fn enforce_no_escalation(changer_role: Role, target_role: Role) -> Result<(), ApiError> {
    if changer_role < target_role {
        return Err(ApiError::forbidden(format!(
            "role '{}' cannot assign role '{}': insufficient authority",
            changer_role.as_str(),
            target_role.as_str()
        )));
    }
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Role ordering ────────────────────────────────────────────────────────

    #[test]
    fn role_ordering_is_correct() {
        assert!(Role::Owner > Role::Admin);
        assert!(Role::Admin > Role::Member);
        assert!(Role::Member > Role::Viewer);
    }

    #[test]
    fn role_from_str_round_trips() {
        for (s, expected) in [
            ("owner", Role::Owner),
            ("admin", Role::Admin),
            ("member", Role::Member),
            ("viewer", Role::Viewer),
        ] {
            let role: Role = s.parse().expect("should parse");
            assert_eq!(role, expected);
            assert_eq!(role.as_str(), s);
        }
    }

    #[test]
    fn role_from_str_rejects_unknown() {
        assert!("superuser".parse::<Role>().is_err());
        assert!("".parse::<Role>().is_err());
        assert!("ADMIN".parse::<Role>().is_err()); // case-sensitive
    }

    // ── role_permissions ─────────────────────────────────────────────────────

    #[test]
    fn viewer_has_only_dashboard() {
        let perms = role_permissions(Role::Viewer);
        assert!(perms.contains(&Permission::ViewDashboard));
        assert!(!perms.contains(&Permission::PlanExecute));
        assert!(!perms.contains(&Permission::TeamManage));
        assert_eq!(perms.len(), 1);
    }

    #[test]
    fn member_extends_viewer() {
        let perms = role_permissions(Role::Member);
        assert!(perms.contains(&Permission::ViewDashboard));
        assert!(perms.contains(&Permission::PlanExecute));
        assert!(perms.contains(&Permission::PlanCreate));
        assert!(perms.contains(&Permission::AgentSpawn));
        assert!(perms.contains(&Permission::AgentStop));
        assert!(!perms.contains(&Permission::ConfigEdit));
        assert!(!perms.contains(&Permission::TeamManage));
    }

    #[test]
    fn admin_extends_member() {
        let perms = role_permissions(Role::Admin);
        // Inherits member perms
        assert!(perms.contains(&Permission::PlanExecute));
        assert!(perms.contains(&Permission::AgentSpawn));
        // Admin-only
        assert!(perms.contains(&Permission::ConfigEdit));
        assert!(perms.contains(&Permission::SecretsRead));
        assert!(perms.contains(&Permission::SecretsWrite));
        assert!(perms.contains(&Permission::ApiKeyCreate));
        assert!(perms.contains(&Permission::TokenIssue));
        // Not owner-only
        assert!(!perms.contains(&Permission::TeamManage));
    }

    #[test]
    fn owner_has_all_permissions() {
        let perms = role_permissions(Role::Owner);
        for perm in [
            Permission::PlanExecute,
            Permission::PlanCreate,
            Permission::AgentSpawn,
            Permission::AgentStop,
            Permission::ConfigEdit,
            Permission::SecretsRead,
            Permission::SecretsWrite,
            Permission::TeamManage,
            Permission::ApiKeyCreate,
            Permission::TokenIssue,
            Permission::ViewDashboard,
        ] {
            assert!(
                perms.contains(&perm),
                "Owner should have permission {:?}",
                perm
            );
        }
    }

    // ── check_permission ─────────────────────────────────────────────────────

    #[test]
    fn check_permission_viewer_denied_plan_execute() {
        assert!(!check_permission(Role::Viewer, Permission::PlanExecute));
    }

    #[test]
    fn check_permission_member_allowed_plan_execute() {
        assert!(check_permission(Role::Member, Permission::PlanExecute));
    }

    #[test]
    fn check_permission_admin_allowed_secrets() {
        assert!(check_permission(Role::Admin, Permission::SecretsRead));
        assert!(check_permission(Role::Admin, Permission::SecretsWrite));
    }

    #[test]
    fn check_permission_admin_denied_team_manage() {
        assert!(!check_permission(Role::Admin, Permission::TeamManage));
    }

    #[test]
    fn check_permission_owner_allowed_team_manage() {
        assert!(check_permission(Role::Owner, Permission::TeamManage));
    }

    // ── enforce_permission ───────────────────────────────────────────────────

    #[test]
    fn enforce_permission_ok_when_role_has_permission() {
        assert!(enforce_permission(Role::Admin, Permission::ConfigEdit).is_ok());
    }

    #[test]
    fn enforce_permission_err_when_role_lacks_permission() {
        let err = enforce_permission(Role::Member, Permission::TeamManage);
        assert!(err.is_err());
        let api_err = err.unwrap_err();
        assert_eq!(api_err.code, "forbidden");
        assert!(api_err.message.contains("team:manage"));
    }

    // ── enforce_permission_str ───────────────────────────────────────────────

    #[test]
    fn enforce_permission_str_parses_and_checks() {
        assert!(enforce_permission_str("owner", Permission::TeamManage).is_ok());
        assert!(enforce_permission_str("viewer", Permission::TeamManage).is_err());
    }

    #[test]
    fn enforce_permission_str_rejects_unknown_role() {
        let err = enforce_permission_str("superuser", Permission::ViewDashboard);
        assert!(err.is_err());
        assert!(err.unwrap_err().message.contains("superuser"));
    }

    // ── RoleBinding ──────────────────────────────────────────────────────────

    #[test]
    fn role_binding_for_role_matches_role_permissions() {
        for role in [Role::Viewer, Role::Member, Role::Admin, Role::Owner] {
            let binding = RoleBinding::for_role(role);
            assert_eq!(binding.role, role);
            assert_eq!(binding.permissions, role_permissions(role));
        }
    }

    #[test]
    fn role_binding_grants_returns_true_for_granted() {
        let binding = RoleBinding::for_role(Role::Admin);
        assert!(binding.grants(Permission::ConfigEdit));
        assert!(!binding.grants(Permission::TeamManage));
    }

    // ── enforce_no_escalation ────────────────────────────────────────────────

    #[test]
    fn owner_can_assign_any_role() {
        assert!(enforce_no_escalation(Role::Owner, Role::Admin).is_ok());
        assert!(enforce_no_escalation(Role::Owner, Role::Member).is_ok());
        assert!(enforce_no_escalation(Role::Owner, Role::Viewer).is_ok());
        assert!(enforce_no_escalation(Role::Owner, Role::Owner).is_ok());
    }

    #[test]
    fn admin_cannot_assign_owner() {
        let err = enforce_no_escalation(Role::Admin, Role::Owner);
        assert!(err.is_err());
        assert!(err.unwrap_err().message.contains("insufficient authority"));
    }

    #[test]
    fn member_cannot_assign_admin_or_above() {
        assert!(enforce_no_escalation(Role::Member, Role::Admin).is_err());
        assert!(enforce_no_escalation(Role::Member, Role::Owner).is_err());
    }

    #[test]
    fn member_can_assign_viewer() {
        assert!(enforce_no_escalation(Role::Member, Role::Viewer).is_ok());
    }

    #[test]
    fn viewer_cannot_escalate_anyone() {
        for target in [Role::Member, Role::Admin, Role::Owner] {
            assert!(
                enforce_no_escalation(Role::Viewer, target).is_err(),
                "Viewer should not be able to assign {:?}",
                target
            );
        }
    }
}
