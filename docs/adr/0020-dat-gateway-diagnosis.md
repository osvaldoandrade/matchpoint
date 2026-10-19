# ADR-0020: diagnose persistent DAT gateway timeouts before changing policy

2026-10-16. T3 local investigation; accepted for bounded diagnosis only.

Context: the user still receives HTTP 504 after bounded GET retries. The source
URL is correct; current Single GET uses a 600s total and 60s body-idle deadline.
Earlier local/QA evidence demonstrated successful encrypted DAT downloads and
occasional remote 504 responses around 60s. Progress UI cannot repair that cause.

Decision: reproduce with the existing isolated hidden-input diagnostic, first
Single GET without transient retries, then a bounded sequential Range comparison
only if necessary. Preserve all existing user settings/jobs and credential policy.
Do not add a speculative header/query, unbounded retry, POST replay or disk cache.

Alternatives: increasing client timeout cannot override a received HTTP 504;
blind parallelism can aggravate provider 429; assuming a server outage without
a live reproduction leaves the actual request behavior unverified.

Consequences: testing may confirm an external blocker instead of producing a
client patch. Successful validation proves that attempt, not service reliability.
Any source/policy change requires evidence and an amended implementation plan.

Rollback: exit the diagnostic and remove temporary metadata; no saved secrets,
payloads or user state are introduced. See ../dat-gateway-plan.md for limits.
