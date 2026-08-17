//! Restart-durable runtime owner for group membership and pheromone state.
//!
//! Mutations are serialized through one lock and persisted before becoming
//! visible. This deliberately favors correctness over write throughput: group
//! administration is low-volume, while preventing duplicate invitations,
//! capacity overbooking, and lost updates is critical.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use chrono::{DateTime, Duration, Utc};
use roko_core::config::schema::GroupDefinition;
use roko_core::groups::{
    AssignmentStrategy, CoordinationMode, Group, GroupConfig, GroupEvent, GroupId, GroupInvitation,
    GroupMember, GroupPheromone, InvitationId, InvitationStatus, InviteRequest, InviteResponse,
    KnowledgePolicy, MemberPermissions, MemberRole,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;

const GROUP_STATE_VERSION: u32 = 1;
const MAX_GROUP_EVENTS: usize = 4_096;
const MAX_GROUPS: usize = 10_000;
const MAX_MEMBERS: usize = 10_000;
const INVITATION_LIFETIME_HOURS: i64 = 24;
const BASE_PHEROMONE_DECAY_PER_DAY: f64 = 0.01;

/// Runtime failure with a stable API classification.
#[derive(Debug)]
pub enum GroupRuntimeError {
    NotFound(String),
    Forbidden(String),
    Conflict(String),
    Invalid(String),
    Storage(String),
}

impl fmt::Display for GroupRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(message)
            | Self::Forbidden(message)
            | Self::Conflict(message)
            | Self::Invalid(message)
            | Self::Storage(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for GroupRuntimeError {}

/// Input for creating a group.
#[derive(Debug, Clone)]
pub struct CreateGroupInput {
    pub name: String,
    pub description: String,
    pub coordination: CoordinationMode,
    pub config: GroupConfig,
}

/// Partial owner-only update.
#[derive(Debug, Clone, Default)]
pub struct UpdateGroupInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub coordination: Option<CoordinationMode>,
    pub config: Option<GroupConfig>,
}

/// Partial owner-only membership update.
#[derive(Debug, Clone, Default)]
pub struct UpdateMemberInput {
    pub role: Option<MemberRole>,
    pub permissions: Option<MemberPermissions>,
}

/// A stored pheromone plus its derived current balance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PheromoneView {
    pub id: String,
    #[serde(flatten)]
    pub pheromone: GroupPheromone,
    pub balance: f64,
    pub last_touched_at: DateTime<Utc>,
}

/// Durable group event envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupEventRecord {
    pub seq: u64,
    pub group_id: GroupId,
    pub room: String,
    pub event: GroupEvent,
    pub occurred_at: DateTime<Utc>,
}

/// Mutation output committed atomically with its event.
#[derive(Debug, Clone)]
pub struct GroupMutation<T> {
    pub value: T,
    pub event: GroupEventRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredPheromone {
    id: String,
    pheromone: GroupPheromone,
    balance: f64,
    last_touched_at: DateTime<Utc>,
}

impl StoredPheromone {
    fn balance_at(&self, decay_modifier: f64, now: DateTime<Utc>) -> f64 {
        let elapsed_hours =
            (now - self.last_touched_at).num_milliseconds().max(0) as f64 / 3_600_000.0;
        let daily_rate = (BASE_PHEROMONE_DECAY_PER_DAY * decay_modifier).clamp(0.0, 1.0);
        let retention = (1.0 - daily_rate).powf(elapsed_hours / 24.0);
        (self.balance * retention).clamp(0.0, 1.0)
    }

