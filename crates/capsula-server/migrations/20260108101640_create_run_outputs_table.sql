CREATE TABLE run_outputs (
    id SERIAL PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    phase TEXT NOT NULL CHECK(phase IN ('pre', 'post')),
    hook_id TEXT NOT NULL,
    output JSONB NOT NULL,
    success BOOLEAN NOT NULL,
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_run_outputs_run_phase ON run_outputs(run_id, phase);
CREATE INDEX idx_run_outputs_hook_success ON run_outputs(hook_id, success);
CREATE INDEX idx_run_outputs_output ON run_outputs USING GIN(output);
