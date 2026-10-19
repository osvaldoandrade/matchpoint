# MATCHPOINT

Ultra-fast, safe dataset matching. Match customer IDs, device IDs, product SKUs or other identifiers in **CSV or Parquet** against very large reference collections using local **Filtering**. The executable and storage namespace remain `pgm-data`.

1. Obtain a fresh OAuth token and download an AES-256-GCM encrypted DAT for each job.
2. Select a dataset, its identifier column, and an optional transformation.
3. Stream the dataset against the immutable in-memory filter and export one boolean per original key.

```text
customer-000042: true
customer-000099: false
customer-000042: true
```

Order, duplicates and leading zeros in string keys are preserved. `false` excludes membership; `true` is a possible match, with a **0.1% target false-positive rate**. Identifiers stay local; no per-identifier HTTP request occurs. Parsing, checksum, transformation or I/O errors stop the job rather than becoming `false`.

## Dataset examples and scale

Use datasets from any domain with a selected UTF-8 or integer key column.
CSV/Parquet are the supported formats; string keys preserve leading zeros.
Prepare composite keys as a deterministic string column using the same rules
as the reference producer. For a reference built by hashing raw keys with SHA-256:

```sh
pgm-data match --input customers.csv --key customer_id --transform sha256 --threads 8
pgm-data match --input events.parquet --key device_id --transform sha256 --threads 8
pgm-data match --input catalog.csv --key sku --transform sha256 --threads 8
```

The dataset streams in bounded batches, while workers share one compact,
immutable reference in locked RAM. This supports very large inputs without
loading the entire dataset into memory. The reference must fit the configured
memory budget; throughput depends on storage, decoding, transformations and CPU.

DAT authentication and integrity checks finish before matching. Errors stop the
job. Positive candidates may need authoritative verification because false
positives are possible; results describe the reference at snapshot time.
Input files, job storage and exports retain original keys and are not encrypted
by the application. Protect them according to their sensitivity.

## Terminal

**F1** opens the full manual from any screen. Browse chapters with Tab/Shift+Tab
or h/l; scroll with j/k, Space/b, Home/End; search all chapters with `/` and
repeat with n/N. q/Esc restores the previous screen and its form draft. Wide
terminals show a contents panel; compact layouts retain the same chapters.
The manual documents every command, setting, key rule, result, file and recovery
path from the same source as its terminal/Unix exports:

```sh
pgm-data manual | less
pgm-data manual --roff > /tmp/pgm-data.1
man /tmp/pgm-data.1
pgm-data match --help
```

```sh
cargo run --bin pgm-data -- tui
```

Home opens directly with **Sign in**: client ID, hidden client secret, token URL and hidden DAT AES-256 key (Base64). Tab/Enter advances fields; Enter on the AES key or Sign in submits. Only a valid token response unlocks imports and Jobs. Client ID and URL may be prefilled; both secrets must be entered on every launch. F2 opens advanced Settings before login (DAT URL, limits, appearance); F1 opens help. Esc quits from login; Ctrl+C stops safely. After login, L signs out locally without revocation or deleting jobs. Change client ID, client secret, token URL or DAT key by signing out and using Home again. DAT/scope changes require signing in again. The same token URL handles login and renewal; the DAT URL is configured separately. **New Job** comes before Jobs and Settings. New Job has four fields: dataset path, key column, transformation and matching threads (1–256). Enter prepares and runs it. CSV headers/delimiter/trim defaults live in Settings → Rules. Settings tabs: API, Rules, Secrets, Appearance. Tab moves through fields; arrows select tabs/toggles; Ctrl+S saves. N/J/S open the corresponding screens, D preloads the next job’s DAT, R runs/resumes, P pauses, T edits the selected job’s thread count, X/Delete asks to delete, E exports and K verifies checksums. `/` opens command suggestions. Esc returns Home; Q quits outside editors; Ctrl+C stops safely.

For Ghostty, run `packaging/ghostty/run-ansi.command`. It builds the current binary and applies the dedicated font profile. Save Appearance and reopen through the launcher to apply font changes; terminal applications cannot change the host font directly.

## CLI

Configure the source once; each matching job downloads its own DAT. The optional token is entered in a hidden interactive prompt and stored in macOS Keychain, bound to the exact source URL. Tokens and AES keys are never command-line arguments. Matching/download commands prompt for the AES key without echo; it is not persisted between CLI processes.