    fn view(&self, decay_modifier: f64, now: DateTime<Utc>) -> PheromoneView {
        PheromoneView {
            id: self.id.clone(),
            pheromone: self.pheromone.clone(),
            balance: self.balance_at(decay_modifier, now),
            last_touched_at: self.last_touched_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedGroupState {
    version: u32,
    #[serde(default)]
    groups: BTreeMap<GroupId, Group>,
    #[serde(default)]
    invitations: BTreeMap<InvitationId, GroupInvitation>,
    #[serde(default)]
    pheromones: BTreeMap<GroupId, Vec<StoredPheromone>>,
    #[serde(default)]
    events: Vec<GroupEventRecord>,
    #[serde(default)]
    next_event_seq: u64,
}

impl Default for PersistedGroupState {
    fn default() -> Self {
        Self {
            version: GROUP_STATE_VERSION,
            groups: BTreeMap::new(),
            invitations: BTreeMap::new(),
            pheromones: BTreeMap::new(),
            events: Vec::new(),
            next_event_seq: 0,
        }
    }
}

/// Transactional runtime and durable store for all group state.
pub struct GroupRuntime {
    path: PathBuf,
    state: Mutex<PersistedGroupState>,
}

impl GroupRuntime {
    /// Open persisted state and reconcile declarative `[[groups]]` entries.
    pub fn open(
        workdir: &Path,
        definitions: &[GroupDefinition],
    ) -> Result<Self, GroupRuntimeError> {
        let path = workdir.join(".roko").join("groups").join("state.json");
        let mut state = load_state(&path)?;
        let changed = reconcile_definitions(&mut state, definitions)?;
        if changed {
            persist_state_sync(&path, &state)?;
        }
        Ok(Self {
            path,
            state: Mutex::new(state),
        })
    }

    /// List groups visible to an authenticated user.
    pub async fn list(&self, actor: &str) -> Vec<Group> {
        self.state
            .lock()
            .await
            .groups
            .values()
            .filter(|group| can_view(group, actor))
            .cloned()
            .collect()
    }

    /// Read one group if the caller may see it.
    pub async fn get(&self, group_id: &GroupId, actor: &str) -> Result<Group, GroupRuntimeError> {
        let state = self.state.lock().await;
        let group = state
            .groups
            .get(group_id)
            .ok_or_else(|| GroupRuntimeError::NotFound(format!("group '{group_id}' not found")))?;
        require_view(group, actor)?;
        Ok(group.clone())
    }

    /// Create and durably commit a group.
    pub async fn create(
        &self,
        owner: &str,
        input: CreateGroupInput,
    ) -> Result<GroupMutation<Group>, GroupRuntimeError> {
        validate_actor(owner)?;
        validate_group_input(&input.name, &input.config)?;
        let mut guard = self.state.lock().await;
        if guard.groups.len() >= MAX_GROUPS {
            return Err(GroupRuntimeError::Conflict(format!(
                "group limit of {MAX_GROUPS} reached"
            )));
        }
        if group_name_exists(&guard, owner, &input.name, None) {
            return Err(GroupRuntimeError::Conflict(format!(
                "group name '{}' already exists for owner '{owner}'",
                input.name.trim()
            )));
        }

        let now = Utc::now();
        let group = Group {
            id: GroupId::new(format!("grp-{}", Uuid::new_v4().simple())),
            name: input.name.trim().to_string(),
            description: input.description.trim().to_string(),
            owner: owner.to_string(),
            members: Vec::new(),
            coordination: input.coordination,
            config: input.config,
            created_at: now,
            updated_at: now,
        };
        let event = GroupEvent::Created {
            group_id: group.id.clone(),
            name: group.name.clone(),
            owner: group.owner.clone(),
        };
        let group_id = group.id.clone();
        let mut next = guard.clone();
        next.groups.insert(group_id.clone(), group.clone());
        let event = append_event(&mut next, &group_id, event);
        self.commit(&mut guard, next).await?;
        Ok(GroupMutation {
            value: group,
            event,
        })
    }

    /// Apply an owner-only group update.
    pub async fn update(
        &self,
        group_id: &GroupId,
        actor: &str,
        input: UpdateGroupInput,
    ) -> Result<GroupMutation<Group>, GroupRuntimeError> {
        if input.name.is_none()
            && input.description.is_none()
            && input.coordination.is_none()
            && input.config.is_none()
        {
            return Err(GroupRuntimeError::Invalid(
                "group update must change at least one field".to_string(),
            ));
        }
        let mut guard = self.state.lock().await;
        let current = require_group(&guard, group_id)?.clone();
        require_owner(&current, actor)?;
        let next_name = input.name.as_deref().unwrap_or(&current.name);
        let next_config = input.config.as_ref().unwrap_or(&current.config);
        validate_group_input(next_name, next_config)?;
        if group_name_exists(&guard, actor, next_name, Some(group_id)) {
            return Err(GroupRuntimeError::Conflict(format!(
                "group name '{}' already exists for owner '{actor}'",
                next_name.trim()
            )));
        }
        if next_config
            .max_members
            .is_some_and(|maximum| current.members.len() > maximum)
        {
            return Err(GroupRuntimeError::Conflict(
                "max_members cannot be lower than the current member count".to_string(),
            ));
        }

        let mut updated = current;
        let mut changes = serde_json::Map::new();
        if let Some(name) = input.name {
            updated.name = name.trim().to_string();
            changes.insert("name".into(), serde_json::json!(updated.name));
        }
        if let Some(description) = input.description {
            updated.description = description.trim().to_string();
            changes.insert("description".into(), serde_json::json!(updated.description));
        }
        if let Some(coordination) = input.coordination {
            updated.coordination = coordination;
            changes.insert("coordination".into(), serde_json::json!(coordination));
        }
        if let Some(config) = input.config {
            updated.config = config;
            changes.insert("config".into(), serde_json::json!(updated.config));
        }
        updated.updated_at = Utc::now();

        let event = GroupEvent::Updated {
            group_id: group_id.clone(),
            changes: serde_json::Value::Object(changes),
        };
        let mut next = guard.clone();
        next.groups.insert(group_id.clone(), updated.clone());
        let event = append_event(&mut next, group_id, event);
        self.commit(&mut guard, next).await?;
        Ok(GroupMutation {
            value: updated,
            event,
        })
    }

    /// Delete a group and all group-scoped durable runtime state.
    pub async fn delete(
        &self,
        group_id: &GroupId,
        actor: &str,
    ) -> Result<GroupMutation<Group>, GroupRuntimeError> {
        let mut guard = self.state.lock().await;
        let group = require_group(&guard, group_id)?.clone();
        require_owner(&group, actor)?;
        let mut next = guard.clone();
        next.groups.remove(group_id);
        next.pheromones.remove(group_id);
        next.invitations
            .retain(|_, invitation| invitation.group_id != *group_id);
        let event = append_event(
            &mut next,
            group_id,
            GroupEvent::Deleted {
                group_id: group_id.clone(),
                owner: actor.to_string(),
            },
        );
        self.commit(&mut guard, next).await?;
        Ok(GroupMutation {
            value: group,
            event,
        })
    }

    /// Invite an agent, auto-joining only same-owner or auto-accepted agents.
    pub async fn invite(
        &self,
        group_id: &GroupId,
        actor: &str,
        agent_owner: &str,
        request: InviteRequest,
    ) -> Result<GroupMutation<InviteResponse>, GroupRuntimeError> {
        validate_actor(agent_owner)?;
        validate_agent_id(&request.agent_id)?;
        validate_member_permissions(request.role, request.permissions)?;
        let mut guard = self.state.lock().await;
        let group = require_group(&guard, group_id)?.clone();
        require_owner(&group, actor)?;
        if group.member(&request.agent_id).is_some() {
            return Err(GroupRuntimeError::Conflict(format!(
                "agent '{}' is already a member",
                request.agent_id
            )));
        }
        expire_invitations(&mut guard, Utc::now());
        if guard.invitations.values().any(|invitation| {
            invitation.group_id == *group_id
                && invitation.agent_id == request.agent_id
                && invitation.status == InvitationStatus::Pending
        }) {
            return Err(GroupRuntimeError::Conflict(format!(
                "agent '{}' already has a pending invitation",
                request.agent_id
            )));
        }
        ensure_capacity(&guard, &group)?;

        let immediate = group.owner == agent_owner || group.config.auto_accept;
        let now = Utc::now();
        let mut next = guard.clone();
        let (response, event) = if immediate {
            let member = GroupMember {
                agent_id: request.agent_id.clone(),
                owner: agent_owner.to_string(),
                role: request.role,
                permissions: request.permissions,
                joined_at: now,
            };
            let target = next.groups.get_mut(group_id).ok_or_else(|| {
                GroupRuntimeError::NotFound(format!("group '{group_id}' not found"))
            })?;
            target.members.push(member);
            target.updated_at = now;
            (
                InviteResponse {
                    status: InvitationStatus::Accepted,
                    invitation_id: None,
                    agent_id: request.agent_id.clone(),
                    group_id: group_id.clone(),
                },
                GroupEvent::MemberJoined {
                    agent_id: request.agent_id,
                    owner: agent_owner.to_string(),
                    role: request.role,
                },
            )
        } else {
            let invitation_id = InvitationId::new(format!("inv-{}", Uuid::new_v4().simple()));
            let invitation = GroupInvitation {
                id: invitation_id.clone(),
                group_id: group_id.clone(),
                agent_id: request.agent_id.clone(),
                invited_by: actor.to_string(),
                agent_owner: agent_owner.to_string(),
                role: request.role,
                permissions: request.permissions,
                status: InvitationStatus::Pending,
                created_at: now,
                expires_at: now + Duration::hours(INVITATION_LIFETIME_HOURS),
            };
            next.invitations.insert(invitation_id.clone(), invitation);
            (
                InviteResponse {
                    status: InvitationStatus::Pending,
                    invitation_id: Some(invitation_id),
                    agent_id: request.agent_id.clone(),
                    group_id: group_id.clone(),
                },
                GroupEvent::MemberInvited {
                    agent_id: request.agent_id,
                    invited_by: actor.to_string(),
                    role: request.role,
                },
            )
        };
        let event = append_event(&mut next, group_id, event);
        self.commit(&mut guard, next).await?;
        Ok(GroupMutation {
            value: response,
            event,
        })
    }

    /// List invitations visible to the group owner or invited agent owner.
    pub async fn invitations(
        &self,
        group_id: &GroupId,
        actor: &str,
    ) -> Result<Vec<GroupInvitation>, GroupRuntimeError> {
        let mut guard = self.state.lock().await;
        let group = require_group(&guard, group_id)?.clone();
        if group.owner != actor
            && !guard.invitations.values().any(|invitation| {
                invitation.group_id == *group_id && invitation.agent_owner == actor
            })
        {
            return Err(GroupRuntimeError::Forbidden(
                "caller cannot view these invitations".to_string(),
            ));
        }
        let mut next = guard.clone();
        let expired = expire_invitations(&mut next, Utc::now());
        let invitations = next
            .invitations
            .values()
            .filter(|invitation| invitation.group_id == *group_id)
            .cloned()
            .collect();
        if expired {
            self.commit(&mut guard, next).await?;
        }
        Ok(invitations)
    }

    /// Accept a pending cross-owner invitation.
    pub async fn accept_invitation(
        &self,
        invitation_id: &InvitationId,
        actor: &str,
    ) -> Result<GroupMutation<InviteResponse>, GroupRuntimeError> {
        self.decide_invitation(invitation_id, actor, true).await
    }

    /// Reject a pending cross-owner invitation.
    pub async fn reject_invitation(
        &self,
        invitation_id: &InvitationId,
        actor: &str,
    ) -> Result<GroupMutation<InviteResponse>, GroupRuntimeError> {
        self.decide_invitation(invitation_id, actor, false).await
    }

    async fn decide_invitation(
        &self,
        invitation_id: &InvitationId,
        actor: &str,
        accept: bool,
    ) -> Result<GroupMutation<InviteResponse>, GroupRuntimeError> {
        let mut guard = self.state.lock().await;
        expire_invitations(&mut guard, Utc::now());
        let invitation = guard
            .invitations
            .get(invitation_id)
            .cloned()
            .ok_or_else(|| {
                GroupRuntimeError::NotFound(format!("invitation '{invitation_id}' not found"))
            })?;
        if invitation.agent_owner != actor {
            return Err(GroupRuntimeError::Forbidden(
                "only the invited agent owner can decide this invitation".to_string(),
            ));
        }
        if invitation.status != InvitationStatus::Pending {
            return Err(GroupRuntimeError::Conflict(format!(
                "invitation is already {:?}",
                invitation.status
            )));
        }

        let group = require_group(&guard, &invitation.group_id)?.clone();
        if accept {
            ensure_member_slot(&group)?;
        }
        let now = Utc::now();
        let mut next = guard.clone();
        let stored = next.invitations.get_mut(invitation_id).ok_or_else(|| {
            GroupRuntimeError::NotFound(format!("invitation '{invitation_id}' not found"))
        })?;
        stored.status = if accept {
            InvitationStatus::Accepted
        } else {
            InvitationStatus::Rejected
        };
        let event = if accept {
            let target = next.groups.get_mut(&invitation.group_id).ok_or_else(|| {
                GroupRuntimeError::NotFound(format!("group '{}' not found", invitation.group_id))
            })?;
            if target.member(&invitation.agent_id).is_none() {
                target.members.push(GroupMember {
                    agent_id: invitation.agent_id.clone(),
                    owner: invitation.agent_owner.clone(),
                    role: invitation.role,
                    permissions: invitation.permissions,
                    joined_at: now,
                });
                target.updated_at = now;
            }
            GroupEvent::MemberJoined {
                agent_id: invitation.agent_id.clone(),
                owner: invitation.agent_owner.clone(),
                role: invitation.role,
            }
        } else {
            GroupEvent::MemberLeft {
                agent_id: invitation.agent_id.clone(),
                reason: "invitation_rejected".to_string(),
            }
        };
        let event = append_event(&mut next, &invitation.group_id, event);
        let response = InviteResponse {
            status: if accept {
                InvitationStatus::Accepted
            } else {
                InvitationStatus::Rejected
            },
            invitation_id: Some(invitation_id.clone()),
            agent_id: invitation.agent_id,
            group_id: invitation.group_id,
        };
        self.commit(&mut guard, next).await?;
        Ok(GroupMutation {
            value: response,
            event,
        })
    }

    /// Update a member role or permission set.
    pub async fn update_member(
        &self,
        group_id: &GroupId,
        agent_id: &str,
        actor: &str,
        input: UpdateMemberInput,
    ) -> Result<GroupMutation<GroupMember>, GroupRuntimeError> {
        if input.role.is_none() && input.permissions.is_none() {
            return Err(GroupRuntimeError::Invalid(
                "member update must change role or permissions".to_string(),
            ));
        }
        let mut guard = self.state.lock().await;
        let group = require_group(&guard, group_id)?.clone();
        require_owner(&group, actor)?;
        let member = group
            .member(agent_id)
            .cloned()
            .ok_or_else(|| GroupRuntimeError::NotFound(format!("member '{agent_id}' not found")))?;
        let role = input.role.unwrap_or(member.role);
        let permissions = input.permissions.unwrap_or(member.permissions);
        validate_member_permissions(role, permissions)?;
        let mut updated = member;
        updated.role = role;
        updated.permissions = permissions;
        let mut next = guard.clone();
        let target = next
            .groups
            .get_mut(group_id)
            .ok_or_else(|| GroupRuntimeError::NotFound(format!("group '{group_id}' not found")))?;
        let slot = target
            .members
            .iter_mut()
            .find(|candidate| candidate.agent_id == agent_id)
            .ok_or_else(|| GroupRuntimeError::NotFound(format!("member '{agent_id}' not found")))?;
        *slot = updated.clone();
        target.updated_at = Utc::now();
        let event = append_event(
            &mut next,
            group_id,
            GroupEvent::MemberUpdated {
                agent_id: agent_id.to_string(),
                changes: serde_json::json!({ "role": role, "permissions": permissions }),
            },
        );
        self.commit(&mut guard, next).await?;
        Ok(GroupMutation {
            value: updated,
            event,
        })
    }

    /// Remove a member as either the group owner or that agent's owner.
    pub async fn remove_member(
        &self,
        group_id: &GroupId,
        agent_id: &str,
        actor: &str,
    ) -> Result<GroupMutation<GroupMember>, GroupRuntimeError> {
        let mut guard = self.state.lock().await;
        let group = require_group(&guard, group_id)?.clone();
        let member = group
            .member(agent_id)
            .cloned()
            .ok_or_else(|| GroupRuntimeError::NotFound(format!("member '{agent_id}' not found")))?;
        if group.owner != actor && member.owner != actor {
            return Err(GroupRuntimeError::Forbidden(
                "only the group owner or agent owner can remove this member".to_string(),
            ));
        }
        let mut next = guard.clone();
        let target = next
            .groups
            .get_mut(group_id)
            .ok_or_else(|| GroupRuntimeError::NotFound(format!("group '{group_id}' not found")))?;
        target
            .members
            .retain(|candidate| candidate.agent_id != agent_id);
        target.updated_at = Utc::now();
        let event = append_event(
            &mut next,
            group_id,
            GroupEvent::MemberLeft {
                agent_id: agent_id.to_string(),
                reason: if group.owner == actor {
                    "removed_by_group_owner".to_string()
                } else {
                    "removed_by_agent_owner".to_string()
                },
            },
        );
        self.commit(&mut guard, next).await?;
        Ok(GroupMutation {
            value: member,
            event,
        })
    }

    /// Deposit or refresh a member-owned pheromone.
    pub async fn deposit_pheromone(
        &self,
        group_id: &GroupId,
        actor: &str,
        depositor: &str,
        signal_type: &str,
        position_hint: Option<String>,
        metadata: serde_json::Value,
    ) -> Result<GroupMutation<PheromoneView>, GroupRuntimeError> {
        validate_signal_type(signal_type)?;
        if !metadata.is_object() && !metadata.is_null() {
            return Err(GroupRuntimeError::Invalid(
                "pheromone metadata must be a JSON object".to_string(),
            ));
        }
        let mut guard = self.state.lock().await;
        let group = require_group(&guard, group_id)?.clone();
        let member = group.member(depositor).ok_or_else(|| {
            GroupRuntimeError::Forbidden("depositor is not a group member".to_string())
        })?;
        if member.owner != actor && group.owner != actor {
            return Err(GroupRuntimeError::Forbidden(
                "caller does not own the depositing agent".to_string(),
            ));
        }
        if !member.permissions.write || member.role == MemberRole::Observer {
            return Err(GroupRuntimeError::Forbidden(
                "member lacks group write permission".to_string(),
            ));
        }

        let now = Utc::now();
        let mut next = guard.clone();
        let field = next.pheromones.entry(group_id.clone()).or_default();
        let stored = if let Some(existing) = field.iter_mut().find(|candidate| {
            candidate.pheromone.depositor == depositor
                && candidate.pheromone.signal_type == signal_type.trim()
        }) {
            existing.balance = 1.0;
            existing.last_touched_at = now;
            existing.pheromone.metadata = metadata;
            existing.pheromone.position_hint = position_hint;
            existing.pheromone.deposited_at = now;
            existing.clone()
        } else {
            let pheromone = GroupPheromone {
                group_id: group_id.clone(),
                depositor: depositor.to_string(),
                signal_type: signal_type.trim().to_string(),
                position_hint,
                metadata,
                deposited_at: now,
            };
            let stored = StoredPheromone {
                id: format!("phr-{}", Uuid::new_v4().simple()),
                pheromone,
                balance: 1.0,
                last_touched_at: now,
            };
            field.push(stored.clone());
            stored
        };
        let view = stored.view(group.config.pheromone_decay_rate, now);
        let event = append_event(
            &mut next,
            group_id,
            GroupEvent::PheromoneDeposited {
                depositor: depositor.to_string(),
                signal_type: signal_type.trim().to_string(),
                intensity: view.balance,
            },
        );
        self.commit(&mut guard, next).await?;
        Ok(GroupMutation { value: view, event })
    }

    /// Query a group pheromone field with derived demurrage balances.
    pub async fn pheromones(
        &self,
        group_id: &GroupId,
        actor: &str,
        signal_type: Option<&str>,
        min_balance: Option<f64>,
    ) -> Result<Vec<PheromoneView>, GroupRuntimeError> {
        if min_balance.is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value)) {
            return Err(GroupRuntimeError::Invalid(
                "min_balance must be finite and between 0 and 1".to_string(),
            ));
        }
        let state = self.state.lock().await;
        let group = require_group(&state, group_id)?;
        require_view(group, actor)?;
        let now = Utc::now();
        let mut values = state
            .pheromones
            .get(group_id)
            .into_iter()
            .flatten()
            .map(|stored| stored.view(group.config.pheromone_decay_rate, now))
            .filter(|view| {
                signal_type.is_none_or(|expected| view.pheromone.signal_type == expected)
                    && min_balance.is_none_or(|minimum| view.balance >= minimum)
            })
            .collect::<Vec<_>>();
        values.sort_by(|left, right| {
            right
                .balance
                .total_cmp(&left.balance)
                .then_with(|| right.last_touched_at.cmp(&left.last_touched_at))
        });
        Ok(values)
    }

    /// Return durable group events visible to the caller.
    pub async fn events(
        &self,
        group_id: &GroupId,
        actor: &str,
        after_seq: Option<u64>,
        limit: usize,
    ) -> Result<Vec<GroupEventRecord>, GroupRuntimeError> {
        let state = self.state.lock().await;
        let group = require_group(&state, group_id)?;
        require_view(group, actor)?;
        let limit = limit.clamp(1, 500);
        Ok(state
            .events
            .iter()
            .filter(|event| {
                event.group_id == *group_id && after_seq.is_none_or(|sequence| event.seq > sequence)
            })
            .take(limit)
            .cloned()
            .collect())
    }

    /// Durably append an event produced by a higher-layer group operation.
    pub async fn record_event(
        &self,
        group_id: &GroupId,
        actor: &str,
        event: GroupEvent,
    ) -> Result<GroupEventRecord, GroupRuntimeError> {
        let mut guard = self.state.lock().await;
        let group = require_group(&guard, group_id)?;
        if group.owner != actor
            && !group
                .members
                .iter()
                .any(|member| member.owner == actor && member.permissions.read)
        {
            return Err(GroupRuntimeError::Forbidden(
                "caller is not a group participant".to_string(),
            ));
        }
        let mut next = guard.clone();
        let event = append_event(&mut next, group_id, event);
        self.commit(&mut guard, next).await?;
        Ok(event)
    }

    async fn commit(
        &self,
        guard: &mut PersistedGroupState,
        next: PersistedGroupState,
    ) -> Result<(), GroupRuntimeError> {
        let json = serde_json::to_vec_pretty(&next)
            .map_err(|error| GroupRuntimeError::Storage(error.to_string()))?;
        roko_core::io::atomic_write_async(&self.path, &json)
            .await
            .map_err(|error| {
                GroupRuntimeError::Storage(format!(
                    "persist group state at {}: {error}",
                    self.path.display()
                ))
            })?;
        *guard = next;
        Ok(())
    }
}

fn load_state(path: &Path) -> Result<PersistedGroupState, GroupRuntimeError> {
    let data = match std::fs::read(path) {
        Ok(data) => data,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PersistedGroupState::default());
        }
        Err(error) => {
            return Err(GroupRuntimeError::Storage(format!(
                "read group state at {}: {error}",
                path.display()
            )));
        }
    };
    let state: PersistedGroupState = serde_json::from_slice(&data).map_err(|error| {
        GroupRuntimeError::Storage(format!("decode group state at {}: {error}", path.display()))
    })?;
    if state.version != GROUP_STATE_VERSION {
        return Err(GroupRuntimeError::Storage(format!(
            "unsupported group state version {} (expected {GROUP_STATE_VERSION})",
            state.version
        )));
    }
    Ok(state)
}

