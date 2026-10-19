# Tabbed terminal Settings — threat-model delta

The local application owner is configuring processing, endpoints, credentials and presentation. The assets are job data, integrity metadata and OAuth credentials. Untrusted clipboard/file/endpoint inputs can reach the terminal adapter; accidental input or local content can corrupt configuration, disclose a secret, inject terminal controls or initiate unintended work. This redesign adds presentation and optional preferences and retains the application engine as the trust boundary.

| STRIDE risk | Control and verification |
| --- | --- |
| Spoofed or unintended API target | Preserve HTTPS/URL-secret checks, explicit remote enablement and exact saved-profile matching; changed identity/destination requires explicit credentials. |
| Configuration/job tampering | Validate bounded threads/RPS/batch values, stage changes until Save, preserve active-job locks and input/export SHA-256 checks. |
| Ambiguous operations | Bind async results to the originating job; preserve actionable English diagnostics and explicit partial export. |
| Secret disclosure | Zeroizing form storage, masked cells, no secret values in history/events/files/profile argv; test synthetic secret absence in rendered/ANSI/history artifacts. |
| Input/animation denial of service | Bounded literal paste, numeric ranges, fixed sample limits, clipped drawing, no unbounded animation allocation; benchmark frames and navigation. |
| Privilege expansion or accidental send | Keep remote enablement explicit, no implicit run during settings/import/paste, preserve engine authorization and safe argv without shell expansion. |

No new auth mechanism, credential rotation, external dependency or production secret operation applies. Review the final changed surface against these controls and record the result in the workflow ledger.