```sh
pgm-data bloom configure --dat-url https://example.com/partners.dat
pgm-data bloom token       # only if bearer authentication is required
pgm-data match --input partners.csv --key identifier --threads 8 --output results.txt
pgm-data match --input partners.parquet --key identifier --output parquet-results.txt
```

The **New Job** screen, `match` and `job create` pin their primary result to
**`./result-JOB_ID.txt`** in the launch working directory. Jobs shows that actual
**absolute Result path**. Pause/restart/resume keeps the original destination,
even when resumed from another directory. `match --output FILE` overrides the
primary file; an existing directory receives the generated UUID filename.
Publication is atomic and never overwrites an existing destination. Partial
output and checkpoints stay under application storage until completion.

TUI **E** and `job export` make an optional additional copy, defaulting to the
current working directory. Exporting to the primary file itself reports that
verified file without copying it. Deleting stored job state preserves external
results and exports. Existing jobs retain their original result locations.
The Ghostty launcher explicitly sets the project directory as its working directory.

OAuth2 uses one common token URL for acquisition and renewal, plus a DAT URL. Both initially empty, stored only in external `matching/settings.json`:

```sh
pgm-data bloom configure --token-url https://auth.example.com/token \
  --dat-url https://data.example.com/partners.dat \
  --client-id YOUR_CLIENT_ID --scope partners.read --oauth
pgm-data auth credentials    # hidden client secret + optional refresh token
pgm-data auth get-token
pgm-data auth refresh-token  # reset means renewal, not revocation
pgm-data match --input partners.csv --key identifier --threads 8
```

Settings → API edits DAT URL, optional scope and limits. Secrets shows read-only session/access-token status and the common token URL, plus **Renew token**. Client ID and client secret can only be entered on Home login; neither can be changed inside the signed-in UI. Sign out with L, then log in again to change identity. Login requires a valid issued access token, Bearer type and positive expiry; HTTP 2xx by itself does not authenticate. Rejected attempts preserve previously valid saved credentials.

Both initial acquisition (`client_credentials`) and renewal (`refresh_token`) POST to `token_url` using the established JSON contract. Home login keeps client secret, access and rotating refresh tokens in session RAM; login, jobs and renewal never access Keychain. Explicit CLI `auth credentials` retains Keychain persistence for automation. Expiry triggers renewal; a DAT 401 permits one authentication renewal and one GET retry. `invalid_grant` permits one fallback to acquisition at the same endpoint. No revocation request is sent. `/token` and `/refresh` work only during an authenticated UI session; CLI automation retains its existing modes.

Credentials and resident snapshots are bound to DAT URL, common token URL, client ID and scope. Old jobs remain pinned to their exact filter. Legacy `reset_token_url` is read but ignored and omitted on save; the hidden legacy CLI flag is accepted only when it equals the common token URL, and rejects a distinct URL without saving. Historical profiles with distinct token/refresh destinations require fresh login; no credentials are copied between profiles. `url` JSON and `--url` remain aliases for `dat_url` / `--dat-url`.

Settings → API also selects **Single GET**, **Parallel GET (HTTP Range)** or **Multipart POST (form-data)**. Defaults: DAT deadline 600 s (separate OAuth deadline 60 s), body idle 60 s, Range parts 16 MiB, 4 simultaneous requests and 2 transient GET retries. Part size is 1–128 MiB, concurrency 1–16 and retries 0–5. Multipart text fields use a bounded JSON object, e.g. `{"format":"dat"}`; they are saved as non-secret metadata. Never enter credentials or AES keys there. Tab scrolls through all API fields; arrows/Space cycle Download request, Ctrl+S saves.

DAT transfers use **HTTP/1.1 over verified TLS** to avoid the repeated HTTP/2 body
resets observed on the provider route. OAuth retains its existing client. Parallel
mode still uses the configured number of connections. A stream interruption or
drop in speed alone does not establish rate limiting; HTTP 429 and provider logs
are separate evidence. Existing bounded retries and exact Range recovery remain.

Parallel downloads require exact ranges and a strong ETag, with `If-Match` on every part. If the initial Range probe receives HTTP 200, it consumes that response as a single stream. A missing validator or inconsistent later part fails closed. Multipart POST requires server support; it has no transient-error retries (the initial request retains one 401 authentication recovery). Transient GET 429/502/503/504 responses, interrupted bodies and idle stalls share one retry budget across the whole DAT, with bounded backoff/jitter and one total deadline. A strong ETag/known size allows exact Range/If-Match suffix recovery in locked RAM; otherwise the prefix is wiped before a full GET restart. Parallel failures cannot multiply the attempt budget. Changed versions/ranges abort, and every completed transfer still passes AES-GCM/checksum validation. Read the **DOWNLOAD REQUESTS** manual chapter for all limits and Retry-After behavior.

