# DAT gateway failure diagnosis

2026-10-16 UTC. Tier T3, conservative: authorized read of a real remote DAT,
existing session credentials and bounded transport troubleshooting. No auth,
crypto, policy, dependency or persisted job/config change is planned. The local
scope is one user/session; no service deployment or external message is authorized.

Goal: reproduce the reported HTTP 504 independently of the TUI, identify whether
an existing safe request strategy can deliver and authenticate the DAT, and fix
the client only if evidence identifies a client defect. A remote timeout is not
declared repaired by UI changes or a longer client deadline.

Constraints: never persist DAT/token/secret/key or put them in arguments/env/logs;
use the existing hidden interactive diagnostic with MemorySecrets and locked DAT
buffers. Preserve the user's process, prepared UUID, source CSV and all settings.
No retry storm: initial Single GET has zero transient retries and a 90s budget;
on failure, at most one bounded Range-mode comparison, concurrency one and no
transient retries. Stop on rate limiting. No parallel request fan-out or POST
contract guessing. Any broader code fix requires an updated plan first.

Ordered design consultations (self-applied using the previously read skills):
architect GO: isolated existing Service/diagnostic, no TUI/endpoint redesign.
curator GO: pending job/checkpoints immutable, no incomplete DAT enters matching.
herald GO: current configured endpoint and supported request contracts only.
migrator N/A: no schema, data or settings migration; no user-job creation/deletion.
bastion GO: owned no-echo terminal, provided credentials only to configured HTTPS
origin; temporary metadata only, no durable secrets or downloaded payloads.
scribe GO: ADR-0020 and this plan precede changes; record actual outcomes/limits.
helmsman GO: one request strategy at a time, explicit deadlines, zero transient
retries for diagnosis, no new service-side writes or automatic preference flips.
beacon GO: record safe status/phase/protocol/byte/timing/validator metadata only.

Acceptance: obtain a concrete live status/timing outcome, distinguish remote
504 from token/TLS/body/AES/checksum failure, leave user data/config unchanged,
and either verify a justified local repair or name the unresolved external cause.

Verification commands: existing `target/release/examples/matching_diagnose
TEMP_SETTINGS_JSON single|parallel` with hidden prompts; capture safe diagnostics
only, validate final GCM/envelope/Bloom checks through the real client. No probes
with personal identifiers are required. For a source change: cargo test --offline
--all-targets --all-features; cargo clippy --offline --all-targets --all-features
-- -D warnings; cargo fmt --all -- --check; cargo audit --no-fetch; gitleaks dir
src --no-banner --redact; both debug/release builds. No new tests for docs only.

Rollback: diagnostic temporary metadata is removed; memory is released at exit.
No application rollback is needed without a code/settings change. If code must
change, take a source-only backup before edits and record its rollback command.
No PR, CI, independent review, mutation/coverage floors, signing or production
rollout is claimed. Final report includes the tier DoD with unfulfilled gates.

## Execution and local consultation ledger

Single GET: login 0.734s, fresh-job OAuth succeeded, HTTP 504 at 60.860s.
Range probe bytes=0-0: login 0.523s, fresh-job OAuth succeeded, HTTP 504 at
60.963s. No parts started; no further requests were made. Both sessions exited
and temporary metadata was removed. See dat-gateway-verification.md.

forge N/A implementation: no evidence justified changing application code.
refactory N/A: no restructure. sentinel GO diagnostic evidence only: existing
release example compiled; two live negative-path checks reproduced the failure.
helmsman GO: sequential, bounded, zero transient retries; no request amplification.
bastion GO local custody: echo-off verified; supplied credentials used only in
isolated RAM; no DAT, token or secret persisted; no Keychain/user data changes.
beacon GO: safe timing/status metadata recorded, no payload/error-body logging.
inquisitor GO diagnosis, origin repair pending: the TUI is not the source of
the reproduced 504; a 1-byte probe failed too. Backend/gateway logs are required
to identify the slow upstream. No speculative workaround is approved.
scribe GO: measured claims, limits, follow-up and DoD written in verification.
conductor N/A publishing: docs-only investigation; no PR/commit/service rollout.
Consultations are self-applied, not independent reviews or production approvals.

## Exact-URL clarification — 2026-10-16

User confirms precisely two endpoints: common token acquisition/refresh and
encrypted DAT download. The word service referred to the server handling DAT,
not an additional endpoint. New input includes a trailing slash on the DAT path;
the saved URL and prior two checks used the non-trailing form. Do not assume
that these paths route identically.

Tier remains T3 for the bounded authorized live read; no application-source,
auth, crypto, endpoint-count or concurrency change. Ordered self-applied design
verdicts: architect GO existing isolated diagnostic; curator GO preserve job and
RAM-only DAT; herald GO exact supplied URL, same token/Bearer contract; migrator
N/A no migration; bastion GO hidden input and unchanged HTTPS host; scribe GO
this amendment before execution; helmsman GO one additional Single GET, 90s
deadline, zero transient retries, stop on 429; beacon GO safe status/time only.

Plan: copy non-secret external metadata to a private temporary file, change only
the DAT URL there to the supplied trailing-slash path and set the diagnostic
deadline/retries. Run the existing `target/release/examples/matching_diagnose
TEMP_SETTINGS_JSON single` with verified no-echo inputs. If successful, verify
the envelope/GCM/Bloom in RAM and release it without identifier probes. Record
the observed outcome without attributing success solely to the slash. Preserve
real settings/job and delete temporary metadata; no speculative path fallback.

Acceptance/rollback: one concrete result for the exact supplied URL; no saved
state or software change; remove diagnostic metadata and release session memory.
No full source test run is necessary for this docs-only follow-up; live outcome,
cleanup and state hashes are the checks. Existing ADR-0020 remains applicable.

Outcome: exact trailing-slash URL returned HTTP 504 at 60.872s; login 0.604s,
fresh-job OAuth successful. No DAT reached AES validation. The saved-settings
hash matches; no job metadata was present in the baseline inventory.
Temporary metadata was removed; no application change. Local
review GO diagnosis only; origin correction remains pending. See verification.
