# Large DAT body interruption repair

2026-10-16 UTC. Tier T3: bounded transport reliability change on an existing
external read path; no auth/crypto checks, schema, dependency, pool or endpoint
change. Scope is the local client. No server deployment or external message.

Goal: diagnose and mitigate a DAT that begins downloading, loses speed and then
breaks. Current logs show HTTP 200/206 over HTTP/2.0, repeated http2_stream_error,
two exact Range resumes and budget exhaustion. Single GET ended at 26,563,549 of
169,768,248 bytes; a later 4-way Range run also reset part streams. No HTTP 429
was recorded in these attempts. Rate limiting remains a hypothesis, not a fact.

Ordered self-applied design consultations:
architect GO: isolate HTTP policy in DAT transport; OAuth client remains intact.
curator GO: keep strong ETag/exact suffix/ordered assembly; incomplete DAT never
reaches AES, checksum or matching. Session/job ownership and RAM policy stay.
herald GO: same configured GET/POST contracts; no endpoint or saved-settings wire
change. HTTP/1.1 is a transport choice, not a new API or crypto format.
migrator N/A: no persisted data/settings/job migration or automatic preference edit.
bastion GO: preserve verified TLS, no redirects, sensitive Bearer header, hidden
inputs and locked/wiped DAT buffers. HTTP version change is not a TLS downgrade.
scribe GO: ADR-0021, this plan and source-only backup precede the experiment.
helmsman GO: test one HTTP/1.1 stream with zero transient retries and 120s deadline;
stop on 429, no parallel remote fan-out or speculative increased retry count.
beacon GO: actual response protocol/bytes/time and existing static errors identify
the stage; add bounded progress timing if needed, never payload/raw errors.
pathfinder GO: retain current bounded part futures, budget and cancellation; no
new worker/pool/lock. Preserve configured user parallelism rather than flip it.

Plan: introduce a DAT-only HTTP/1.1 client retaining TLS/redirect/connect/deadline
bounds. First run existing transport regressions and an isolated authorized Single
GET using this client (hidden inputs, temporary non-secret metadata, MemorySecrets,
no user job or identifier probes). Validate full PGMENC01/GCM/Bloom before claiming
success. If it fails, record that result and reassess rather than claim a fix.
Keep OAuth negotiation unchanged. Prefer an explicit small DAT builder over
changing the shared auth client, to avoid altering a working login path.

Then verify interruption/resume and bounded parallel assembly locally; add only
regressions that test a new observable behavior. Update shared manual/README/
runbook with protocol/recovery limits and actual evidence. Existing strategy,
part sizes, retries, rate-limit handling and deadlines are preserved.

Acceptance: protocol observed as HTTP/1.1; successful current full encrypted DAT
validation if the provider is available; no hidden restart inflation, incomplete
snapshot, extra request fan-out or durable payload/credential. Never claim that
one successful download proves absence of provider throttling.

Verification: cargo test --offline --all-targets --all-features;
cargo clippy --offline --all-targets --all-features -- -D warnings;
cargo fmt --all -- --check; cargo audit --no-fetch;
gitleaks dir src --no-banner --redact;
cargo build --offline --release --bin pgm-data --bin pgm-bench;
cargo build --offline --bin pgm-data; cargo build --offline --release --example
matching_diagnose; owned hidden-input diagnostic, Single GET, zero transient
retries. Preserve/check protected animation hashes and dependency files.

Rollback: restore only task source/docs from /tmp/pgm-http-body-backup-path and
rebuild. Preserve user process, preferences, jobs, datasets, exports and secrets.
No CI, independent approval, mutation/coverage floors, production rollout or
server-side rate-limit repair is claimed by local tests.

## Execution ledger

forge GO: DAT-only HTTP/1.1 builder with existing transport bounds, OAuth unchanged.
refactory GO scope: no pool/client/auth redesign; distinct protocol policy boundary.
sentinel GO local functionality: 21 focused transport tests and 246 all-target
tests; real protocol/full authenticated payload check, no mirror constructor test.
helmsman GO: one completed real DAT stream without transient retries, existing
fault-injection/429/deadline/ordered-recovery tests; no provider quota inference.
bastion GO local: no TLS/auth/checksum/AEAD weakening, no payload persistence;
hidden inputs and locked RAM, source scan and cached audit pass.
beacon GO: protocol/status/byte/timing evidence separates header and body failures.
inquisitor GO local functional scope: one successful HTTP/1.1 transfer supports
this mitigation, not a claim about HTTP/2/provider internals or availability SLO.
scribe GO: measured result, first auth-only failure, limitations, help and DoD
recorded in http-body-verification.md. conductor GO local builds only; no publish.
These are self-applied consultations, not independent reviews or merge approval.
