# Home login — local verification

2026-10-15. User-requested interactive behavior is implemented locally. Tier T4 (login/secret check); [plan](home-login-plan.md), [ADR-0010](adr/0010-home-login.md), [threat model](threat-models/0005-home-login.md). No release, commit, production endpoint or native Ghostty session was exercised.

Home opens directly to three bordered fields: Client ID, masked Client secret and Token URL, plus Sign in. Client ID/URL can be prefilled; the secret is always blank on launch. A successful provider grant unlocks the dashboard. Public/manual bearer modes remain CLI-only. F2 opens setup before login; Tab/Enter navigate; Esc cancels an active login or quits an idle login; Ctrl+C stops safely. L signs out locally. Profile edits and credential replacements relock; appearance/rule edits preserve the session.

The acquisition adapter accepts an incomplete DAT/refresh profile, without inventing destinations. Existing complete profile hashes and JSON grant payloads remain compatible. Rejected fresh credentials do not overwrite valid stored credentials. A successful login stores client/refresh secrets in the existing Keychain namespace; access stays in RAM. Cancellation and storage failures remain locked, with the secret editor cleared. Secret preparation/persistence stays off the terminal input loop; the worker joins preparation on cancellation. Sign out preserves monotonic token generations.

| Command / check | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo test --offline --all-targets --all-features` | PASS, 168 tests, zero failures |
| `cargo clippy --offline --all-targets --all-features -- -D warnings` | PASS |
| `cargo build --offline --release --bin pgm-data` | PASS |
| `cargo audit --no-fetch` | PASS against installed advisory database; 392 dependencies |
| `gitleaks detect --no-git --redact --source src` | PASS, no leaks |
| `cargo run --offline --example matching_review -- target/home-login-review` | PASS, synthetic login/error/busy/dashboard/forms at four sizes |
| `python3 /tmp/pgm-home-login-pty.py` | PASS, 16 real-release PTY assertions at 80x24 and 40x18 |

Regression coverage: launch without splash; each field visible when focused at 40x18/60x22/80x24/120x36; fixed secret masking; missing fields and rejected URLs; saved credentials/token cannot unlock a new session; protected navigation and slash commands cannot bypass; valid login using only the requested three fields; busy input ignored; denial/cancel/retry; credential-store failure; profile change versus appearance/rules; sign out; generation preservation; prior secret preserved after a rejected login; unchanged local matching/OAuth renewal/isolation suites.

Test API traffic used loopback fixtures and MemorySecrets only. The release PTY checks exercised Home, masked paste, URL input, setup, blocked New Job and safe quit without submitting credentials. No synthetic secret appeared in the ANSI output, and no credential settings were written by those PTY runs. Screenshots use a synthetic TestBackend and bundled font, not native Ghostty acceptance.

Evidence: `target/home-login-verification/{tests,clippy,build,audit,secrets-scan}.log`, source/release hashes and result counts in `target/home-login-verification/receipt.json`; `target/home-login-pty/checks.json` and screen captures; `target/home-login-review/*.{txt,svg,png}`.

Ordered local review ledger: sentinel GO for tested local behavior; helmsman/pathfinder GO for existing bounded worker/deadlines/cancel/join and no POST retry; bastion GO for masked input, destination/profile isolation, successful-grant-only replacement, safe errors and scans; beacon GO for safe OAuth lifecycle events and visible login status; inquisitor GO for authorized local scope; scribe GO for README/ADR/threat/recovery updates; conductor records no .git and no publication request. These are self-applied reviews, not independent approvals.

Local DoD: [x] classification/design/ADR/threat/runbook, [x] requested behavior, [x] meaningful regressions, [x] lint/build/scans, [x] synthetic visual inspection and actual release PTY, [x] rollback evidence. Publication DoD: coverage and mutation thresholds, independent reviewers/security sign-off, signed SBOM, CI/PR, staging rollback, production ramps/observability and on-call/executive sign-off remain unverified; no publication is claimed or performed. No dependencies, parser, wire format, dataset processing or job schema changed.

Rollback uses the task snapshot pointed to by `/tmp/pgm-home-login-backup-path`; reconcile concurrent edits before restoring only task-owned source/docs. The test-file baseline is reconstructed by excluding exactly the three added auth tests; preserve any unrelated test changes. Keep user settings, Keychain, snapshots and job/results intact. This local UI login is a workflow gate, not an OS security boundary on local files or CLI automation.