A failed download preserves the prepared job: select it under Jobs and press **R** to retry the same UUID. Only failures before input preparation return the New Job draft. Existing jobs are never automatically deleted or merged. An HTTP 504 comes from the provider gateway and remains possible even with a longer client deadline.

The **LIVE ACTIVITY** panel distinguishes input preparation, OAuth, DAT response
wait, transfer/retries, decryption/validation and local matching. It shows live
elapsed time, attempts/last HTTP failure, received MiB or processed rows and rate.
Unknown totals have no percentage. Matching updates every 8,192-row wave; durable
checkpoints keep their configured interval. **P** stops safely; **R** retries the
same prepared job once the operation stops.

Both URLs require HTTPS without embedded credentials, query or fragment. Redirects are rejected. `--allow-http-loopback` exists only for local fixtures. No per-key API endpoint exists.

Rules: `--key identifier` or one-based `--key '#2'`; `--transform none|sha256|sha256-raw`; `--trim true|false`. SHA-256 means lowercase hexadecimal; `sha256-raw` means 32 digest bytes. Native DATs embed transform/trim rules and reject a mismatch. Provider DATs consume SHA-256 digests: use None for a 64-character hex column, or SHA-256 for raw identifiers. The provider reader decodes existing hex directly; malformed digests stop the job. Only the lookup key is transformed; output keeps the original value. CSV also supports `--headers false` and `--delimiter ';'` (or `tab`). Defaults are headers, comma, first column, no transform/trim.

Parquet reads only the selected top-level UTF-8 or integer column, in batches of up to 8,192 rows. Use strings to preserve leading zeros. Null, floating-point and nested keys fail explicitly. Supported codecs: uncompressed, Snappy, GZip, LZ4, Zstd and Brotli. The full dataset is never collected in RAM; memory also includes the current record/batch and decoder buffers.

## Produce the DAT

The producer writes [native PGMBLOOM v1](docs/contracts/bloom-dat-v1.md), a seeded 128-byte little-endian header wrapped in GZip. The reader also accepts [provider PGMBLOOM v1](docs/contracts/provider-bloom-v1.md): a 64-byte big-endian header, direct digest probes, a bit array and SHA-256 trailer, delivered raw or in plain GZip. It detects the layouts explicitly, validates sizes/checksums/parameters, and preserves existing native pins. Provider snapshots remain immutable and are never rewritten by the native encoder. `bloom inspect` reports `wire_format`. The generator accepts CSV or Parquet.

```sh
pgm-data bloom build --input members.csv --key identifier --output partners.dat
pgm-data bloom inspect partners.dat
```

Serve that DAT from the configured endpoint. `build` targets 0.1% false positives with a sizing margin and rejects capacity overflow. Defaults limit downloaded files to 256 MiB and the decoded bit array to 512 MiB. Settings exposes both limits and the download timeout. A checksum detects corruption; HTTPS establishes transport trust.

## Encrypted DAT

The download body is **PGMENC01 header (32 bytes) + nonce (12 bytes) +
ciphertext + GCM tag (16 bytes) + SHA-256 trailer (32 bytes)**. All integers are
big-endian. The reader validates header fields, declared plaintext length + 92,
and the outer checksum before AES-256-GCM opening. The exact 32-byte header is
**AAD**; empty AAD is rejected. Authentication precedes raw/GZip Filtering data decoding
and publication. Wrong header/size/checksum/key/tag, truncation and plaintext
responses in an encrypted session stop loading.

Home requires a standard padded **Base64 key encoding exactly 32 bytes**.
This is the raw AES key, not a password to hash or derive. Input stays masked.
The key remains in locked RAM for the signed-in session; it is never written to
Settings, Keychain, job metadata, logs or HTTP requests. Sign out with L and log
in again to change it. Failed/cancelled login clears both secret drafts and
installs no key. Login checks key syntax and the issued OAuth token; correctness
of the AES key is verified when a DAT is authenticated.

Each job still gets a fresh token/DAT and releases its Filtering data on completion;
the login key survives between jobs until sign-out, profile change or exit.
CLI `match`, `job run/resume` and `bloom download` use a hidden interactive key
prompt; there is no key flag/environment variable or saved-key shortcut. Local
`bloom build/inspect` still operate on explicitly supplied plaintext DAT files.
The library retains its explicit no-key plaintext mode for trusted local uses;
application encrypted sessions never fall back to plaintext.

