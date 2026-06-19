-- Multiple instances of the same hook_id within a single (run_id, phase)
-- are a first-class case for capsula-toml / capsula-json / capture-command
-- style hooks that can appear several times in [[*-run.hooks]] arrays. The
-- previous UNIQUE(run_id, phase, hook_id) index caused later uploads to
-- silently overwrite earlier ones via ON CONFLICT.
--
-- This migration adds a `hook_index` column that records each hook's
-- position in the capsula.toml `pre_run` / `post_run` array — global
-- within the phase, not per hook_id. Querying becomes natural:
--
--   ORDER BY hook_index           -- original array order
--   WHERE  hook_index = 2          -- "the 3rd hook in pre_run"
--
-- and the unique key (run_id, phase, hook_index) does not need to mention
-- hook_id at all.
--
-- Backfill: existing rows get a global ordinal within each (run_id, phase)
-- using insertion order (id) as a proxy for original array position. Under
-- the old UNIQUE(run_id, phase, hook_id) constraint there is no ambiguity,
-- so the assignment is stable.

ALTER TABLE run_outputs
ADD COLUMN hook_index INTEGER NOT NULL DEFAULT 0;

UPDATE run_outputs r
SET hook_index = o.new_index
FROM (
    SELECT id,
           ROW_NUMBER() OVER (
               PARTITION BY run_id, phase
               ORDER BY id
           ) - 1 AS new_index
    FROM run_outputs
) AS o
WHERE r.id = o.id;

DROP INDEX idx_run_outputs_unique;

CREATE UNIQUE INDEX idx_run_outputs_unique
ON run_outputs(run_id, phase, hook_index);