fn persist_state_sync(path: &Path, state: &PersistedGroupState) -> Result<(), GroupRuntimeError> {
    let json = serde_json::to_vec_pretty(state)
        .map_err(|error| GroupRuntimeError::Storage(error.to_string()))?;
    roko_core::io::atomic_write(path, &json).map_err(|error| {
        GroupRuntimeError::Storage(format!(
            "persist group state at {}: {error}",
            path.display()
        ))
    })
}

fn reconcile_definitions(
    state: &mut PersistedGroupState,
    definitions: &[GroupDefinition],
) -> Result<bool, GroupRuntimeError> {
    let mut names = BTreeSet::new();
    let mut changed = false;
    for definition in definitions {
        let normalized_name = definition.name.trim().to_ascii_lowercase();
        if !names.insert(normalized_name) {
            return Err(GroupRuntimeError::Invalid(format!(
                "duplicate configured group name '{}'",
                definition.name
            )));
        }
        let coordination = CoordinationMode::from_str(&definition.coordination)
            .map_err(GroupRuntimeError::Invalid)?;
        definition
            .assignment_strategy
            .as_deref()
            .map(AssignmentStrategy::from_str)
            .transpose()
            .map_err(GroupRuntimeError::Invalid)?;
        if coordination == CoordinationMode::LeaderFollower {
            let leader = definition.leader.as_deref().ok_or_else(|| {
                GroupRuntimeError::Invalid(format!(
                    "configured leader_follower group '{}' requires a leader",
                    definition.name
                ))
            })?;
            if !definition.members.iter().any(|member| member == leader) {
                return Err(GroupRuntimeError::Invalid(format!(
                    "configured leader '{}' is not a member of group '{}'",
                    leader, definition.name
                )));
            }
        }
        let knowledge_policy = definition
            .knowledge_policy
            .as_deref()
            .map(KnowledgePolicy::from_str)
            .transpose()
            .map_err(GroupRuntimeError::Invalid)?
            .unwrap_or_default();
        let config = GroupConfig {
            max_members: definition.max_members,
            auto_accept: true,
            public: definition.public,
            knowledge_policy,
            pheromone_decay_rate: definition.pheromone_decay_rate.unwrap_or(1.0),
        };
        validate_group_input(&definition.name, &config)?;
        if config
            .max_members
            .is_some_and(|maximum| definition.members.len() > maximum)
        {
            return Err(GroupRuntimeError::Invalid(format!(
                "configured group '{}' has more members than max_members",
                definition.name
            )));
        }

        let existing_id = state
            .groups
            .values()
            .find(|group| {
                group.owner == "local" && group.name.eq_ignore_ascii_case(&definition.name)
            })
            .map(|group| group.id.clone());
        let now = Utc::now();
        let members = definition
            .members
            .iter()
            .map(|agent_id| {
                let role = if definition.leader.as_deref() == Some(agent_id.as_str()) {
                    MemberRole::Leader
                } else {
                    MemberRole::Member
                };
                GroupMember {
                    agent_id: agent_id.clone(),
                    owner: "local".to_string(),
                    role,
                    permissions: MemberPermissions::FULL,
                    joined_at: now,
                }
            })
            .collect::<Vec<_>>();
        if let Some(group_id) = existing_id {
            let group = state.groups.get_mut(&group_id).ok_or_else(|| {
                GroupRuntimeError::Storage("configured group disappeared during reconcile".into())
            })?;
            let mut by_agent = group
                .members
                .iter()
                .map(|member| (member.agent_id.clone(), member.clone()))
                .collect::<BTreeMap<_, _>>();
            for member in members {
                by_agent.entry(member.agent_id.clone()).or_insert(member);
            }
            let reconciled_members = by_agent.into_values().collect::<Vec<_>>();
            if group.description != definition.description
                || group.coordination != coordination
                || group.config != config
                || group.members != reconciled_members
            {
                group.description = definition.description.clone();
                group.coordination = coordination;
                group.config = config;
                group.members = reconciled_members;
                group.updated_at = now;
                changed = true;
            }
        } else {
            let group = Group {
                id: GroupId::new(format!("grp-{}", Uuid::new_v4().simple())),
                name: definition.name.trim().to_string(),
                description: definition.description.trim().to_string(),
                owner: "local".to_string(),
                members,
                coordination,
                config,
                created_at: now,
                updated_at: now,
            };
            state.groups.insert(group.id.clone(), group);
            changed = true;
        }
    }
    Ok(changed)
}

