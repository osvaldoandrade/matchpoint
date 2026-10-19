CREATE TABLE schema_migrations (
  version INTEGER PRIMARY KEY NOT NULL,
  applied_at TEXT NOT NULL
);

CREATE TABLE job_meta (
  key TEXT PRIMARY KEY NOT NULL,
  value TEXT NOT NULL
);

CREATE TABLE batches (
  batch_id INTEGER PRIMARY KEY NOT NULL,
  state TEXT NOT NULL CHECK (state IN ('PENDING', 'RUNNING', 'COMPLETED', 'COMPLETED_WITH_ERRORS')),
  row_from INTEGER NOT NULL,
  row_to INTEGER NOT NULL,
  pending_count INTEGER NOT NULL,
  in_flight_count INTEGER NOT NULL,
  retry_wait_count INTEGER NOT NULL,
  succeeded_count INTEGER NOT NULL,
  failed_count INTEGER NOT NULL,
  unknown_count INTEGER NOT NULL
);

CREATE TABLE records (
  row_id INTEGER PRIMARY KEY NOT NULL,
  batch_id INTEGER NOT NULL REFERENCES batches(batch_id),
  cpf TEXT NOT NULL,
  state TEXT NOT NULL CHECK (state IN (
    'PENDING', 'IN_FLIGHT', 'RETRY_WAIT', 'SUCCEEDED', 'FAILED_FINAL', 'UNKNOWN'
  )),
  is_customer INTEGER NULL CHECK (is_customer IS NULL OR is_customer IN (0, 1)),
  evidence_source TEXT NOT NULL,
  checked_at TEXT NULL,
  attempts INTEGER NOT NULL DEFAULT 0,
  attempt_origin INTEGER NOT NULL DEFAULT 0,
  http_status INTEGER NULL,
  error_code TEXT NULL,
  request_id TEXT NULL,
  next_attempt_at TEXT NULL,
  validation_error TEXT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE attempts (
  attempt_id INTEGER PRIMARY KEY AUTOINCREMENT,
  row_id INTEGER NOT NULL REFERENCES records(row_id),
  started_at TEXT NOT NULL,
  finished_at TEXT NULL,
  http_status INTEGER NULL,
  error_code TEXT NULL,
  request_id TEXT NULL,
  outcome TEXT NOT NULL,
  token_generation INTEGER NULL
);

CREATE INDEX idx_attempts_row ON attempts(row_id);

-- Claim PENDING rows in input order. Partial index stays small while the job runs down.
CREATE INDEX idx_records_pending ON records(row_id) WHERE state = 'PENDING';

-- Due retries. Columns match WHERE state AND next_attempt_at, then row_id.
CREATE INDEX idx_records_retry ON records(next_attempt_at, row_id) WHERE state = 'RETRY_WAIT';

-- Reconcile and per-batch completion. Not used on the per-request hot path.
CREATE INDEX idx_records_batch_state ON records(batch_id, state);

CREATE TABLE events (
  event_id INTEGER PRIMARY KEY AUTOINCREMENT,
  occurred_at TEXT NOT NULL,
  kind TEXT NOT NULL,
  detail TEXT NOT NULL
);

CREATE TABLE prepare_checkpoint (
  id INTEGER PRIMARY KEY NOT NULL CHECK (id = 1),
  rows_imported INTEGER NOT NULL,
  byte_offset INTEGER NOT NULL,
  line_no INTEGER NOT NULL,
  record_index INTEGER NOT NULL,
  header_done INTEGER NOT NULL,
  complete INTEGER NOT NULL
);
