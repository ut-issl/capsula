-- Multiple instances of the same hook_id within a single (run_id, phase)
-- are a first-class case for capsula-toml / capsula-json -style hooks
-- that capture exactly one file per [[*-run.hooks]] entry. The previous
-- UNIQUE(run_id, phase, hook_id) index caused later uploads to silently
-- overwrite earlier ones via ON CONFLICT.
--
-- This migration adds a generated `config_hash` column derived from the
-- canonical jsonb text representation of the hook config, and widens the
-- uniqueness key to include it. Two hook outputs with the same hook_id
-- but distinct configs now coexist; a re-upload of the same hook+config
-- still UPSERTs the existing row, preserving idempotency.

ALTER TABLE run_outputs
ADD COLUMN config_hash TEXT
GENERATED ALWAYS AS (md5(coalesce(config::text, ''))) STORED;

DROP INDEX idx_run_outputs_unique;

CREATE UNIQUE INDEX idx_run_outputs_unique
ON run_outputs(run_id, phase, hook_id, config_hash);
