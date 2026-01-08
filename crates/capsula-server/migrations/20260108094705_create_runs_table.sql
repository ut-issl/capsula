CREATE TABLE runs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    command TEXT NOT NULL,
    vault TEXT NOT NULL,
    project_root TEXT NOT NULL,
    exit_code INTEGER,
    duration_ms INTEGER,
    stdout TEXT,
    stderr TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_runs_name ON runs(name);
CREATE INDEX idx_runs_timestamp_desc ON runs(timestamp DESC);
CREATE INDEX idx_runs_vault_timestamp ON runs(vault, timestamp DESC);