fn validate_actor(actor: &str) -> Result<(), GroupRuntimeError> {
    if actor.trim().is_empty() || actor.len() > 256 {
        return Err(GroupRuntimeError::Invalid(
            "actor identity must be 1..=256 characters".to_string(),
        ));
    }
    Ok(())
}

fn validate_agent_id(agent_id: &str) -> Result<(), GroupRuntimeError> {
    if agent_id.trim().is_empty() || agent_id.len() > 256 {
        return Err(GroupRuntimeError::Invalid(
            "agent_id must be 1..=256 characters".to_string(),
        ));
    }
    Ok(())
}

fn validate_signal_type(signal_type: &str) -> Result<(), GroupRuntimeError> {
    let signal_type = signal_type.trim();
    if signal_type.is_empty()
        || signal_type.len() > 128
        || !signal_type
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
    {
        return Err(GroupRuntimeError::Invalid(
            "signal_type must be 1..=128 ASCII identifier characters".to_string(),
        ));
    }
    Ok(())
}

fn validate_group_input(name: &str, config: &GroupConfig) -> Result<(), GroupRuntimeError> {
    let name = name.trim();
    if name.is_empty() || name.len() > 128 {
        return Err(GroupRuntimeError::Invalid(
            "group name must be 1..=128 characters".to_string(),
        ));
    }
    if config
        .max_members
        .is_some_and(|maximum| maximum == 0 || maximum > MAX_MEMBERS)
    {
        return Err(GroupRuntimeError::Invalid(format!(
            "max_members must be between 1 and {MAX_MEMBERS}"
        )));
    }
    if !config.pheromone_decay_rate.is_finite()
        || !(0.0..=100.0).contains(&config.pheromone_decay_rate)
    {
        return Err(GroupRuntimeError::Invalid(
            "pheromone_decay_rate must be finite and between 0 and 100".to_string(),
        ));
    }
    Ok(())
}

