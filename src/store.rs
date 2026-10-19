use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use crate::domain::{
    transition_record, BatchState, Counts, Evidence, JobState, RecordEvent, RecordState,
};
use crate::error::{map_sql, Error};

pub const SCHEMA_VERSION: i32 = 1;

#[derive(Clone, Debug)]
pub struct Checkpoint {
    pub rows_imported: i64,
    pub byte_offset: u64,
    pub line_no: u64,
    pub record_index: u64,
    pub header_done: bool,
    pub complete: bool,
}

impl Default for Checkpoint {
    fn default() -> Self {
        Self {
            rows_imported: 0,
            byte_offset: 0,
            line_no: 1,
            record_index: 0,
            header_done: false,
            complete: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct NewRow {
    pub row_id: i64,
    pub cpf: String,
    pub rejected: Option<&'static str>,
}

#[derive(Clone, Debug)]
pub struct WorkItem {
    pub row_id: i64,
    pub batch_id: i64,
    pub cpf: String,
    pub attempts: i32,
    pub attempt_origin: i32,
}

#[derive(Clone, Debug)]
pub struct Outcome {
    pub row_id: i64,
    pub batch_id: i64,
    pub event: RecordEvent,
    pub sends: i32,
    pub http_status: Option<i32>,
    pub error_code: Option<String>,
    pub request_id: Option<String>,
    pub is_customer: Option<bool>,
    pub evidence: Evidence,
    pub next_attempt_at: Option<String>,
    pub token_generation: Option<i64>,
    pub started_at: String,
    pub finished_at: String,
}

#[derive(Clone, Debug)]
pub struct BatchInfo {
    pub batch_id: i64,
    pub state: String,
    pub row_from: i64,
    pub row_to: i64,
    pub pending: i64,
    pub in_flight: i64,
    pub retry_wait: i64,
    pub succeeded: i64,
    pub failed: i64,
    pub unknown: i64,
}

#[derive(Clone, Debug)]
pub struct ExportRow {
    pub row_id: i64,
    pub cpf: String,
    pub state: String,
    pub is_customer: Option<bool>,
    pub checked_at: Option<String>,
    pub attempts: i64,
    pub http_status: Option<i64>,
    pub error_code: Option<String>,
    pub request_id: Option<String>,
    pub evidence_source: String,
}

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self, Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path).map_err(map_sql)?;
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
             PRAGMA busy_timeout=5000;
             PRAGMA synchronous=FULL;
             PRAGMA temp_store=MEMORY;
             PRAGMA cache_size=-65536;
             PRAGMA wal_autocheckpoint=1000;",
        )
        .map_err(map_sql)?;
        let mode: String = conn
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
            .map_err(map_sql)?;
        if mode != "wal" {
            return Err(Error::Msg(format!("sqlite journal_mode is {mode}")));
        }
        Ok(Self { conn })
    }

    pub fn open_read(path: &Path) -> Result<Self, Error> {
        let conn = Connection::open(path).map_err(map_sql)?;
        conn.execute_batch("PRAGMA query_only=ON; PRAGMA busy_timeout=5000;")
            .map_err(map_sql)?;
        Ok(Self { conn })
    }

    pub fn migrate(&self) -> Result<(), Error> {
        let version: i32 = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(map_sql)?;
        if version == SCHEMA_VERSION {
            return Ok(());
        }
        if version > SCHEMA_VERSION {
            return Err(Error::Schema {
                found: version,
                supported: SCHEMA_VERSION,
            });
        }
        if version == 0 {
            self.conn
                .execute_batch(include_str!("../migrations/0001_up.sql"))
                .map_err(map_sql)?;
            self.conn
                .execute(
                    "INSERT INTO schema_migrations(version, applied_at) VALUES (1, ?1)",
                    params![crate::timeutil::format_rfc3339(0)],
                )
                .map_err(map_sql)?;
            self.conn
                .execute_batch("PRAGMA user_version=1")
                .map_err(map_sql)?;
        }
        Ok(())
    }

    pub fn meta_get(&self, key: &str) -> Result<Option<String>, Error> {
        self.conn
            .query_row(
                "SELECT value FROM job_meta WHERE key=?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .map_err(map_sql)
    }

    pub fn meta_set(&self, key: &str, value: &str) -> Result<(), Error> {
        self.conn
            .execute(
                "INSERT INTO job_meta(key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                params![key, value],
            )
            .map_err(map_sql)?;
        Ok(())
    }

    pub fn checkpoint(&self) -> Result<Checkpoint, Error> {
        let row = self
            .conn
            .query_row(
                "SELECT rows_imported, byte_offset, line_no, record_index, header_done, complete
                 FROM prepare_checkpoint WHERE id=1",
                [],
                |row| {
                    Ok(Checkpoint {
                        rows_imported: row.get(0)?,
                        byte_offset: row.get::<_, i64>(1)? as u64,
                        line_no: row.get::<_, i64>(2)? as u64,
                        record_index: row.get::<_, i64>(3)? as u64,
                        header_done: row.get::<_, i64>(4)? != 0,
                        complete: row.get::<_, i64>(5)? != 0,
                    })
                },
            )
            .optional()
            .map_err(map_sql)?;
        Ok(row.unwrap_or_default())
    }

    /// Inserts one preparation batch and the checkpoint in a single transaction.
    pub fn insert_prepared(
        &mut self,
        batch_id: i64,
        rows: &[NewRow],
        ckpt: &Checkpoint,
        now: &str,
        fail_before_commit: bool,
    ) -> Result<(), Error> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_sql)?;
        if rows.is_empty() {
            write_checkpoint(&tx, ckpt)?;
            if fail_before_commit {
                return Err(Error::DiskFull);
            }
            tx.commit().map_err(map_sql)?;
            return Ok(());
        }
        let row_from = rows.first().map(|r| r.row_id).unwrap_or(0);
        let row_to = rows.last().map(|r| r.row_id).unwrap_or(0);
        let mut pending = 0_i64;
        let mut failed = 0_i64;
        for row in rows {
            if row.rejected.is_some() {
                failed += 1;
            } else {
                pending += 1;
            }
        }
        let state = if pending == 0 {
            BatchState::CompletedWithErrors
        } else {
            BatchState::Pending
        };
        tx.execute(
            "INSERT INTO batches(batch_id, state, row_from, row_to, pending_count, in_flight_count,
             retry_wait_count, succeeded_count, failed_count, unknown_count)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, 0, ?6, 0)",
            params![batch_id, state.as_str(), row_from, row_to, pending, failed],
        )
        .map_err(map_sql)?;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO records(row_id, batch_id, cpf, state, is_customer, evidence_source,
                     checked_at, attempts, attempt_origin, http_status, error_code, request_id,
                     next_attempt_at, validation_error, updated_at)
                     VALUES (?1, ?2, ?3, ?4, NULL, 'none', NULL, 0, 0, NULL, ?5, NULL, NULL, ?6, ?7)",
                )
                .map_err(map_sql)?;
            for row in rows {
                let (state, err_code, validation) = if let Some(reason) = row.rejected {
                    (
                        RecordState::FailedFinal.as_str(),
                        Some(format!("validation:{reason}")),
                        Some(reason),
                    )
                } else {
                    (RecordState::Pending.as_str(), None, None)
                };
                stmt.execute(params![
                    row.row_id, batch_id, row.cpf, state, err_code, validation, now
                ])
                .map_err(map_sql)?;
            }
        }
        write_checkpoint(&tx, ckpt)?;
        if fail_before_commit {
            return Err(Error::DiskFull);
        }
        tx.commit().map_err(map_sql)?;
        Ok(())
    }

    pub fn claim(&mut self, limit: usize, now: &str) -> Result<Vec<WorkItem>, Error> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_sql)?;
        let mut items = Vec::new();
        {
            let mut stmt = tx
                .prepare(
                    "SELECT row_id, batch_id, cpf, attempts, attempt_origin
                     FROM records WHERE state='PENDING' ORDER BY row_id LIMIT ?1",
                )
                .map_err(map_sql)?;
            let mut rows = stmt.query(params![limit as i64]).map_err(map_sql)?;
            while let Some(row) = rows.next().map_err(map_sql)? {
                items.push(read_work(row)?);
            }
        }
        let mut left = limit.saturating_sub(items.len());
        if left > 0 {
            let mut stmt = tx
                .prepare(
                    "SELECT row_id, batch_id, cpf, attempts, attempt_origin
                     FROM records
                     WHERE state='RETRY_WAIT' AND next_attempt_at <= ?1
                     ORDER BY next_attempt_at, row_id LIMIT ?2",
                )
                .map_err(map_sql)?;
            let mut rows = stmt.query(params![now, left as i64]).map_err(map_sql)?;
            while let Some(row) = rows.next().map_err(map_sql)? {
                items.push(read_work(row)?);
                left = left.saturating_sub(1);
            }
        }
        for item in &items {
            let from = current_state(&tx, item.row_id)?;
            let to = transition_record(from, RecordEvent::Claim)?;
            tx.execute(
                "UPDATE records SET state=?1, updated_at=?2 WHERE row_id=?3",
                params![to.as_str(), now, item.row_id],
            )
            .map_err(map_sql)?;
            apply_delta(&tx, item.batch_id, from, to)?;
        }
        tx.commit().map_err(map_sql)?;
        Ok(items)
    }

    pub fn apply(
        &mut self,
        outcomes: &[Outcome],
        now: &str,
        fail_before_commit: bool,
    ) -> Result<(), Error> {
        if outcomes.is_empty() && !fail_before_commit {
            return Ok(());
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_sql)?;
        for outcome in outcomes {
            let from = match current_state_opt(&tx, outcome.row_id)? {
                Some(state) => state,
                None => continue,
            };
            if from != RecordState::InFlight {
                continue;
            }
            let to = transition_record(from, outcome.event)?;
            let attempts_delta = outcome.sends.max(0);
            tx.execute(
                "UPDATE records SET state=?1, is_customer=?2, evidence_source=?3, checked_at=?4,
                 attempts=attempts+?5, http_status=?6, error_code=?7, request_id=?8,
                 next_attempt_at=?9, updated_at=?10
                 WHERE row_id=?11 AND state='IN_FLIGHT'",
                params![
                    to.as_str(),
                    outcome.is_customer.map(|v| if v { 1 } else { 0 }),
                    outcome.evidence.as_str(),
                    if to.is_terminal() {
                        Some(outcome.finished_at.as_str())
                    } else {
                        None
                    },
                    attempts_delta,
                    outcome.http_status,
                    outcome.error_code,
                    outcome.request_id,
                    outcome.next_attempt_at,
                    now,
                    outcome.row_id,
                ],
            )
            .map_err(map_sql)?;
            tx.execute(
                "INSERT INTO attempts(row_id, started_at, finished_at, http_status, error_code,
                 request_id, outcome, token_generation)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    outcome.row_id,
                    outcome.started_at,
                    outcome.finished_at,
                    outcome.http_status,
                    outcome.error_code,
                    outcome.request_id,
                    outcome.event.as_str(),
                    outcome.token_generation,
                ],
            )
            .map_err(map_sql)?;
            apply_delta(&tx, outcome.batch_id, from, to)?;
        }
        if fail_before_commit {
            return Err(Error::DiskFull);
        }
        tx.commit().map_err(map_sql)?;
        Ok(())
    }

    pub fn counts(&self) -> Result<Counts, Error> {
        self.conn
            .query_row(
                "SELECT
                   COALESCE(SUM(pending_count),0),
                   COALESCE(SUM(in_flight_count),0),
                   COALESCE(SUM(retry_wait_count),0),
                   COALESCE(SUM(succeeded_count),0),
                   COALESCE(SUM(failed_count),0),
                   COALESCE(SUM(unknown_count),0)
                 FROM batches",
                [],
                |row| {
                    Ok(Counts {
                        pending: row.get(0)?,
                        in_flight: row.get(1)?,
                        retry_wait: row.get(2)?,
                        succeeded: row.get(3)?,
                        failed_final: row.get(4)?,
                        unknown: row.get(5)?,
                    })
                },
            )
            .map_err(map_sql)
    }

    pub fn recover_inflight(&mut self, now: &str) -> Result<u64, Error> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_sql)?;
        let rows: Vec<(i64, i64)> = {
            let mut stmt = tx
                .prepare(
                    "SELECT row_id, batch_id FROM records WHERE state='IN_FLIGHT' ORDER BY row_id",
                )
                .map_err(map_sql)?;
            let mut out = Vec::new();
            let mut q = stmt.query([]).map_err(map_sql)?;
            while let Some(row) = q.next().map_err(map_sql)? {
                out.push((row.get(0).map_err(map_sql)?, row.get(1).map_err(map_sql)?));
            }
            out
        };
        for (row_id, batch_id) in &rows {
            let to = transition_record(RecordState::InFlight, RecordEvent::LoseOutcome)?;
            tx.execute(
                "UPDATE records SET state=?1, evidence_source='http_only', error_code='inflight_recovered',
                 checked_at=?2, updated_at=?2 WHERE row_id=?3 AND state='IN_FLIGHT'",
                params![to.as_str(), now, row_id],
            )
            .map_err(map_sql)?;
            tx.execute(
                "INSERT INTO attempts(row_id, started_at, finished_at, outcome, error_code)
                 VALUES (?1, ?2, ?2, 'LoseOutcome', 'inflight_recovered')",
                params![row_id, now],
            )
            .map_err(map_sql)?;
            apply_delta(&tx, *batch_id, RecordState::InFlight, to)?;
        }
        tx.commit().map_err(map_sql)?;
        Ok(rows.len() as u64)
    }

    pub fn requeue(&mut self, from: RecordState, now: &str) -> Result<u64, Error> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_sql)?;
        let rows: Vec<(i64, i64, i64)> = {
            let mut stmt = tx
                .prepare(
                    "SELECT row_id, batch_id, attempts FROM records WHERE state=?1 ORDER BY row_id",
                )
                .map_err(map_sql)?;
            let mut out = Vec::new();
            let mut q = stmt.query(params![from.as_str()]).map_err(map_sql)?;
            while let Some(row) = q.next().map_err(map_sql)? {
                out.push((
                    row.get(0).map_err(map_sql)?,
                    row.get(1).map_err(map_sql)?,
                    row.get(2).map_err(map_sql)?,
                ));
            }
            out
        };
        for (row_id, batch_id, attempts) in &rows {
            let to = transition_record(from, RecordEvent::Requeue)?;
            tx.execute(
                "UPDATE records SET state='PENDING', attempt_origin=?1, is_customer=NULL,
                 evidence_source='none', checked_at=NULL, http_status=NULL, error_code=NULL,
                 request_id=NULL, next_attempt_at=NULL, validation_error=NULL, updated_at=?2
                 WHERE row_id=?3",
                params![attempts, now, row_id],
            )
            .map_err(map_sql)?;
            apply_delta(&tx, *batch_id, from, to)?;
        }
        tx.commit().map_err(map_sql)?;
        Ok(rows.len() as u64)
    }

    /// Rebuilds batch counters from records when they drift. Returns true when a repair ran.
    pub fn reconcile(&mut self) -> Result<bool, Error> {
        let summed = self.counts()?;
        let actual = self.count_records()?;
        if summed == actual {
            return Ok(false);
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_sql)?;
        tx.execute_batch(
            "UPDATE batches SET
               pending_count=(SELECT COUNT(*) FROM records r WHERE r.batch_id=batches.batch_id AND r.state='PENDING'),
               in_flight_count=(SELECT COUNT(*) FROM records r WHERE r.batch_id=batches.batch_id AND r.state='IN_FLIGHT'),
               retry_wait_count=(SELECT COUNT(*) FROM records r WHERE r.batch_id=batches.batch_id AND r.state='RETRY_WAIT'),
               succeeded_count=(SELECT COUNT(*) FROM records r WHERE r.batch_id=batches.batch_id AND r.state='SUCCEEDED'),
               failed_count=(SELECT COUNT(*) FROM records r WHERE r.batch_id=batches.batch_id AND r.state='FAILED_FINAL'),
               unknown_count=(SELECT COUNT(*) FROM records r WHERE r.batch_id=batches.batch_id AND r.state='UNKNOWN');",
        )
        .map_err(map_sql)?;
        refresh_all_batch_states(&tx)?;
        tx.commit().map_err(map_sql)?;
        Ok(true)
    }

    pub fn count_records(&self) -> Result<Counts, Error> {
        let mut counts = Counts::default();
        let mut stmt = self
            .conn
            .prepare("SELECT state, COUNT(*) FROM records GROUP BY state")
            .map_err(map_sql)?;
        let mut rows = stmt.query([]).map_err(map_sql)?;
        while let Some(row) = rows.next().map_err(map_sql)? {
            let state = RecordState::parse(&row.get::<_, String>(0).map_err(map_sql)?)?;
            let n: i64 = row.get(1).map_err(map_sql)?;
            counts.add(state, n);
        }
        Ok(counts)
    }

    pub fn list_batches(
        &self,
        filter: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<BatchInfo>, Error> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT batch_id, state, row_from, row_to, pending_count, in_flight_count,
                        retry_wait_count, succeeded_count, failed_count, unknown_count
                 FROM batches
                 WHERE (?1 = 'all')
                    OR (?1 = 'errors' AND (failed_count > 0 OR unknown_count > 0))
                    OR (?1 = 'running' AND state = 'RUNNING')
                 ORDER BY batch_id
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(map_sql)?;
        let mut rows = stmt
            .query(params![filter, limit, offset])
            .map_err(map_sql)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(map_sql)? {
            out.push(BatchInfo {
                batch_id: row.get(0).map_err(map_sql)?,
                state: row.get(1).map_err(map_sql)?,
                row_from: row.get(2).map_err(map_sql)?,
                row_to: row.get(3).map_err(map_sql)?,
                pending: row.get(4).map_err(map_sql)?,
                in_flight: row.get(5).map_err(map_sql)?,
                retry_wait: row.get(6).map_err(map_sql)?,
                succeeded: row.get(7).map_err(map_sql)?,
                failed: row.get(8).map_err(map_sql)?,
                unknown: row.get(9).map_err(map_sql)?,
            });
        }
        Ok(out)
    }

    pub fn batch_count(&self) -> Result<i64, Error> {
        self.conn
            .query_row("SELECT COUNT(*) FROM batches", [], |row| row.get(0))
            .map_err(map_sql)
    }

    pub fn next_retry_at(&self) -> Result<Option<String>, Error> {
        self.conn
            .query_row(
                "SELECT MIN(next_attempt_at) FROM records WHERE state='RETRY_WAIT'",
                [],
                |row| row.get(0),
            )
            .map_err(map_sql)
    }

    pub fn export_range(
        &self,
        after: i64,
        through: i64,
        limit: usize,
    ) -> Result<Vec<ExportRow>, Error> {
        self.export_query(after, Some(through), limit)
    }

    pub fn export_page(&self, after: i64, limit: usize) -> Result<Vec<ExportRow>, Error> {
        self.export_query(after, None, limit)
    }

    fn export_query(
        &self,
        after: i64,
        through: Option<i64>,
        limit: usize,
    ) -> Result<Vec<ExportRow>, Error> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT row_id, cpf, state, is_customer, checked_at, attempts, http_status,
                        error_code, request_id, evidence_source
                 FROM records
                 WHERE row_id > ?1 AND (?2 IS NULL OR row_id <= ?2)
                 ORDER BY row_id LIMIT ?3",
            )
            .map_err(map_sql)?;
        let mut rows = stmt
            .query(params![after, through, limit as i64])
            .map_err(map_sql)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(map_sql)? {
            let flag: Option<i64> = row.get(3).map_err(map_sql)?;
            out.push(ExportRow {
                row_id: row.get(0).map_err(map_sql)?,
                cpf: row.get(1).map_err(map_sql)?,
                state: row.get(2).map_err(map_sql)?,
                is_customer: flag.map(|v| v != 0),
                checked_at: row.get(4).map_err(map_sql)?,
                attempts: row.get(5).map_err(map_sql)?,
                http_status: row.get(6).map_err(map_sql)?,
                error_code: row.get(7).map_err(map_sql)?,
                request_id: row.get(8).map_err(map_sql)?,
                evidence_source: row.get(9).map_err(map_sql)?,
            });
        }
        Ok(out)
    }

    pub fn push_event(&self, now: &str, kind: &str, detail: &str) -> Result<(), Error> {
        self.conn
            .execute(
                "INSERT INTO events(occurred_at, kind, detail) VALUES (?1, ?2, ?3)",
                params![now, kind, detail],
            )
            .map_err(map_sql)?;
        self.conn
            .execute(
                "DELETE FROM events WHERE event_id NOT IN (
                    SELECT event_id FROM events ORDER BY event_id DESC LIMIT 200
                 )",
                [],
            )
            .map_err(map_sql)?;
        Ok(())
    }

    pub fn recent_events(&self, limit: u32) -> Result<Vec<(String, String, String)>, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT occurred_at, kind, detail FROM events ORDER BY event_id DESC LIMIT ?1")
            .map_err(map_sql)?;
        let mut rows = stmt.query(params![limit]).map_err(map_sql)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(map_sql)? {
            out.push((
                row.get(0).map_err(map_sql)?,
                row.get(1).map_err(map_sql)?,
                row.get(2).map_err(map_sql)?,
            ));
        }
        out.reverse();
        Ok(out)
    }

    pub fn explain_pending(&self) -> Result<String, Error> {
        let mut stmt = self
            .conn
            .prepare(
                "EXPLAIN QUERY PLAN
                 SELECT row_id FROM records WHERE state='PENDING' ORDER BY row_id LIMIT 10",
            )
            .map_err(map_sql)?;
        let mut rows = stmt.query([]).map_err(map_sql)?;
        let mut text = String::new();
        while let Some(row) = rows.next().map_err(map_sql)? {
            let detail: String = row.get(3).map_err(map_sql)?;
            text.push_str(&detail);
            text.push('\n');
        }
        Ok(text)
    }

    pub fn passive_checkpoint(&self) -> Result<(), Error> {
        self.conn
            .execute_batch("PRAGMA wal_checkpoint(PASSIVE);")
            .map_err(map_sql)
    }

    pub fn set_state(&self, state: JobState) -> Result<(), Error> {
        self.meta_set("state", state.as_str())
    }

    pub fn state(&self) -> Result<JobState, Error> {
        let value = self
            .meta_get("state")?
            .unwrap_or_else(|| "PREPARING".into());
        Ok(JobState::parse(&value)?)
    }

    pub fn attempt_rows(&self, row_id: i64) -> Result<i64, Error> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM attempts WHERE row_id=?1",
                params![row_id],
                |row| row.get(0),
            )
            .map_err(map_sql)
    }

    pub fn down_migrate_for_test(&self) -> Result<(), Error> {
        self.conn
            .execute_batch(include_str!("../migrations/0001_down.sql"))
            .map_err(map_sql)?;
        self.conn
            .execute_batch("PRAGMA user_version=0")
            .map_err(map_sql)?;
        Ok(())
    }
}

