# API configuration threat-model delta

Scope: local implementation on 2026-10-13 UTC. Assets: client secret, refresh token, CPF records, and the chosen API destination. The trusted local operator can edit configuration; terminal input and job overrides can be malformed. The application must prevent inadvertent disclosure and reuse of credentials against a different destination.

| STRIDE | Boundary / failure | Control |
| --- | --- | --- |
| Spoofing | A job overrides the OAuth or process destination | Match both URLs and the client ID to the enabled saved profile before any remote I/O |
| Tampering | Old or malformed configuration enables remote calls | Missing enable flag defaults to false; validate configuration on save and run |
| Repudiation | Operator cannot distinguish a configured profile from a working API | Show saved URL and remote state; never claim a credential is validated merely because it was saved |
| Information disclosure | Secret appears in rendering, CLI errors, history, config, or events | Hidden prompt/form, zeroized input, no secret argument, redacted sensitive REPL commands; only Keychain stores the secret |
| Denial of service | Invalid rates, huge inputs, or stalled calls | Positive finite rates, bounded form strings, response limits, timeouts, worker cap, existing circuit breaker |
| Elevation of privilege | Test fixture runs on an external host | Test mode and simulated contract stay loopback-only; remote access requires HTTPS and explicit enabled configuration |

Failure catalogue: configuration save failure retains the form for correction; Escape clears temporary secret input; an ambiguous HTTP outcome remains `UNKNOWN`; unexpected destinations fail before token acquisition. At minute 5, pause/interrupt sends and inspect job counts; at minute 15, disable remote access and check saved URLs; at minute 60, inspect UNKNOWN records and obtain provider evidence before requeueing. Never infer that an unknown request failed at the provider.

Independent security review, provider validation, signed distribution, and production staged-ramp evidence remain release gates outside this local edit. No real credentials or production data are required for automated validation.

## 2026-10-14 — Local slash-command input

The command menu adds no trust boundary. It shows static descriptions and maps local input to existing CLI actions; selected jobs still resolve through `App`, and the existing busy/profile guards remain authoritative. Completion never invokes an action. Import/export commands remain editable drafts. The command field accepts at most 8,192 UTF-8 bytes; suggestions are bounded to 12 entries and rendering to six visible rows. Sensitive commands remain masked and excluded from history; the menu never includes user values. Configuration-form input remains isolated from global shortcuts. Spoofing/tampering/elevation use existing executor guards; disclosure and input exhaustion use the controls above; history and existing job events provide the same local operation evidence. Tests use synthetic data and in-memory secrets.