fn validate_member_permissions(
    role: MemberRole,
    permissions: MemberPermissions,
) -> Result<(), GroupRuntimeError> {
    if role == MemberRole::Observer && (permissions.write || permissions.execute) {
        return Err(GroupRuntimeError::Invalid(
            "observer permissions must be read-only".to_string(),
        ));
    }
    if !permissions.read && (permissions.write || permissions.execute) {
        return Err(GroupRuntimeError::Invalid(
            "write/execute permissions require read permission".to_string(),
        ));
    }
    Ok(())
}

fn require_group<'a>(
    state: &'a PersistedGroupState,
    group_id: &GroupId,
) -> Result<&'a Group, GroupRuntimeError> {
    state
        .groups
        .get(group_id)
        .ok_or_else(|| GroupRuntimeError::NotFound(format!("group '{group_id}' not found")))
}

fn require_owner(group: &Group, actor: &str) -> Result<(), GroupRuntimeError> {
    if group.owner != actor {
        return Err(GroupRuntimeError::Forbidden(
            "only the group owner may perform this operation".to_string(),
        ));
    }
    Ok(())
}

fn require_view(group: &Group, actor: &str) -> Result<(), GroupRuntimeError> {
    if group.owner == actor
        || group
            .members
            .iter()
            .any(|member| member.owner == actor && member.permissions.read)
    {
        Ok(())
    } else {
        Err(GroupRuntimeError::Forbidden(
            "caller cannot view this group".to_string(),
        ))
    }
}

