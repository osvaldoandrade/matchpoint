# ADR-0021: isolate large DAT transfers from HTTP/2 stream resets

2026-10-16. T3 local transport repair; Single GET verified against the live API.

Context: recent successful DAT headers use HTTP/2.0, but body streams reset in
both Single and Parallel modes. Exact Range recovery already works, then exhausts
its bounded retry budget. HTTP 429 is not observed in these failures; attributing
the stalls solely to rate limiting is unsupported.

Decision: constrain DAT transport to HTTP/1.1 while retaining TLS certificate
verification and all request, auth-header, deadline, Range/version, RAM and
cryptographic checks. Keep the OAuth client unchanged. Verify a current full
download with one connection and zero transient retries before claiming repair.

Alternatives: more concurrent parts can amplify upstream load; more retries hide
the repeated reset and amplify requests; larger deadlines do not repair a stream
reset. Disabling TLS, AES, checksums, validator checks or RAM-only storage is not
an acceptable workaround. No protocol auto-fallback loop or new dependency.

Consequences: DAT requests use one established HTTP version; parallel mode still
uses the user's bounded number of connections. A small dedicated client builder
keeps the OAuth path unchanged. Shared transport bounds are deliberately repeated
at this policy boundary and verified by existing deadline/redirect/auth tests.
The change mitigates the observed path; it cannot guarantee provider availability
or identify its internal HTTP/2/reset/quota cause without upstream evidence.
An HTTP/2-only source would require revisiting this DAT policy; the current
provider has been verified to support HTTP/1.1. No silent protocol retry loop.

Rollback: restore task source and rebuild; no settings/data migration. Preserve
the same job UUID/checkpoint and exact filter pin. See ../http-body-plan.md.

Evidence: the first isolated comparison failed before DAT while requesting a
token. A subsequent fresh session obtained a token in 0.488s, received HTTP/1.1
200 headers in 0.446s and all 169,768,248 bytes in 6.530s. Envelope/GCM/Bloom
validation completed in 7.254s without transient retries or personal-key probes.
The plaintext fingerprint matches the previously verified provider snapshot.
No HTTP/1.1 provider failure or 429 was observed in that completed transfer;
this does not prove the provider's internal HTTP/2 reset cause or long-term SLO.
