# Explicit API URLs and refresh-token renewal — local handoff

Implemented 2026-10-15. Classified T3 for URL configuration, then **T4 before auth implementation** after the user confirmed that reset means refresh-token renewal. [Plan](api-urls-plan.md), [ADR-0009](adr/0009-explicit-api-urls.md), [threat model](threat-models/0004-api-urls-oauth.md).

The active application independently configures `token_url`, `reset_token_url` and `dat_url`, with empty defaults. API settings also expose client ID, optional scope and limits. Initial JSON `client_credentials` goes to the acquisition URL, JSON `refresh_token` goes to the reset URL, and bearer-authenticated GET goes to the DAT URL. No endpoint paths are inferred. The existing provider JSON grant contract is preserved; real-provider acceptance is not asserted.

Secrets selects Public / Saved bearer / OAuth2, hides all credential inputs, and offers Get token / Reset token actions. CLI exposes `bloom configure --token-url ... --reset-token-url ... --dat-url ... --client-id ... --scope ... --oauth`, hidden `auth credentials`, `auth get-token`, `auth refresh-token`; slash actions `/token` and `/refresh`. Generated access tokens stay in RAM; client secret and returned refresh tokens use profile-scoped Keychain items. Mode remains opt-in; the old manual bearer path and old `url` JSON / `--url` aliases work.

Expiry renews automatically. DAT 401 allows one renewal and one GET retry; another 401 stops. Invalid refresh grant allows one acquisition fallback. A refresh response without a replacement refresh token preserves the existing one. No revocation operation, per-key request or new dependency was introduced. Matching/PGMBLOOM/output algorithms were unchanged.

## Files and compatibility

- `src/matching/{mod,auth,source}.rs`: external settings, normalized full-profile credential/cache identity, bounded JSON grants, token cache, serialization/process lock, cancellation, refresh rotation, bounded retry/cooldown and DAT authentication.
- `src/matching/{cli,ui}.rs`, `src/matching/ui/view.rs`: API URL editors, metadata/auth modes, hidden secret editors, button/CLI/slash routing and safe status messages. Loaded filters are invalidated after profile changes; saved manual bearer replacement locks the source and invalidates the current cache pointer. Pinned jobs remain reproducible.
- `src/auth.rs`, `src/engine{,/configuration}.rs`: remove fixed provider URL values. Historical constants remain empty compatibility names. Disabled historical profiles can be saved as drafts and local CSV preparation can proceed without URLs; remote execution/enabling still requires explicit destinations and authorization. Existing saved profiles were not rewritten.
- `tests/matching.rs`, new `tests/matching_auth.rs`, matching UI tests and narrow legacy fixture updates: explicit test endpoints instead of relying on provider defaults, plus regression coverage described below.
- `README.md`, ADR/plan/threat/runbook: current setup, JSON contract and recovery. No user settings, live endpoints, production data or real Keychain items were accessed or changed.

The snapshot/changed paths and final binary/source hashes are in the [receipt](../target/api-urls-verification/receipt.json). There is no `.git` checkout; no commit, PR, publication or deployment occurred.

## Verification

| Command / evidence | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo check --offline --all-targets` | PASS |
| `cargo test --offline --all-targets --all-features` | PASS, **157 tests** (91 library, 65 integration, 1 example); [log](../target/api-urls-verification/tests.log) |
| `cargo clippy --offline --all-targets --all-features -- -D warnings` | PASS, no warnings |
| `cargo build --offline --release --bin pgm-data` | PASS |
| `cargo audit --no-fetch` | PASS, 392 dependencies, zero advisories in the previously refreshed database |
| `gitleaks detect --no-git --redact --source src` and same for `tests` | PASS, no leaks; reports in `target/api-urls-verification/` |
| Scan actual release binary for fixed `https://api.picpay.com` | PASS, absent |
| Active release binary in an owned synthetic PTY | PASS, 12 checks: three API URL fields, three masked secrets, DAT/New Job/export/checksum, error draft recovery, slash navigation, narrow exit, one GET and clean logs; [receipt](../target/api-urls-pty/checks.json) |
| `cargo run --offline --example matching_review -- target/api-urls-review` | PASS, four-size TestBackend/SVG/VGA render evidence; [API screen](../target/api-urls-review/settings-80x24.png) |

New tests verify independent URL persistence/canonical alias migration, URL policy/secret-free errors, CLI preservation of unspecified values, narrow editor scrolling, actual UI Get/Reset actions with MemorySecrets, issue/refresh/DAT routing and grant payloads, cache reuse across Service clones, expiry, concurrent refresh rotation, omitted replacement refresh token, invalid_grant fallback, retry cap, malformed/oversized/redirect/rejected token responses, endpoint/client/scope cache isolation, cancellation/retry and explicit refusal of historical remote runs with empty defaults. Existing matching/no-per-key HTTP suites still pass.

All auth traffic used loopback fixtures, and all test credentials were synthetic. The executable PTY checks used public-DAT mode and discarded secret drafts; real Keychain integration and real-provider/native Ghostty acceptance were not exercised. No performance rerun was required: dataset matching and its checkpoint algorithm were unchanged by this task; earlier matching measurements remain historical evidence, not a new OAuth benchmark.

## Ordered final review and DoD

Self-applied skills, not independent reviewers: code-workflow; architect → curator → herald → migrator → bastion → sentry → helmsman → beacon → scribe → pathfinder → forge/refactory. Final sentinel (local behavioral suites GO), helmsman (bounded calls, cancellation/locks GO), bastion (destination binding, secrecy, limits/scans GO), beacon (safe auth lifecycle/status GO), inquisitor (local scope/test/source review complete), scribe (docs current), conductor (local handoff only). Inquisitor/conductor do **not** approve merge/release without the unmeasured independent review, coverage/mutation, CI and production gates.

- [x] Written classification/plan/ADR preceded implementation; scope reclassified before adding active OAuth.
- [x] Three configured destinations, initial acquisition, refresh renewal, DAT authorization and settings/CLI/GUI controls implemented.
- [x] Compatibility, negative paths, secrecy, cancellation and local suites verified.
- [x] Snapshot and non-destructive rollback direction recorded; no production changes.
- [ ] Full T4 publishing gates (independent/security reviews, coverage/mutation floors, live provider, staged rollout/signing) are not claimed or performed.

Rollback: compare receipt fingerprints before restoring files from `/var/folders/14/52y2hz4j46s7_h04n92vcrx00000gn/T/pgm-api-urls-before-u1ccnla4`, preserving any newer user edits; remove only unchanged newly introduced auth source/test files and rebuild. Preserve all data/filter/job/Keychain items. For an older matching binary, translate `dat_url` back to `url` and remove `token_url`, `reset_token_url`, `client_id`, `scope`, `oauth` in a backup copy of settings; keep the user's original. No SQL/backfill or credential deletion is required. Real API URL values and provider acceptance remain user configuration/integration work; no release approval was requested.
