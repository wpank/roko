# Plan 02: Agent creation and control

**Layer:** 2
**Effort:** M (2-3 days)
**Depends on:** Plan 01 (dashboard resilience)

## Goal

Users can create agents from the dashboard, configure LLM provider keys,
deploy agents from templates, and start/stop agents through the UI.

## Current state

### Backend (roko-serve)

The backend endpoints already exist and are tested. Key routes from
`/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/`:

**Agents** (`agents.rs`):
- `POST /api/agents/create` -- create a new agent (spawns process via supervisor)
- `GET /api/managed-agents` -- list managed agents
- `GET /api/agents/{id}` -- get agent detail
- `GET /api/agents/{id}/profile` -- get agent model profile
- `POST /api/agents/{id}/start` -- start a stopped agent
- `POST /api/agents/{id}/stop` -- stop a running agent
- `POST /api/agents/{id}/message` -- send message to running agent

**Secrets** (`secrets.rs`):
- `GET /api/secrets` -- list configured secret namespaces (no values returned)
- `POST /api/secrets/{namespace}/{key}` -- set a secret value
- `DELETE /api/secrets/{namespace}/{key}` -- delete a secret
- `POST /api/secrets/{namespace}/{key}/test` -- test if key is valid (calls provider API)

**Templates** (`templates.rs`):
- `GET /api/templates` -- list templates
- `POST /api/templates` -- create a template
- `GET /api/templates/{name}` -- get template detail
- `DELETE /api/templates/{name}` -- delete template
- `POST /api/templates/{name}/deploy` -- deploy (run) a template

### Frontend (nunchi-dashboard)

**What exists:**
- `useCreateAgent()` mutation in `rokoApi.ts` -- calls `POST /api/agents/create`
- `useAgents()` query -- calls `GET /api/managed-agents`
- `useAgent(id)` query -- calls `GET /api/agents/{id}`
- `useSendMessage()` mutation -- calls `POST /api/agents/{id}/message`
- Fleet pages: `AgentDetailPage.tsx`, `GroupDetailPage.tsx`, `GroupsPage.tsx`, `TemplatesPage.tsx`
- Settings pages: `Settings.tsx`, `SettingsLayout.tsx`, `ThemeSettings.tsx`, `NotificationsSettings.tsx`, `EpistemicSettings.tsx`

**What is missing:**
- No UI for creating agents (the mutation exists but no form calls it)
- No provider keys settings page (secrets endpoints exist but no UI)
- No template deploy button (template list exists but no deploy action)
- No start/stop controls for agents
- No agent creation wizard/form

### Backend request/response shapes

Read these from the source:

**CreateAgentRequest** (from `agents.rs` lines ~80-120):
```rust
struct CreateAgentRequest {
    name: String,
    domain: String,
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    skills: Vec<String>,
    #[serde(default)]
    tier: Option<u32>,
    #[serde(default)]
    model: Option<String>,
}
```

Response: `{ "agent_id": "...", "status": "created" }`

**Secrets list response:**
```json
{
  "secrets": [
    { "namespace": "llm.anthropic", "source": "file", "configured": true }
  ]
}
```

**Set secret request:** `POST /api/secrets/llm/anthropic` with body `{ "value": "sk-ant-..." }`

**Test secret response:** `{ "status": "valid"|"invalid"|"error", "message": "..." }`

Supported test namespaces: `llm.anthropic`, `llm.openai`, `llm.gemini`, `llm.perplexity`.

## Tasks

### 2.1 Provider keys settings page

**What:** Wire the Settings page to display, set, test, and remove LLM
provider API keys through the secrets endpoints.

**Source files to read:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/secrets.rs` -- full endpoint implementation, request/response types, test functions
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/settings/Settings.tsx` -- existing settings page structure
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/settings/SettingsLayout.tsx` -- settings navigation
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoApi.ts` -- `fetchRoko` helper

**Target files to create:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/settings/ProviderKeys.tsx`

**Target files to modify:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/settings/SettingsLayout.tsx` -- add route for provider keys
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoApi.ts` -- add hooks for secrets API
- `/Users/will/dev/nunchi/nunchi-dashboard/src/router.tsx` -- add route if not present

**New hooks to add in rokoApi.ts:**

```typescript
// Add to rokoApi.ts

type SecretEntry = {
  namespace: string;
  source: string;
  configured: boolean;
};

