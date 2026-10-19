# ADR-0019: operation-scoped live progress

Accepted for local implementation, 2026-10-16. Tier T3.

Context: the UI says Working locally while the DAT endpoint waits a minute then
returns 504 and retries. A prepared 20M-row job remains at zero, with no visible
distinction between dataset preparation, server wait, transfer and matching.

Decision: a scoped clone of Service owns an optional watch sender with one latest
progress snapshot. The corresponding UI worker reads snapshots on its existing
timer. Numeric/enumerated observations carry phase, done/known total, retry count,
HTTP status, clocks and optional typed UUID. No payload/credential/URL free text.
Update copy per chunk, scan/match per bounded wave and download per received chunk.
Parallel/recovery counters reflect logical bytes, not repeated/probe bytes.
Show a bordered activity panel and meaningful compact status even before a job
exists. Clear stale observers on complete/fail; stop feedback takes precedence.

Alternatives: tailing event logs is delayed and couples UI to storage; persisting
every progress update adds hot-path disk writes; a fake spinner or timer without
real stages still mislabels server waits; an unbounded event queue can accumulate.

Consequences: optional observer overhead is bounded; CLI behavior is unchanged
when no observer is attached. No new pool, per-row event stream or network request.
No auth/crypto/deadline/checkpoint semantics change. A server 504 remains external.

Rollback: restore source/docs and rebuild; no data/config rollback or migration.
Publication gates remain separate from local tests; no independent sign-off or
production rollout is implied. Verification is in live-progress-verification.md.
