# Provider Bloom compatibility threat delta

Assets: credentials/access token, provider membership bit array, correct local outcomes and existing job pins. Boundaries: explicit issuer/provider HTTPS → bounded stream → codec → local job/output; terminal credentials → RAM token; private provider response → diagnostic verifier.

| Threat | Control |
|---|---|
| Credential disclosure / forwarding | Hidden getpass input in the diagnostic, no credentials in source/arguments/logs/settings/Keychain, token RAM only, exact user-supplied HTTPS destinations and no redirects. Real payload is private outside repository. |
| Format spoofing / algorithm confusion | Explicit native/provider header discriminator; require known version/algorithm and zero reserved fields; keep hash families separate. No conversion of unknown bits to a guessed native filter. |
| Tampering | Verify SHA-256(header+bits) trailer for provider, existing native checksum and GZip CRC; exact length/single member and no trailing bytes; preserve validated fingerprint and cache/job pins. |
| Memory/CPU denial | Check m/k/capacity/FPP/budget before one bounded bit-array allocation; limited download/timeout; no optional GZip headers or unbounded decompression; existing joined decoder worker/cancellation topology. Mutation harness covers hostile sizes/flags/truncations/CRC/checksum. |
| False output from malformed keys / wrong probes | Decode SHA-256 hex directly, explicit hashing for raw identifiers, reject malformed/empty digest inputs, wrapping big-endian digest probes and LSB bit order covered by independent vectors; checked errors propagate before final output publication. |
| False assurance | Distinguish independent verifier agreement from a labelled provider membership audit; no claim of real-cohort false-negative/FPP certification without controls. Native strict transform compatibility remains. |
| Sensitive test fixture leakage | Repository fixtures use only small synthetic keys and independent generated bits. Real provider response/cache/output stay in owner-only temporary storage; repository receipts contain only aggregate counts/type/size/fingerprints. |

No new dependency, endpoint, lock, secret custody mechanism or per-key network call. The existing native producer/protocol and interactive login remain unchanged; animation/art/runtime SHA-256 values are preserved. Local self-review only; independent security/signing/production gates remain unverified. Recovery/rollback is in ADR-0012 and the matching runbook.
