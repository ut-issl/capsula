-- Delete any existing duplicates (keep the earliest by id)
DELETE FROM run_outputs a USING run_outputs b
WHERE a.id > b.id
  AND a.run_id = b.run_id
  AND a.phase = b.phase
  AND a.hook_id = b.hook_id;

CREATE UNIQUE INDEX idx_run_outputs_unique ON run_outputs(run_id, phase, hook_id);