fn can_view(group: &Group, actor: &str) -> bool {
    group.config.public
        || group.owner == actor
        || group
            .members
            .iter()
            .any(|member| member.owner == actor && member.permissions.read)
}

fn group_name_exists(
    state: &PersistedGroupState,
    owner: &str,
    name: &str,
    except: Option<&GroupId>,
) -> bool {
    state.groups.values().any(|group| {
        group.owner == owner
            && group.name.eq_ignore_ascii_case(name.trim())
            && except != Some(&group.id)
    })
}

fn ensure_capacity(state: &PersistedGroupState, group: &Group) -> Result<(), GroupRuntimeError> {
    let pending = state
        .invitations
        .values()
        .filter(|invitation| {
            invitation.group_id == group.id && invitation.status == InvitationStatus::Pending
        })
        .count();
    if group
        .config
        .max_members
        .is_some_and(|maximum| group.members.len() + pending >= maximum)
    {
        return Err(GroupRuntimeError::Conflict(
            "group membership capacity has been reached".to_string(),
        ));
    }
    Ok(())
}

fn ensure_member_slot(group: &Group) -> Result<(), GroupRuntimeError> {
    if group
        .config
        .max_members
        .is_some_and(|maximum| group.members.len() >= maximum)
    {
        return Err(GroupRuntimeError::Conflict(
            "group membership capacity has been reached".to_string(),
        ));
    }
    Ok(())
}