Download and decryption staging use locked, zeroized anonymous mappings. Peak
memory includes copying bounded transfer chunks into a contiguous AEAD buffer,
then compressed plaintext plus decoded bits. HTTP/TLS/codec/crypto-library
internals can hold ordinary temporary buffers; this is not whole-process memory
confidentiality. See [wire contract](docs/contracts/encrypted-dat-gcm.md) and [PGMENC01 verification/DoD](docs/pgmenc01-verification.md).

## Resident memory and CPU workers

Downloaded DAT bytes are never staged or cached in a file. The decoded bit array
uses anonymous memory locked with `mlock` before use. Loading fails if the OS
cannot lock it; there is no disk or unlocked-memory fallback. The last owner
zeroizes the bit array before unmapping it. Transfer staging also uses bounded
locked chunks; HTTP/TLS/decompressor internals may use temporary normal buffers.
This is not a guarantee against privileged process dumps, hibernation or abrupt
termination preventing an application-level wipe.

A paused job keeps its own filter in the running process. Completion, failure,
selected-job deletion, sign-out or normal exit releases it after workers join.
Each subsequent OAuth job obtains a fresh `client_credentials` token and DAT,
even if the previous token remains valid or the returned filter is identical.
D preloads one next job; it does not enable reuse across jobs. A standalone
`bloom download` verifies in RAM and releases everything when that command exits.
The first launch of this policy removes recognized legacy cache files; keep old
binaries stopped during that transition. Explicit `bloom build` output remains
an intentional producer file.

Choose 1–256 threads in New Job, Settings → Rules, T on Jobs or `--threads`.
The default is available CPUs capped at 32; old job metadata defaults to 1.
Persistent workers share **one read-only reference allocation**. The coordinator
parses bounded waves of up to 8,192 real CSV/Parquet records, dispatches chunks,
and writes results in original order. Quoted multiline CSV is preserved; raw
files are never split at arbitrary newlines. The input is streamed, and memory
is bounded by configured DAT limits plus batches and decoder buffers. Speedup
depends on hashing, filter size, parsing and storage; more threads may be slower.

## Jobs and recovery

```sh
pgm-data job create --input partners.parquet --key identifier
pgm-data job list
pgm-data job run JOB_ID
pgm-data job resume JOB_ID --threads 4
pgm-data job threads JOB_ID --threads 8
pgm-data job checksum JOB_ID
pgm-data job export JOB_ID --output results.txt
pgm-data job delete JOB_ID --yes
```

Jobs snapshot their input and pin the rule and exact DAT fingerprint. Checkpoints commit every 100,000 rows by default (configurable in Rules) and on pause, with ordered output and SHA-256 prefix verification. A restart requires a fresh token/download of the **exact pinned DAT**; a changed fingerprint refuses resume. It truncates uncommitted bytes before resuming; Parquet resumes by row offset. Final output is published atomically only when complete. Export preserves an existing destination. Keys containing separators/control characters are JSON-string-escaped so each record remains one physical line.

Deleting a job removes only its stored input snapshot, internal results and checkpoint. The original dataset, external primary result and exports are preserved. TUI confirmation defaults to Cancel; the CLI requires `--yes`. Active jobs are locked against deletion. Interrupted staged deletions are retried at the next launch.

State: `~/Library/Application Support/pgm-data/matching/` on macOS, override with `PGM_DATA_HOME`. It stores input snapshots and checkpoints. New jobs publish their complete result to the pinned launch-directory destination (`./result-JOB_ID.txt` by default); `matching/jobs/ID/result.txt` remains the legacy location. Lifecycle logs are `matching/events.log`, without keys or credentials. Matching directories/files and published results are owner-only. Historical jobs and credentials remain in their original namespace; legacy library recovery APIs remain available but are not used by the executable matching workflow.

See [resident/parallel decision](docs/adr/0014-resident-parallel-matching.md), [resident/parallel verification](docs/resident-parallel-verification.md), [recovery](docs/runbooks/matching/local-recovery.md), [decision](docs/adr/0008-local-bloom-matching.md), and [local verification](docs/bloom-matching-verification.md). No production endpoint, release signing, notarization or deployment acceptance is implied by local tests.

## Development

```sh
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo audit
cargo build --release --bin pgm-data --bin pgm-bench
cargo run --release --bin pgm-bench -- --rows 1000000
cargo run --example matching_review
```

The benchmark uses synthetic local identifiers and makes no HTTP calls. Tests use temporary state, in-memory secrets and loopback servers; they do not contact production APIs.
