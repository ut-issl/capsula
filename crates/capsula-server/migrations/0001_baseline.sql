-- Baseline schema of the multi-tenant capsula-server (API v2).
--
-- This replaces the migrations of the single-tenant server. Databases created
-- by earlier versions are not migrated: drop and recreate the database, then
-- refill it with `capsula push --all` from the local `.capsula/` directories.
--
-- Tenancy model: an organization (lab or personal space) owns vaults
-- (projects), vaults own runs. Every org-scoped row carries `org_id`, and the
-- rows below a run are tied to it through composite foreign keys that include
-- `org_id`, so a run can never reference data of another organization.

-- Accounts ------------------------------------------------------------------

CREATE TABLE users (
    id UUID PRIMARY KEY,
    -- Also the slug of the user's personal organization.
    slug TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    primary_email TEXT,
    accepted_terms_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- One row per identity-provider account linked to a user.
CREATE TABLE identities (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider TEXT NOT NULL CHECK (provider IN ('google', 'github')),
    subject TEXT NOT NULL,
    email TEXT,
    email_verified BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (provider, subject)
);

CREATE INDEX idx_identities_user ON identities(user_id);

-- Organizations and membership ----------------------------------------------

CREATE TABLE organizations (
    id UUID PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('personal', 'team')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);

-- Access to an organization is decided only by this table.
CREATE TABLE memberships (
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('owner', 'member')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (org_id, user_id)
);

CREATE INDEX idx_memberships_user ON memberships(user_id);

-- Invitation links. Only the hash of the code is stored.
CREATE TABLE invitations (
    id UUID PRIMARY KEY,
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    code_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    max_uses INTEGER,
    uses INTEGER NOT NULL DEFAULT 0,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_invitations_org ON invitations(org_id);

-- Credentials ---------------------------------------------------------------

-- SSH public keys registered for `capsula login`.
CREATE TABLE ssh_keys (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- SHA-256 fingerprint (`SHA256:...`). A key belongs to at most one user.
    fingerprint TEXT NOT NULL UNIQUE,
    public_key TEXT NOT NULL,
    title TEXT NOT NULL,
    source TEXT NOT NULL CHECK (source IN ('manual', 'github')),
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ssh_keys_user ON ssh_keys(user_id);

-- Single-use nonces issued by `POST /api/v2/auth/ssh/challenge`.
CREATE TABLE auth_challenges (
    nonce TEXT PRIMARY KEY,
    fingerprint TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ
);

-- Bearer tokens. `login` tokens are short-lived and issued through an SSH key
-- (deleting the key revokes them through the cascade); `manual` tokens are
-- long-lived tokens created in the web UI for CI and scripts.
CREATE TABLE api_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('login', 'manual')),
    ssh_key_id UUID REFERENCES ssh_keys(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    -- Public, non-secret prefix shown in the UI to identify the token.
    prefix TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    last_used_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_api_tokens_user ON api_tokens(user_id);

-- Browser sessions.
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sessions_user ON sessions(user_id);

-- Vaults and runs -----------------------------------------------------------

CREATE TABLE vaults (
    id UUID PRIMARY KEY,
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (org_id, name),
    -- Lets `runs` reference the vault together with its org.
    UNIQUE (org_id, id)
);

-- Run ids are ULIDs generated by the CLI; they are unique within an org.
CREATE TABLE runs (
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    vault_id UUID NOT NULL,
    name TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    command TEXT NOT NULL,
    project_root TEXT NOT NULL,
    exit_code INTEGER,
    duration_ms INTEGER,
    stdout TEXT,
    stderr TEXT,
    pushed_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (org_id, id),
    FOREIGN KEY (org_id, vault_id) REFERENCES vaults(org_id, id) ON DELETE CASCADE
);

CREATE INDEX idx_runs_vault_timestamp ON runs(vault_id, timestamp DESC);
CREATE INDEX idx_runs_org_timestamp ON runs(org_id, timestamp DESC);
CREATE INDEX idx_runs_org_name ON runs(org_id, name);
CREATE INDEX idx_runs_pushed_by ON runs(pushed_by_user_id);

-- Hook outputs of a run. `hook_index` is the position of the hook in the
-- `[[pre-run.hooks]]` / `[[post-run.hooks]]` array of `capsula.toml`, so the
-- same hook id can appear several times in one phase.
CREATE TABLE run_outputs (
    id BIGSERIAL PRIMARY KEY,
    org_id UUID NOT NULL,
    run_id TEXT NOT NULL,
    phase TEXT NOT NULL CHECK (phase IN ('pre', 'post')),
    hook_index INTEGER NOT NULL,
    hook_id TEXT NOT NULL,
    output JSONB NOT NULL,
    config JSONB,
    success BOOLEAN NOT NULL,
    error TEXT,
    failure_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (org_id, run_id, phase, hook_index),
    FOREIGN KEY (org_id, run_id) REFERENCES runs(org_id, id) ON DELETE CASCADE
);

CREATE INDEX idx_run_outputs_hook ON run_outputs(org_id, hook_id);

-- Captured files of a run. The content lives in object storage under
-- `storage_key` (`{org_id}/sha256/{hash[0..2]}/{hash[2..4]}/{hash}`), so
-- identical content is stored once per organization.
CREATE TABLE captured_files (
    id BIGSERIAL PRIMARY KEY,
    org_id UUID NOT NULL,
    run_id TEXT NOT NULL,
    path TEXT NOT NULL,
    size BIGINT NOT NULL,
    hash TEXT NOT NULL,
    storage_key TEXT NOT NULL,
    content_type TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (org_id, run_id, path),
    FOREIGN KEY (org_id, run_id) REFERENCES runs(org_id, id) ON DELETE CASCADE
);

-- Finding the other runs that still reference an object before deleting it.
CREATE INDEX idx_captured_files_storage_key ON captured_files(org_id, storage_key);

-- Audit log -----------------------------------------------------------------

-- Rows outlive the org or user they refer to; the references become NULL and
-- `detail` keeps what is needed to read the entry.
CREATE TABLE audit_logs (
    id BIGSERIAL PRIMARY KEY,
    org_id UUID REFERENCES organizations(id) ON DELETE SET NULL,
    actor_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    target TEXT,
    detail JSONB,
    at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_logs_org_at ON audit_logs(org_id, at DESC);
