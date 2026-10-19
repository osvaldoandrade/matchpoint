//! Persistent CPU workers. One bounded wave in flight; only the coordinator writes.
use super::{bad, format::Bloom, Rule};
use crate::error::Error;
use std::{
    sync::{
        mpsc::{self, Receiver, SyncSender},
        Arc,
    },
    thread::JoinHandle,
};

/// At most 8192 original keys (each <=4096 bytes) are retained per wave.
pub(super) const WAVE_ROWS: usize = 8192;
pub(super) struct Row {
    pub key: String,
    pub position: csv::Position,
    pub matched: bool,
}
type Reply = (usize, Result<Vec<Row>, Error>);
type Request = (usize, Vec<Row>);
pub(super) struct Pool {
    input: Vec<SyncSender<Request>>,
    results: Receiver<Reply>,
    handles: Vec<JoinHandle<()>>,
}
impl Pool {
    pub fn new(threads: u16, bloom: Arc<Bloom>, rule: Rule) -> Result<Self, Error> {
        super::validate_threads(threads)?;
        // One reply per worker per wave: bounded even if the coordinator fails early.
        let (reply, results) = mpsc::sync_channel(usize::from(threads));
        let mut pool = Self {
            input: Vec::new(),
            results,
            handles: Vec::new(),
        };
        for n in 0..threads {
            let (tx, rx) = mpsc::sync_channel::<Request>(1);
            let reply = reply.clone();
            let bloom = Arc::clone(&bloom);
            let rule = rule.clone();
            // Worker exits when input closes or reply receiver disappears. Drop always joins.
            let handle = std::thread::Builder::new()
                .name(format!("match-{n}"))
                .spawn(move || {
                    while let Ok((seq, mut rows)) = rx.recv() {
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            for row in &mut rows {
                                row.matched = bloom.contains_with_rule(&row.key, &rule)?;
                            }
                            Ok(rows)
                        }))
                        .unwrap_or_else(|_| {
                            Err(bad("Matching worker panicked; no results published."))
                        });
                        if reply.send((seq, result)).is_err() {
                            break;
                        }
                    }
                })
                .map_err(|_| bad("Cannot start matching threads."))?;
            pool.input.push(tx);
            pool.handles.push(handle);
        }
        Ok(pool)
    }
    pub fn evaluate(&self, rows: Vec<Row>) -> Result<Vec<Vec<Row>>, Error> {
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let chunk = rows.len().div_ceil(self.input.len());
        let mut rows = rows.into_iter();
        let mut count = 0;
        for (seq, tx) in self.input.iter().enumerate() {
            let batch = rows.by_ref().take(chunk).collect::<Vec<_>>();
            if batch.is_empty() {
                break;
            }
            tx.send((seq, batch))
                .map_err(|_| bad("Matching worker stopped."))?;
            count += 1;
        }
        let mut ordered = (0..count).map(|_| None).collect::<Vec<_>>();
        for _ in 0..count {
            let (seq, result) = self
                .results
                .recv()
                .map_err(|_| bad("Matching result channel closed."))?;
            ordered[seq] = Some(result);
        }
        ordered
            .into_iter()
            .map(|rows| rows.ok_or_else(|| bad("Missing matching chunk."))?)
            .collect()
    }
}
impl Drop for Pool {
    fn drop(&mut self) {
        // Closing all inputs wakes idle workers. In-flight replies fit in the capacity-N queue.
        self.input.clear();
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workers_share_one_bloom_and_all_references_leave_on_drop() {
        let mut bloom = Bloom::new(1, &Rule::default()).unwrap();
        bloom.insert("000123").unwrap();
        let bloom = Arc::new(bloom);
        let pool = Pool::new(4, Arc::clone(&bloom), Rule::default()).unwrap();
        assert_eq!(Arc::strong_count(&bloom), 5);
        let rows = ["000123", "absent", "000123"]
            .into_iter()
            .map(|key| Row {
                key: key.into(),
                position: csv::Position::new(),
                matched: false,
            })
            .collect();
        let result = pool.evaluate(rows).unwrap();
        assert_eq!(
            result
                .into_iter()
                .flatten()
                .map(|r| r.matched)
                .collect::<Vec<_>>(),
            vec![true, false, true]
        );
        drop(pool);
        assert_eq!(Arc::strong_count(&bloom), 1);
        assert!(Pool::new(0, Arc::clone(&bloom), Rule::default()).is_err());
        assert!(Pool::new(257, bloom, Rule::default()).is_err());
    }
    #[test]
    fn drop_joins_full_reply_queue_without_a_reader() {
        let bloom = Arc::new(Bloom::new(1, &Rule::default()).unwrap());
        let pool = Pool::new(3, bloom, Rule::default()).unwrap();
        for (seq, tx) in pool.input.iter().enumerate() {
            tx.send((
                seq,
                vec![Row {
                    key: "absent".into(),
                    position: csv::Position::new(),
                    matched: false,
                }],
            ))
            .ok()
            .unwrap();
        }
        drop(pool);
    }
}