type SecretsListResponse = {
  secrets: SecretEntry[];
};

type TestSecretResponse = {
  status: "valid" | "invalid" | "error";
  message: string;
};

export function useSecrets() {
  const online = useBackendOnline();
  return useQuery({
    queryKey: ["roko", "secrets"],
    queryFn: () => fetchRoko<SecretsListResponse>("/secrets"),
    enabled: online,
    refetchInterval: onlineInterval(60_000),
    retry: 1,
  });
}

export function useSetSecret() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ namespace, key, value }: { namespace: string; key: string; value: string }) =>
      fetchRoko<{ namespace: string; key: string; status: string }>(
        `/secrets/${namespace}/${key}`,
        { method: "POST", body: JSON.stringify({ value }) },
      ),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["roko", "secrets"] });
    },
  });
}

export function useDeleteSecret() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ namespace, key }: { namespace: string; key: string }) =>
      fetchRoko<{ namespace: string; key: string; deleted: boolean }>(
        `/secrets/${namespace}/${key}`,
        { method: "DELETE" },
      ),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["roko", "secrets"] });
    },
  });
}

export function useTestSecret() {
  return useMutation({
    mutationFn: ({ namespace, key }: { namespace: string; key: string }) =>
      fetchRoko<TestSecretResponse>(`/secrets/${namespace}/${key}/test`, {
        method: "POST",
      }),
  });
}
```

**ProviderKeys page structure:**

The page shows a card for each supported provider. Each card has:
- Provider name and icon
- Status: "Configured" (green) or "Not configured" (gray)
- "Test" button (calls `/test`, shows result inline)
- "Set key" or "Update key" button (opens inline input)
- "Remove" button (calls DELETE, with confirmation)

Supported providers (hardcoded list, matches backend test functions):

```typescript
const PROVIDERS = [
  { namespace: "llm", key: "anthropic", label: "Anthropic", description: "Claude models" },
  { namespace: "llm", key: "openai", label: "OpenAI", description: "GPT models" },
  { namespace: "llm", key: "gemini", label: "Google Gemini", description: "Gemini models" },
  { namespace: "llm", key: "perplexity", label: "Perplexity", description: "Research models" },
];
```

Wrap the page in `<RequiresWorkspace>` from Plan 01.

**Acceptance criteria:**
- Navigate to `/app/settings/providers` (or wherever the settings layout puts it).
- Without roko-serve: see workspace placeholder.
- With roko-serve: see provider cards. Each shows "Not configured" initially.
- Set an Anthropic key: enter key, click Save. Card shows "Configured."
- Test the key: click Test. See "valid" or "invalid" result.
- Remove the key: click Remove. Card shows "Not configured."
- Invalid key (empty string): backend rejects with 400. UI shows validation error.

---

### 2.2 Agent creation wizard — [x] DONE — implemented 2026-04-24

**What:** Add a multi-step form for creating agents from the Fleet page.

**Source files to read:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/agents.rs` -- `CreateAgentRequest` struct (search for `create_agent` handler), validation rules
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoApi.ts` -- `useCreateAgent()` mutation (already exists)
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/fleet/` -- existing fleet pages

**Target files to create:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/fleet/CreateAgentPage.tsx`

**Target files to modify:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/router.tsx` -- add `/app/fleet/create` route
- Fleet page -- add "Create Agent" button that links to the create page

**Form fields (matching CreateAgentRequest):**

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| name | text | yes | Must be non-empty, alphanumeric + hyphens |
| domain | text | yes | Free text describing the agent's domain |
| prompt | textarea | no | System prompt override |
| skills | tag input | no | Array of skill strings |
| model | select | no | Model override (e.g. "claude-sonnet-4-20250514") |

The `useCreateAgent()` mutation in rokoApi.ts already exists and sends:
```typescript
fetchRoko<Record<string, unknown>>("/agents/create", {
  method: "POST",
  body: JSON.stringify({ name, domain, prompt, skills }),
})
```

The implementer should verify the request shape matches the backend's
`CreateAgentRequest` by reading `agents.rs`. The current mutation omits
`tier` and `model` -- add those to the mutation if the backend accepts them.

**UI flow:**
1. User clicks "Create Agent" on Fleet page.
2. Form page opens with the fields above.
3. On submit, call `useCreateAgent()`.
4. On success, navigate to the new agent's detail page (`/app/fleet/{agent_id}`).
5. On error, show error message inline.

