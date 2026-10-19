# Live operation progress — local verification

2026-10-16 UTC. The local client now displays the actual operation phase instead
of the fixed Working locally notice. The observed prepared job had processed
0/20,000,000 rows; OAuth succeeded and the DAT gateway returned HTTP 504 after
about 60 seconds, then retried. The UI hid that remote wait. This change exposes
the wait and progress; it does not repair the provider's gateway timeout.

## Behavior and scope

One operation-scoped, latest-value watch snapshot carries typed phase/UUID,
numeric quantities, HTTP status and monotonic timestamps. No row values, paths,
URLs, credentials, keys, payloads or raw request errors enter this channel.
The UI clones the snapshot without holding a borrow over await and renders on
its existing 50 ms timer. There is no additional worker, queue or API request.

The bordered LIVE ACTIVITY panel shows copy/scan, token acquisition, header wait,
download/retry, decryption/Bloom validation, input/checkpoint validation, matching
and publication. Header wait advances a clock without data events, retains the
last HTTP failure and shows the shared retry budget. Body recovery and parallel
parts count unique logical bytes; discarded prefixes subtract progress and the
one-byte Range probe is excluded. A body interruption is distinguished from an
older HTTP failure. Unknown totals do not invent a percentage. Matching updates
every 8192-row wave; durable checkpoints keep their configured interval.

Stopping overrides live feedback. Completion/failure removes stale progress.
Prepared-job retry preserves its UUID. At narrow sizes the dataset summary keeps
its name/state visible below the panel instead of showing a cropped chart.
The manual, README and recovery runbook describe this behavior.

## Local validation

Commands (all from the project root):

| Command | Result |
| --- | --- |
| `cargo test --offline --all-targets --all-features` | 246 passed; 0 failed |
| `cargo clippy --offline --all-targets --all-features -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo audit --no-fetch` | No vulnerabilities reported in the cached advisory database; 393 dependencies |
| `gitleaks dir src --no-banner --redact` | No leaks found |
| `cargo run --offline --example matching_review -- target/live-progress-review` | Four progress states at 40×18, 60×22, 80×24 and 120×36 |
| `cargo build --offline --release --bin pgm-data --bin pgm-bench` | Passed |
| `cargo build --offline --bin pgm-data` | Passed; launcher binary updated |

New regression cases exercise a real loopback 504 followed by gated response
headers and a half-delivered body. They assert that wait/status/attempt and live
byte counts are visible while the HTTP operation is still pending. Other tests
cover latest-value observation/isolation, clocks/rates/unknown totals, parallel
byte aggregation, suffix recovery, full restart, ordered 20,000-row pause/resume,
checksum verification, narrow rendering and cancellation/completion cleanup.
Existing AES-GCM/checksum, RAM lifetime, auth, duplicate-job and recovery suites
remain green. Snapshot rates/times are synthetic UI examples, not benchmarks.

Logs are `/tmp/pgm-live-progress-{all-tests,clippy,fmt,audit,secrets,render,release,debug}.log`.
Render artifacts are under `target/live-progress-review/`.

## Boundaries and rollback

The user's active terminal process was not killed or restarted. Reopen the app
to use the rebuilt binary; after signing in, Jobs → R retries the same prepared
job. An unfinished operation can be stopped with P before quitting. A repeated
HTTP 504 still requires provider/gateway repair.

No preferences, threads, jobs/checkpoints, credentials, original datasets or
exports were changed by this task. The 20M-row CSV remains 240,000,004 bytes.
Cargo.toml/Cargo.lock and all seven protected animation/font/palette files match
the pre-task hashes. The observer is optional for existing CLI/library callers.
There is no crypto, endpoint, deadline, retry-policy or persistence migration.

Local functional/build checks are complete. CI, independent review, coverage and
mutation floors, native Ghostty acceptance, signing, canary and production rollout
were not performed or claimed. No PR/commit/release was requested or published.
Rollback restores only task source/docs from the backup recorded in
`/tmp/pgm-live-progress-backup-path`, then rebuilds both binaries. Preserve user
data/settings and running-session state.
