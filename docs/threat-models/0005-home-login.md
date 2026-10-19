# Home login threat delta

Assets: client secret, access/refresh tokens, endpoint identity, local job/results and session state. Boundaries: terminal input → UI, UI → worker/secret store, worker → provider, settings → credential/cache profile. An attacker may supply malformed endpoints/responses or attempt navigation before login; a local process with the user's filesystem/Keychain privileges is outside the UI session boundary.

| STRIDE | Boundary / failure | Control and evidence |
|---|---|---|
| Spoofing | Saved credentials or menu actions impersonate login | Fresh secret and successful acquisition required each launch; navigation and command/use-case guards; no persisted signed-in flag. Locked-route regression tests. |
| Tampering | Destination/client edits reuse a different token | Existing normalized full-profile binding retained; empty optional destinations explicitly hashed; edits relock and clear loaded snapshot/access cache. Profile-change and existing isolation tests. |
| Repudiation | Failed operation appears successful | UI success only consumes SignedIn worker outcome; existing safe oauth_ready/oauth_failed tracing; error/cancel tests. No credential values in lifecycle output. |
| Information disclosure | Secret appears in screen, history, settings or diagnostic | Fixed mask, no secret arguments, zeroize form/paste buffers, Keychain custody, sensitive bearer header, bounded/sanitized HTTP errors, no redirects. Render tests and owned PTY capture scan. |
| Denial of service | Large paste, hanging issuer or blocked Keychain freezes UI | Existing input/response limits and deadlines; existing single worker, joined blocking preparation; cancellation watch; no POST retry loop. Cancel/retry and busy-input tests. |
| Elevation of privilege | Slash command, alternate bearer mode or old splash bypasses login | TUI OAuth-only presentation; protected routes/actions require session; Settings/help remain usable for setup; provider still authorizes all external requests. CLI remains a separate explicitly invoked automation interface. |

Sign out is local, without revocation; it discards RAM access and UI snapshot while preserving generation counters, preventing a stale in-flight 401 from matching a later token generation. Refresh tokens and historical jobs remain recoverable. Login is a workflow requirement, not an OS-level lock on local files or the CLI.

Local self-review only. Real provider/Keychain acceptance, independent security review, signed SBOM and release sign-offs remain unverified; no publication is performed.
