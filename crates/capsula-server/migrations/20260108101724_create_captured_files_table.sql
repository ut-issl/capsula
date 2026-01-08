CREATE TABLE captured_files (
    id SERIAL PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    size BIGINT NOT NULL,
    hash TEXT,
    storage_path TEXT NOT NULL,
    content_type TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(run_id, path)
);

CREATE INDEX idx_captured_files_run ON captured_files(run_id);
CREATE INDEX idx_captured_files_hash ON captured_files(hash);
