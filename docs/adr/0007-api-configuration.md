# ADR 0007 — Configure the API before sending remote requests

- Status: accepted for local implementation, 2026-10-13 UTC
- Owner: pgm-data operator

## Context

The operator requested a colored terminal interface, Clap commands for API configuration, and batches of 1,000 records instead of 10,000. The operator confirmed the existing PicPay process and OAuth URLs on 2026-10-13. No production request, deployment, release, or credential rotation is part of this change. Response semantics remain unspecified.

## Decision

Keep the existing single-process boundaries: Clap and Ratatui call `App`; `App` validates and persists non-secret settings; Keychain owns credentials; the HTTP adapter interprets responses. Add an API configuration form (F3) and additive `auth configure` options. New jobs inherit the saved URLs and client ID. Existing job configurations and SQLite schema remain unchanged. Each new default batch holds at most 1,000 input records; retries remain attempts of the original record, and the last batch may be smaller.

Remote access defaults to disabled, including when loading old configuration files. The operator explicitly enables it using `--allow-remote` or the labeled form checkbox. A remote job must match the saved process URL, token URL, and client ID exactly, use HTTPS, and use the pending contract. Test mode and the simulated contract remain restricted to loopback. Refuse URL userinfo, queries, fragments, and control characters. Do not follow redirects. Retain timeout, concurrency, rate-limit, and circuit-breaker limits.

HTTP 2xx still yields `UNKNOWN` with a null customer flag. A pending-contract 5xx also yields `UNKNOWN` rather than automatically repeating a potentially accepted POST. This supersedes ADR 0002 only for explicit remote access; its evidence and merge rules still apply.

## Alternatives

1. Accept a client secret as a CLI argument. Rejected: process listings and shell history expose it.
2. Permit arbitrary job URLs to use stored credentials. Rejected: overrides could redirect credentials to an unconfigured destination.
3. Interpret every HTTP 2xx as a customer. Rejected: the provider response contract is absent.

## Verification and rollout

Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-targets --all-features`, `cargo build --release`, and a synthetic local load run. Test persistence, old configuration compatibility, destination and identity checks, masked/cancelled form input, and 999/1,000/1,001/2,001-row batch boundaries. The TUI is the local dashboard; persistent job counters and events remain the evidence source. Coverage, mutation, signing, independent review, and production rollout results must be recorded as measured or pending, never assumed.

Before any production rollout, the operator validates the provider contract, uses synthetic permitted accounts, and stages 1% → 10% → 50% → 100% of the intended volume with at least 24 hours between data-plane stages. Abort on unexpected customer interpretation, credential/destination mismatch, or ambiguous outcomes. This task does not execute that rollout.

## Rollback

Disable remote access with `pgm-data auth configure --disable-remote` (or F3), then interrupt any active job with Ctrl+C and allow its 15-second drain. New remote runs fail closed. Restore source files from the task backup and rebuild to reverse the local implementation; keep existing job directories. Provider-side effects cannot be reversed by restoring local files.

