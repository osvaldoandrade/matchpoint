# Token endpoint/session — local verification

2026-10-15, T4 authentication correction. [Plan](token-session-plan.md), [ADR-0011](adr/0011-token-session.md), [threat delta](threat-models/0006-token-session.md). Local implementation only; no publishing, production credentials, real issuer or native Keychain operation.

Both grants now POST to token_url; only grant_type/payload differs. Deprecated reset_token_url is accepted on read, ignored in routing/profile identity and omitted from saved settings. The hidden compatibility flag rejects distinct destinations before persistence. Profiles previously using one common URL keep their normalized hash; distinct historical profiles require fresh login. No automatic credential migration or job/filter changes.

Settings API edits DAT/scope/limits. Secrets contains read-only session/token metadata and Renew token. It has no client-ID/secret/refresh editor and no credential save path. A save guard prevents changed login ID/URL even if draft state is manipulated. Home remains the only interactive credential editor, after sign out. Login success requires a validated issued access token/Bearer type/expiry and successful secret-store commit; saved secrets, empty 2xx, denial and cancellation never unlock.

| Verification | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo test --offline --all-targets --all-features` | PASS, 172 tests |
| `cargo clippy --offline --all-targets --all-features -- -D warnings` | PASS |
| `cargo build --offline --release --bin pgm-data` | PASS |
| `cargo audit --no-fetch` | PASS using installed advisory DB, 392 dependencies |
| `gitleaks detect --no-git --redact --source src` | PASS |
| `cargo run --offline --example matching_review -- target/token-session-review` | PASS, synthetic four-size views |
| Owned release PTY `/tmp/pgm-token-session-pty.py` | PASS, 16 assertions at 80x24/40x18, no credential submission |
| Protected art/animation/motion/splash/palette/runtime SHA-256 | All seven files unchanged |

New behavioral coverage: acquisition and rotating refresh at the same endpoint, expiry/401/invalid_grant there; ignored legacy destination; no reset URL serialized; distinct CLI destination rejected without saving; HTTP success missing/empty/whitespace token, wrong type or invalid expiry rejected; end-to-end UI 200-without-token stays locked; no authenticated credential editors or paste/key mutation; manipulated login identity/URL cannot be saved. Existing cancellation, storage failure, profile isolation, no automatic login, ordered local matching and recovery regressions remain green.

CSV output: `test-data/cpfs_sha256_1m.csv`, header `cpf_sha256`, 1,000,000 unique records, 65,000,011 bytes, UTF-8/LF. Lowercase SHA-256 hexadecimal of unique synthetic check-digit-valid 11-digit numbers, no punctuation/salt/newline. File SHA-256: `f26cd8d076a6f5971e46a557c7d065991b8470cb9b921783df0c4bf1cff9732d`. Artifact Tool authored bounded chunks; a standard Node writer streamed the requested CSV without an unsolicited XLSX. Independently parsed all rows, recomputed every hash, checked CPF check digits with a second formula and verified uniqueness/count/size/checksum. Metadata/validation receipts and preview are beside the CSV.

Release round trip: built a DAT from all million hash strings with transformation none, downloaded once from a local loopback fixture, matched all 1,000,000 rows and compared each output with its original key. 1,000,000 true, zero false; exactly one HTTP GET and no per-key calls. This proves membership for a filter populated with these same synthetic hashes, not positive/negative discrimination against an external dataset. Use key cpf_sha256 and transformation None with a DAT built for those already-hashed keys; do not apply SHA-256 a second time. Pinned transformation compatibility still applies.

Evidence: `target/token-session-verification/` logs, dataset-match.json and hash receipt; `target/token-session-pty/` screen captures/checks; `target/token-session-review/`; `test-data/cpfs_sha256_1m.{metadata,validation}.json`. Synthetic render/sample validation is separate from native Ghostty/provider acceptance.

Review ledger, self-applied in order: sentinel GO for measured local behavior, helmsman/pathfinder GO for existing deadline/retry/join/lock behavior, bastion GO for one destination/credential gating/redaction/scans, beacon GO for safe existing auth events/status, inquisitor GO for authorized scope, scribe GO for current README/ADR/runbook/threat notes, conductor records no .git and no publication. No independent approvals are claimed.

DoD local: [x] classification/plan/ADR/threat/runbook, [x] four requested outcomes, [x] behavior regressions, [x] lint/build/scans, [x] million-row independent validation and release round trip, [x] animation preservation/rollback record. T4 publication gates (coverage/mutation floors, independent security review, CI/PR, signed SBOM, staging rollback, production canary/observability and human sign-offs) remain unverified and block publication. No release is requested or performed. Restore only task snapshot paths from /tmp/pgm-token-session-backup-path after reconciling concurrent edits; preserve all user settings/Keychain/jobs/filter pins and generated dataset. Rollback details are in ADR-0011.