**Acceptance criteria:**
- Navigate to `/app/fleet`. See a "Create Agent" button.
- Click it. See the creation form.
- Fill in name="test-agent", domain="testing". Click Create.
- Agent appears in fleet list. Agent detail page loads.
- Submit with empty name: see validation error. No network request fires.

---

### 2.3 Template deployment — [x] DONE — implemented 2026-04-24

**What:** Add a "Deploy" action to each template card on the Templates page.

**Source files to read:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/templates.rs` -- `deploy_template` handler, request shape
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/fleet/TemplatesPage.tsx` -- existing templates page

**Target files to modify:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/fleet/TemplatesPage.tsx`
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoApi.ts` -- add `useDeployTemplate()` mutation

**New hook:**

```typescript
export function useDeployTemplate() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (name: string) =>
      fetchRoko<{ id: string }>(`/templates/${name}/deploy`, {
        method: "POST",
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.agents });
    },
  });
}
```

**Backend deploy endpoint** (`POST /api/templates/{name}/deploy`):

Read `templates.rs` for the full handler. It creates a new agent from the
template definition and starts it. Response includes the new agent ID.

The deploy_template handler may accept optional body parameters to override
template defaults. Read the handler to confirm. If it does, add an optional
`overrides` parameter to the mutation:

```typescript
mutationFn: ({ name, overrides }: { name: string; overrides?: Record<string, unknown> }) =>
  fetchRoko<{ id: string }>(`/templates/${name}/deploy`, {
    method: "POST",
    body: overrides ? JSON.stringify(overrides) : undefined,
  }),
```

**UI change:**

Each template card gets a "Deploy" button. On click:
1. Call `useDeployTemplate()` with the template name.
2. Show loading state on the button.
3. On success, show a toast/notification with the agent ID and a link to the agent detail page.
4. On error, show error message.

**Acceptance criteria:**
- Navigate to `/app/fleet/templates`. See template cards (if any are configured).
- Each card has a "Deploy" button.
- Click Deploy. Agent is created and appears in fleet list.
- Deploy button shows loading state during the request.

---

### 2.4 Agent start/stop controls — [x] DONE — implemented 2026-04-24

**What:** Add start, stop, and restart buttons to the agent detail page and
fleet list cards.

**Source files to read:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/agents.rs` -- search for `start_agent`, `stop_agent` handlers, find the exact route patterns and request shapes
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/fleet/AgentDetailPage.tsx` -- existing agent detail page

**Target files to modify:**
- `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/fleet/AgentDetailPage.tsx`
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoApi.ts` -- add start/stop mutations

**New hooks:**

```typescript
export function useStartAgent() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (agentId: string) =>
      fetchRoko<Record<string, unknown>>(`/agents/${agentId}/start`, {
        method: "POST",
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.agents });
    },
  });
}

export function useStopAgent() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (agentId: string) =>
      fetchRoko<Record<string, unknown>>(`/agents/${agentId}/stop`, {
        method: "POST",
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.agents });
    },
  });
}
```

**IMPORTANT:** Before implementing, read `agents.rs` to find the exact route
paths. The routes are registered in the `routes()` function at the top of the
file. Search for `start` and `stop` to confirm the paths. They may be:
- `/agents/{id}/start` and `/agents/{id}/stop`, or
- `/managed-agents/{id}/start` and `/managed-agents/{id}/stop`

Verify and use the correct paths.

**UI changes on AgentDetailPage:**

Add a toolbar/action bar at the top of the agent detail page with:
- **Start** button (disabled when agent is running)
- **Stop** button (disabled when agent is stopped)
- **Restart** button (calls stop then start)
- Status badge showing current state (running/stopped/error)

Use the agent's `status` field from the API response to determine which
buttons are enabled. Common status values: `"running"`, `"stopped"`,
`"registered"`, `"created"`.

**Acceptance criteria:**
- Navigate to an agent's detail page.
- Agent is running: see Stop and Restart buttons enabled, Start disabled.
- Click Stop. Agent status changes to stopped. Stop button disables, Start enables.
- Click Start. Agent status changes to running.
- Click Restart. Agent cycles through stop/start.
- Buttons show loading spinners during the request.
- Errors (e.g. agent already stopped) display inline, not as page-level errors.

### 2.5 Team invite and shared workspace access

**What:** Owner can invite teammates by email from the dashboard. Teammates
log in via Privy and get access to the same workspace. Enables shared
development environments.

**Architecture reference:** `08-auth.md` — "Shared workspace access" section.

**Backend implementation (roko-serve):**

Create `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/team.rs`:

```rust
// POST /api/team/invite
// Requires: owner or admin role
async fn invite_member(
    State(state): State<AppState>,
    auth: AuthContext,  // must be owner or admin
    Json(body): Json<InviteRequest>,  // { email: String, role: String }
) -> Result<Json<InviteResponse>> {
    // 1. Validate caller is owner or admin (from auth context)
    // 2. Store invitation in .roko/users/invitations.json
    //    { identifier: body.email, type: "email", role: body.role, invited_at, invited_by }
    // 3. Return success
    // NO Privy API calls — authorization is entirely local
}