fn expire_invitations(state: &mut PersistedGroupState, now: DateTime<Utc>) -> bool {
    let mut changed = false;
    for invitation in state.invitations.values_mut() {
        if invitation.status == InvitationStatus::Pending && invitation.expires_at <= now {
            invitation.status = InvitationStatus::Expired;
            changed = true;
        }
    }
    changed
}

fn append_event(
    state: &mut PersistedGroupState,
    group_id: &GroupId,
    event: GroupEvent,
) -> GroupEventRecord {
    let record = GroupEventRecord {
        seq: state.next_event_seq,
        group_id: group_id.clone(),
        room: event.room(group_id),
        event,
        occurred_at: Utc::now(),
    };
    state.next_event_seq = state.next_event_seq.saturating_add(1);
    state.events.push(record.clone());
    if state.events.len() > MAX_GROUP_EVENTS {
        let excess = state.events.len() - MAX_GROUP_EVENTS;
        state.events.drain(..excess);
    }
    record
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn input(max_members: Option<usize>) -> CreateGroupInput {
        CreateGroupInput {
            name: "research".into(),
            description: "shared".into(),
            coordination: CoordinationMode::Stigmergic,
            config: GroupConfig {
                max_members,
                ..GroupConfig::default()
            },
        }
    }

    #[tokio::test]
    async fn mutations_survive_restart() {
        let temp = tempfile::tempdir().unwrap();
        let runtime = GroupRuntime::open(temp.path(), &[]).unwrap();
        let group = runtime.create("owner", input(None)).await.unwrap().value;
        runtime
            .invite(
                &group.id,
                "owner",
                "owner",
                InviteRequest {
                    agent_id: "agent-a".into(),
                    role: MemberRole::Member,
                    permissions: MemberPermissions::FULL,
                },
            )
            .await
            .unwrap();
        runtime
            .deposit_pheromone(
                &group.id,
                "owner",
                "agent-a",
                "topic_relevance",
                None,
                serde_json::json!({"topic": "MEV"}),
            )
            .await
            .unwrap();
        drop(runtime);

        let reopened = GroupRuntime::open(temp.path(), &[]).unwrap();
        assert_eq!(
            reopened
                .get(&group.id, "owner")
                .await
                .unwrap()
                .members
                .len(),
            1
        );
        assert_eq!(
            reopened
                .pheromones(&group.id, "owner", None, None)
                .await
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn concurrent_invitations_cannot_overbook_capacity() {
        let temp = tempfile::tempdir().unwrap();
        let runtime = Arc::new(GroupRuntime::open(temp.path(), &[]).unwrap());
        let group = runtime.create("owner", input(Some(1))).await.unwrap().value;
        let mut tasks = Vec::new();
        for index in 0..16 {
            let runtime = Arc::clone(&runtime);
            let group_id = group.id.clone();
            tasks.push(tokio::spawn(async move {
                runtime
                    .invite(
                        &group_id,
                        "owner",
                        "owner",
                        InviteRequest {
                            agent_id: format!("agent-{index}"),
                            role: MemberRole::Member,
                            permissions: MemberPermissions::FULL,
                        },
                    )
                    .await
            }));
        }
        let mut successes = 0;
        for task in tasks {
            successes += usize::from(task.await.unwrap().is_ok());
        }
        assert_eq!(successes, 1);
        assert_eq!(
            runtime.get(&group.id, "owner").await.unwrap().members.len(),
            1
        );
    }

    #[tokio::test]
    async fn adversarial_permissions_and_identity_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let runtime = GroupRuntime::open(temp.path(), &[]).unwrap();
        assert!(runtime.create("", input(None)).await.is_err());
        let group = runtime.create("owner", input(None)).await.unwrap().value;
        let error = runtime
            .invite(
                &group.id,
                "intruder",
                "intruder",
                InviteRequest {
                    agent_id: "agent".into(),
                    role: MemberRole::Observer,
                    permissions: MemberPermissions::FULL,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            GroupRuntimeError::Invalid(_) | GroupRuntimeError::Forbidden(_)
        ));

        let public_group = runtime
            .create(
                "owner",
                CreateGroupInput {
                    name: "public-directory-entry".into(),
                    config: GroupConfig {
                        public: true,
                        ..GroupConfig::default()
                    },
                    ..input(None)
                },
            )
            .await
            .unwrap()
            .value;
        assert!(runtime.list("intruder").await.contains(&public_group));
        assert!(matches!(
            runtime.get(&public_group.id, "intruder").await,
            Err(GroupRuntimeError::Forbidden(_))
        ));
    }

    #[test]
    fn configured_groups_reconcile_idempotently() {
        let temp = tempfile::tempdir().unwrap();
        let definition = GroupDefinition {
            name: "configured".into(),
            description: "v1".into(),
            coordination: "leader_follower".into(),
            members: vec!["leader".into(), "worker".into()],
            leader: Some("leader".into()),
            assignment_strategy: Some("capability_match".into()),
            public: false,
            max_members: Some(3),
            knowledge_policy: Some("write_leader".into()),
            pheromone_decay_rate: Some(0.5),
        };
        GroupRuntime::open(temp.path(), std::slice::from_ref(&definition)).unwrap();
        let reopened = GroupRuntime::open(temp.path(), &[definition]).unwrap();
        let state = reopened.state.try_lock().unwrap();
        assert_eq!(state.groups.len(), 1);
        let group = state.groups.values().next().unwrap();
        assert_eq!(group.members.len(), 2);
        assert_eq!(group.members[0].role, MemberRole::Leader);
    }
}