fn write_checkpoint(tx: &rusqlite::Connection, ckpt: &Checkpoint) -> Result<(), Error> {
    tx.execute(
        "INSERT INTO prepare_checkpoint(id, rows_imported, byte_offset, line_no, record_index, header_done, complete)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
           rows_imported=excluded.rows_imported,
           byte_offset=excluded.byte_offset,
           line_no=excluded.line_no,
           record_index=excluded.record_index,
           header_done=excluded.header_done,
           complete=excluded.complete",
        params![
            ckpt.rows_imported,
            ckpt.byte_offset as i64,
            ckpt.line_no as i64,
            ckpt.record_index as i64,
            if ckpt.header_done { 1 } else { 0 },
            if ckpt.complete { 1 } else { 0 },
        ],
    )
    .map_err(map_sql)?;
    Ok(())
}

fn read_work(row: &rusqlite::Row<'_>) -> Result<WorkItem, Error> {
    Ok(WorkItem {
        row_id: row.get(0).map_err(map_sql)?,
        batch_id: row.get(1).map_err(map_sql)?,
        cpf: row.get(2).map_err(map_sql)?,
        attempts: row.get(3).map_err(map_sql)?,
        attempt_origin: row.get(4).map_err(map_sql)?,
    })
}

fn current_state(conn: &rusqlite::Connection, row_id: i64) -> Result<RecordState, Error> {
    current_state_opt(conn, row_id)?.ok_or_else(|| Error::Msg(format!("missing row {row_id}")))
}