// GET /api/team/members
async fn list_members(State(state): State<AppState>, auth: AuthContext) -> Result<Json<Vec<TeamMember>>> {
    // Read from .roko/team/members.json
    // Each member: { privy_did, email, role, joined_at }
}

// PUT /api/team/members/:did
async fn update_role(/* ... */) -> Result<Json<TeamMember>> {
    // Update Privy custom_metadata: POST https://auth.privy.io/api/v1/users/{did}/custom_metadata
    // Body: { "custom_metadata": { "role": new_role } }
}

// DELETE /api/team/members/:did
async fn remove_member(/* ... */) -> Result<()> {
    // Remove from Privy allowlist + delete local record
}
```

Register routes in `routes/mod.rs`:
```rust
.route("/api/team/invite", post(team::invite_member))
.route("/api/team/members", get(team::list_members))
.route("/api/team/members/:did", put(team::update_role).delete(team::remove_member))
.route("/api/team/me", get(team::current_user))
```

**Privy configuration:**
- `PRIVY_APP_ID` env var only (public, baked into deploy template by Nunchi)
- No `PRIVY_APP_SECRET` needed — JWT verification uses public JWKS endpoint
- Privy login is open (no allowlist) — roko-serve handles all authorization locally

**Dashboard implementation:**

Create `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/settings/TeamPage.tsx`:
- List current members with roles
- "Invite teammate" form: email input + role dropdown
- Change role / remove member actions (owner/admin only)
- Show pending invitations

**Auth middleware update:**

In roko-serve middleware, after validating Privy JWT:
1. Extract email (and wallet if present) from JWT claims
2. Look up email in `.roko/users/` (local file, no Privy API call)
3. If found: inject user + role into request context
4. If not found but email matches pending invitation: auto-create user with invited role
5. If first user ever (no users in `.roko/users/`): auto-assign `owner` role
6. If not found and no invitation: return 403 "Not a member of this workspace"
No PRIVY_APP_SECRET or Privy server API calls needed.

**Acceptance criteria:**
- [ ] Owner can invite a teammate by email from Settings > Team
- [ ] Invited user can log in via Privy and sees the same workspace
- [ ] Member cannot access admin routes (secrets, config) — gets 403
- [ ] Owner can change a member's role
- [ ] Owner can remove a member (they can no longer log in)
- [ ] First user to log in becomes owner automatically
- [ ] Team members list shows all members with roles and join dates

## Dependencies

- **Plan 01** must ship first (connectivity gating, RequiresWorkspace wrapper).
  Without Plan 01, the create/deploy mutations throw when roko-serve is down,
  with no graceful fallback.

## Files touched (summary)

| Action | Path |
|--------|------|
| Create | `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/settings/ProviderKeys.tsx` |
| Create | `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/fleet/CreateAgentPage.tsx` |
| Modify | `/Users/will/dev/nunchi/nunchi-dashboard/src/services/rokoApi.ts` |
| Modify | `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/fleet/TemplatesPage.tsx` |
| Modify | `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/fleet/AgentDetailPage.tsx` |
| Modify | `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/settings/SettingsLayout.tsx` |
| Modify | `/Users/will/dev/nunchi/nunchi-dashboard/src/router.tsx` |
| Create | `/Users/will/dev/nunchi/nunchi-dashboard/src/pages/settings/TeamPage.tsx` |
| Create | `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/team.rs` |
| Modify | `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/mod.rs` |
| Modify | `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/middleware.rs` |
