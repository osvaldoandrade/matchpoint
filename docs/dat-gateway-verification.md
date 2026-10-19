# DAT HTTP 504 — live diagnosis

2026-10-16, 05:00–05:03 UTC. Two authorized, isolated checks reproduced the
provider/gateway timeout before a DAT could be delivered. Both OAuth sessions
succeeded in under one second. The second check requested only byte 0; it failed
at the same roughly 60-second boundary as the full GET. No client defect was
identified, so application source, saved preferences and jobs were not changed.

## Evidence

Current external configuration points to the expected consumer-filter endpoint.
The user configuration remains Single GET, total deadline 600s, body idle 60s,
two transient retries. The prepared UUID remains at 0/20,000,000 rows, one worker.

For each diagnostic, an isolated Service used MemorySecrets and the existing
hidden-input flow with echo disabled and verified before each credential entry.
Only temporary, non-secret settings metadata was created. The diagnostic used
90s total deadline and zero transient retries, avoiding repeated requests. OAuth
and DAT destinations were read from the existing external configuration.

| Check | Login token | DAT outcome | DAT elapsed |
| --- | --- | --- | --- |
| Single GET | 0.734s; ready at 05:00:16.769 UTC | HTTP 504 | 60.860s |
| Range-mode probe `bytes=0-0` | 0.523s; ready at 05:01:35.251 UTC | HTTP 504 | 60.963s |

The existing fresh-job token acquisition also succeeded before both GETs.
The Range test had concurrency one, parts 16 MiB and zero transient retries.
Its initial one-byte probe failed; no actual parts were requested. No rate-limit
error was observed. Requests were sequential, not a parallel load test.

Interpretation: a received HTTP 504 is distinct from the client's 90s deadline.
The failure occurs before the client has a usable DAT body, so it does not reach
AES-GCM, Bloom verification, dataset transformation or matching. A one-byte Range
request does not avoid the delay. This rules out total transfer size as the sole
explanation for this pair of failures; it does not identify which upstream
component is slow or prove that every server request fails.

## Follow-up at the origin

Inspect gateway and upstream service logs for the two windows above, allowing
about 61 seconds per request. Identify the upstream timeout and whether the
service has entered its file lookup/build/read path or is waiting on a dependency.
Only those logs can distinguish gateway routing, service execution and storage or
database latency. No unsupported root cause or timeout configuration is asserted.

The matching service implementation was not found locally: a path-based check of
related projects and a Go-source search for the route/envelope magic found none.
The similarly named local partner-get-member directory contains analysis
artifacts, not the API service. The service repository/path was requested for
an actual origin-side repair. No external message or deployment was sent.

## Verification, safety and Definition of Done

- [x] T3 classification, bounded plan and ADR-0020 written before any changes.
- [x] Ordered design consultations recorded in dat-gateway-plan.md.
- [x] Current diagnostic compiled by `cargo run --offline --release --example matching_diagnose -- TEMP_SETTINGS_JSON single`.
- [x] Live Single and Range checks reproduced the reported 504 independently of the TUI; both exited with the failure status.
- [x] No secret/key/token/DAT in files, arguments, environment or printed output; isolated session memory released on exit.
- [x] Temporary diagnostic metadata removed after both checks; no user job created, deleted, duplicated or processed.
- [x] Local scoped review completed; no speculative source patch, retry-policy change or settings default flip.
- [x] Evidence and origin-side follow-up documented; no independent review or service-health success claimed.
- [ ] Origin-side correction and successful current DAT validation: pending the API/gateway implementation or operational access.

No application source, API/codec/schema/concurrency primitive/dependency changed.
Code-change coverage/mutation/fuzz/load gates therefore have no new implementation
to validate. Existing 246-test results belong to the preceding progress repair,
not a new run or proof of a server fix. CI, PR/merge/release, two-reviewer approval,
dashboards/canary/staging rollback gates are not performed or claimed: no such
publication/deployment is part of this local investigation.

Rollback: there is no application-state/code change to undo. Both diagnostic
sessions exited and their temporary metadata was deleted. Preserve the existing
job and retry its UUID after the origin becomes available. Never disable GCM or
checksum validation, persist the DAT or increase request fan-out to bypass 504.

## Follow-up: exact supplied trailing-slash URL

2026-10-16, 05:40–05:41 UTC. The user clarified that there are exactly two
endpoints: the common token endpoint and the encrypted DAT endpoint. References
to service mean the server behind the DAT route, not a third API. The existing
flow is token acquisition, authorized DAT GET, then local AES-256-GCM opening
and Bloom validation in memory. The AES key is not transmitted to either API.

The newly supplied DAT path includes a trailing slash; saved settings and the
earlier two checks did not. One additional isolated Single GET tested that exact
trailing-slash URL, with the same token URL, 90s deadline and zero transient
retries. Login succeeded in 0.604s at 05:40:17.019 UTC; fresh-job OAuth also
succeeded. The DAT endpoint returned HTTP 504 after 60.872s. This attempt did
not obtain a DAT for decryption. Both URL forms failed in the observed checks;
that does not establish identical routing or identify the slow upstream.

No application code or real settings changed. The saved-settings before/after
SHA-256 matched. No stored job metadata was present in this follow-up's initial
inventory; the diagnostic did not create, delete or write a user job. Temporary metadata and its
state-hash inventory were removed, and the diagnostic exited. Ordered local
consultations remain as recorded in the plan; no source patch is justified by
this result, so no new full-suite/coverage/build success is asserted.

Follow-up T3 DoD:

- [x] Clarified the two-endpoint contract and what service refers to.
- [x] Tested the exact supplied trailing-slash URL and recorded the live 504.
- [x] Preserved real settings/jobs; released the isolated session and removed temporary metadata.
- [ ] Successful DAT delivery/origin repair remains unresolved; server/gateway logs are needed to locate the timeout.