fn current_state_opt(
    conn: &rusqlite::Connection,
    row_id: i64,
) -> Result<Option<RecordState>, Error> {
    let value: Option<String> = conn
        .query_row(
            "SELECT state FROM records WHERE row_id=?1",
            params![row_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(map_sql)?;
    value.map(|v| Ok(RecordState::parse(&v)?)).transpose()
}

fn apply_delta(
    conn: &rusqlite::Connection,
    batch_id: i64,
    from: RecordState,
    to: RecordState,
) -> Result<(), Error> {
    let mut delta = [0_i64; 6];
    delta[state_index(from)] -= 1;
    delta[state_index(to)] += 1;
    conn.execute(
        "UPDATE batches SET
           pending_count=pending_count+?1,
           in_flight_count=in_flight_count+?2,
           retry_wait_count=retry_wait_count+?3,
           succeeded_count=succeeded_count+?4,
           failed_count=failed_count+?5,
           unknown_count=unknown_count+?6
         WHERE batch_id=?7",
        params![delta[0], delta[1], delta[2], delta[3], delta[4], delta[5], batch_id],
    )
    .map_err(map_sql)?;
    let (pending, inflight, retry, failed, unknown): (i64, i64, i64, i64, i64) = conn
        .query_row(
            "SELECT pending_count, in_flight_count, retry_wait_count, failed_count, unknown_count
             FROM batches WHERE batch_id=?1",
            params![batch_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .map_err(map_sql)?;
    let state = BatchState::from_counts(pending, inflight, retry, failed, unknown);
    conn.execute(
        "UPDATE batches SET state=?1 WHERE batch_id=?2",
        params![state.as_str(), batch_id],
    )
    .map_err(map_sql)?;
    Ok(())
}

fn refresh_all_batch_states(conn: &rusqlite::Connection) -> Result<(), Error> {
    let ids: Vec<i64> = {
        let mut stmt = conn
            .prepare("SELECT batch_id FROM batches")
            .map_err(map_sql)?;
        let mut rows = stmt.query([]).map_err(map_sql)?;
        let mut ids = Vec::new();
        while let Some(row) = rows.next().map_err(map_sql)? {
            ids.push(row.get(0).map_err(map_sql)?);
        }
        ids
    };
    for id in ids {
        let (pending, inflight, retry, failed, unknown): (i64, i64, i64, i64, i64) = conn
            .query_row(
                "SELECT pending_count, in_flight_count, retry_wait_count, failed_count, unknown_count
                 FROM batches WHERE batch_id=?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .map_err(map_sql)?;
        let state = BatchState::from_counts(pending, inflight, retry, failed, unknown);
        conn.execute(
            "UPDATE batches SET state=?1 WHERE batch_id=?2",
            params![state.as_str(), id],
        )
        .map_err(map_sql)?;
    }
    Ok(())
}

fn state_index(state: RecordState) -> usize {
    match state {
        RecordState::Pending => 0,
        RecordState::InFlight => 1,
        RecordState::RetryWait => 2,
        RecordState::Succeeded => 3,
        RecordState::FailedFinal => 4,
        RecordState::Unknown => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_claim_uses_partial_index_and_down_migration_drops_tables() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("job.sqlite");
        let db = Db::open(&path).unwrap();
        db.migrate().unwrap();
        let plan = db.explain_pending().unwrap();
        assert!(plan.contains("idx_records_pending"), "plan was:\n{plan}");
        db.down_migrate_for_test().unwrap();
        let version: i32 = db
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 0);
        let missing = db.conn.prepare("SELECT * FROM records");
        assert!(missing.is_err());
    }
}
