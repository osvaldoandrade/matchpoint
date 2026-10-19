# ADR-0011 — One token endpoint and login-only credentials

Accepted for local implementation, 2026-10-15. Supersedes ADR-0009's separate refresh destination and ADR-0010's editable credential Settings.

Context: user confirms that refresh and acquisition share a URL, login must actually obtain a token, and authenticated settings must not edit client ID/secret. The user also requests a million-row SHA-256-hex CPF fixture.

Decision: token_url is the sole destination of both JSON OAuth grants. Read old reset_token_url for compatibility, ignore it for routing and omit on save. Hide the old CLI flag and reject a distinct destination; never silently send credentials there. Preserve profile isolation, using the common token URL in both historical hash slots; old common-URL credentials stay compatible, distinct-URL profiles need fresh login. No automatic copying of credentials or filter cache between profiles.

Home login is the only interactive client-ID/secret/token-URL editor. Settings exposes operational configuration and read-only token metadata/renewal. It cannot change login metadata or save credentials. Sign out and log in to change identity. Only a successful grant with valid access token/type/expiry and successful secret-store commit unlocks the UI. A 2xx empty/malformed response, denial, cancellation or storage error does not authenticate. Existing automatic refresh and bounded invalid_grant/401 behavior remain.

Alternatives rejected: keeping editable refresh URL with a copy default (permits drift); editing identity in authenticated Settings (contradicts request); treating saved credentials or 2xx alone as login (does not prove token issuance); auto-migrating old Keychain items (could forward credentials under a changed destination profile).

Consequences: the deprecated destination is inert; users with historical distinct URLs log in again. CLI scripts passing distinct legacy destinations fail with a safe configuration error. No file format/job schema or animation changes. CSV stores already-hashed synthetic identifiers; dataset metadata explains Transformation=None to avoid double hashing. No claim that synthetic CPF check digits identify real people.

Threat/recovery delta: same TLS/no-redirect/response-size/timeout controls, default locked UI and full profile binding; credential editors and their save path are removed. Monitor only safe auth status/events. At minute 5 retry Home with corrected nonsecret metadata; at minute 15 reproduce using MemorySecrets loopback tests; at minute 60 restore source snapshot/rebuild if needed. Preserve all jobs/settings/Keychain, no manual secret migration. Local rollback target <5 minutes and RPO zero committed results. Real provider/native Keychain, independent security review and production ramp/sign-offs remain outside verified local scope; no publication.
