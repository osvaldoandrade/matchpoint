//! One authored manual for the terminal pager, plain text and Unix man output.
use crate::tui::{palette as p, view::block};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Rect, style::Style, text::Line, widgets::Paragraph, Frame};

pub const CHAPTERS: &[(&str, &str)] = &[
    ("NAME", "MATCHPOINT - Ultra-fast dataset matching\n\nUltra-fast, safe matching for datasets from any domain, supplied as CSV or Parquet, against very large reference collections. Filtering runs locally using an immutable reference snapshot loaded from a fresh DAT for each job. The executable remains pgm-data. Identifiers stay local; no per-key API requests occur.\n\nfalse means absent from this snapshot. true means possible membership, not confirmed identity. False positives are expected; the supported target is at most 0.1%."),
    ("SYNOPSIS", r#"    pgm-data [COMMAND]
    pgm-data tui
    pgm-data manual [--roff]
    pgm-data COMMAND --help
    pgm-data --version

No command opens the interactive UI when stdout is a terminal. Redirected stdout without a command exits with status 2. Use manual for pipeable documentation, or --help for the exact flags accepted by a command.

    pgm-data manual | less
    pgm-data manual --roff > /tmp/pgm-data.1
    man /tmp/pgm-data.1"#),
    ("QUICK START", r#"1. Sign in on Home with client ID, hidden client secret, token URL and hidden Base64 AES-256 DAT key. Only an issued Bearer token with positive expiry unlocks the workspace.
2. Open Settings > API, enter the DAT URL, then Ctrl+S. Changing the DAT URL or scope requires signing in again.
3. D preloads a DAT for the next job in locked RAM, or let the job download it automatically. Nothing is cached on disk.
4. N opens New Job. Choose path, key column, transformation and matching threads. Enter prepares and runs the job.
5. J opens Jobs. P pauses, R resumes, T changes threads, X asks to delete, K checks integrity and E exports a completed result.

For customer IDs in customers.csv, choose customer_id. Use SHA-256 hex when the reference was built from those raw identifiers with SHA-256. For an already hashed 64-character hex column with a provider DAT, choose None. Apply the same encoding, case, trim and transformation as the reference producer. Preserve leading zeros with string keys."#),
    ("MANUAL NAVIGATION", "Up/Down or k/j: scroll one line. PageUp/PageDown or b/Space: scroll one page. Home/End or g/G: first/last line of the chapter. Left/Right or h/l, Tab/Shift+Tab: previous/next chapter, wrapping at either end.\n\n/: enter a literal, case-insensitive search across every chapter. Enter finds the first matching line; n/N moves to the next/previous match and wraps. Backspace edits; Esc cancels search entry. Search is limited to 128 UTF-8 bytes. This is not the slash-command palette.\n\nq or Esc closes Help and restores the previous screen, including form drafts. Ctrl+C quits the application safely. F1 opens Help from any screen. The contents panel appears in wide terminals; the chapter number and reading position remain visible in compact layouts."),
    ("KEYBOARD AND COMMANDS", r#"HOME: Left/Right or Tab/Shift+Tab selects New Job, Jobs, Settings; Enter opens. N/J/S opens a screen, ? opens Help. L signs out locally. Q quits outside editors; Ctrl+C quits from any screen and waits for the active operation to stop safely.

FORMS: Tab/Shift+Tab or Down/Up moves focus. Left/Right/Home/End/Delete/Backspace edits text. Ctrl+U clears a field. Arrow keys or Space changes a focused toggle. Esc returns Home and discards unsaved form changes. Text letters remain text while editing; use F1 for Help.

SLASH COMMANDS: / opens suggestions outside forms. Up/Down selects, Tab completes, Enter executes, Esc closes. No arbitrary arguments, identifiers or secrets belong here.
    /new        Open New Job
    /jobs       Open Jobs
    /settings   Open Settings
    /help       Open this manual
    /download   Download and verify DAT
    /run        Run/resume selected job
    /pause      Stop active work safely
    /token      Acquire token when signed in
    /refresh    Renew token when signed in
    /threads    Edit selected job worker count
    /delete     Ask to delete selected job
    /quit       Quit safely

When another operation is active, wait or pause it before starting one. Help navigation never sends commands to the worker."#),
    ("LOGIN AND TOKENS", "Home asks for client ID, client secret, token URL and DAT key (AES-256-GCM, Base64 encoding 32 bytes). Tab moves between fields and Sign in; Enter advances from ID/secret/URL and submits from DAT key/Sign in. F2 opens advanced Settings before login. Esc quits when idle or cancels a pending login. Paste is supported; secret text stays masked.\n\nA 2xx response alone does not authenticate. Each launch requires fresh client secret and AES key input. Client ID/URL can be prefilled. Failed authentication keeps previously valid saved credentials. Home client secret, refresh token, access token and AES key stay in session RAM. Home login, new jobs and renewal never access macOS Keychain. Explicit CLI auth credentials still uses Keychain for automation. Sign-out and source-profile changes drop the RAM credentials. The AES key is never stored or transmitted. It is cleared on logout/profile change/exit. Login validates key encoding; a correct key is established when GCM authenticates a download.\n\nThe same token URL accepts client_credentials and refresh_token grants using the configured JSON contract. Expiry renews access; a DAT 401 permits one renewal and GET retry. invalid_grant permits one acquisition fallback. Refresh tokens rotate when returned. No revoke/reset endpoint is called.\n\nSettings > Secrets shows read-only session/token information and Renew token. Client ID, secret, token URL and AES key cannot be edited inside the signed-in UI. L signs out without deleting jobs or revoking access; use Home to change identity. CLI credential setup remains available for automation."),
    ("NEW JOB AND INPUT", "New Job has four fields: dataset path, key column, transformation and CPU matching threads (1-256). Enter starts preparation and matching. Use absolute paths or paths relative to the launch directory; quote paths containing spaces in the shell. The input is snapshotted into the job before matching.\n\nCSV: use an exact, unique header name or a one-based index such as #1. Without headers, use #index. Settings > Rules controls headers, delimiter and trim. Defaults: header present, comma delimiter, first column, no trim, None transform. Delimiter accepts one ASCII character or tab. Empty/missing keys, duplicated selected headers and malformed CSV stop the job. Keys must be 1-4096 UTF-8 bytes.\n\nParquet: choose a top-level UTF-8 or integer column by name or #index. Only the selected column is read, in batches up to 8192 rows. Null, floating-point and nested keys fail explicitly. Codecs: uncompressed, Snappy, GZip, LZ4, Zstd, Brotli. Use strings to preserve leading zeros.\n\nThe dataset streams from disk. RAM includes the filter, current record or batch and decoder buffers; the complete dataset is not collected in memory."),
    ("TRANSFORMATIONS", r#"Only the lookup value changes; output retains the original key. Trim, when enabled, removes surrounding Unicode whitespace before transformation. There is no numeric conversion, punctuation removal or automatic identifier normalization.

None / --transform none: use the original UTF-8 bytes for native filters. For provider filters, decode an existing 64-character SHA-256 hexadecimal digest into 32 bytes. Invalid provider digests stop the job.
SHA-256 hex / --transform sha256: hash the lookup value once and encode lowercase hex. For provider filters the digest is consumed directly.
SHA-256 raw / --transform sha256-raw: use the 32 raw digest bytes. These are not interchangeable with hex bytes in native filters.

Native DATs embed transform/trim rules and reject mismatches. Provider DATs accept SHA-256 digests; choose SHA-256 for raw identifiers and None for existing hashes. Never hash an existing digest again unless the producer explicitly did so.
    pgm-data match --input identifiers.csv --key identifier --transform sha256
    pgm-data match --input hashed.csv --key identifier_sha256 --transform none"#),
    ("JOBS AND RESULTS", r#"Jobs: Up/Down selects. R runs/resumes; P pauses at a checkpoint. T edits thread count; X/Delete opens a deletion confirmation. E asks for an export destination; Enter exports only a complete result. K verifies input/output checksums. H/Esc returns Home. N creates another job. A completed job cannot run again.

Each output line is the ORIGINAL key followed by : true or : false. Input order, duplicates and leading zeros in string values are preserved.
    customer-000042: true
    customer-000099: false
    customer-000042: true

These lines illustrate syntax, not membership of those example keys. Keys containing colon, backslash, quotes, control characters or surrounding whitespace use JSON string escaping, so one input record produces one physical output line.

true is a possible match. false excludes membership in that snapshot, assuming the producer used the same key rules. Results are not live database queries and cannot reflect changes after the snapshot. A checksum detects corruption, not provider identity.

New Job, match and job create pin the primary result to ./result-JOB_ID.txt in the launch working directory. Jobs shows that actual absolute Result path. Pause and restart/resume retain the original destination even from another working directory. match --output overrides the primary file or directory. Completion publishes atomically without overwriting; partial results and checkpoints stay in application storage. Existing jobs retain their original result locations.

E opens an optional additional Export with an absolute destination defaulting to ./result-JOB_ID.txt in the current working directory. A directory receives that generated filename. Exporting to the already-published primary file reports the verified destination without copying it. Export success reports the absolute path. Existing unrelated files are never overwritten. Paused/failed work is not a complete export."#),
    ("SETTINGS", "Four tabs: API, Rules, Secrets, Appearance. Left/Right changes tabs while the tab row is focused; Tab enters fields. Shift+Tab returns toward the tabs. Ctrl+S saves; Esc discards unsaved changes and returns Home. Pause active work before saving.\n\nAPI: DAT URL, optional OAuth scope, download/decoded-filter limits, OAuth deadline, DAT deadline, body idle deadline, Range part MiB, parallel requests, multipart text fields (JSON), transient GET retries, download request mode and loopback test mode. Defaults: 256 MiB download, 512 MiB decoded filter, OAuth 60 s, DAT 600 s, body idle 60 s, parts 16 MiB, parallelism 4, GET retries 2. Limits: 1-4096 MiB, deadlines 1-3600 s, parts 1-128 MiB, parallelism 1-16, retries 0-5. Tab scrolls to every field; arrows/Space cycle the Download request row. DAT/scope/request-mode/form changes require fresh sign-in and invalidate the current loaded snapshot. See DOWNLOAD REQUESTS for protocol details.\n\nRules: default key column, CSV delimiter, checkpoint interval, CPU threads, transformation, trim and headers. Checkpoint default: 100000 rows; allowed 1-1000000. A new job copies these defaults; existing jobs keep their pinned rule. Matching threads default to available CPUs capped at 32; choose 1-256. T changes a paused/prepared job without changing its keys or filter pin.\n\nSecrets: read-only session/access status and common token URL, plus Renew token. Appearance: IBM VGA or Menlo font; size 12-28 in steps of 2 (default 16); motion 0 off, 1 slow, 2 normal, 3 fast; scene Payment chip or Circuit flow; and the Ghostty profile action. Defaults: IBM VGA, normal motion, Payment chip. Changes to the host font require reopening the dedicated Ghostty launcher; the app cannot change the terminal's font itself. Ghostty Cmd +/- resizes, Cmd 0 resets."),
    ("DOWNLOAD REQUESTS", r#"Settings > API > Download request selects one of three modes. OAuth still uses the configured common token URL and JSON grants in all modes. Download configuration never creates a per-identifier request.

DAT transfers use HTTP/1.1 over verified TLS to avoid the repeated HTTP/2 body-stream resets observed on the provider route. OAuth retains its existing client and token behavior. Parallel mode still uses the configured number of connections; HTTP version selection does not remove its concurrency bound. A slowdown or stream reset alone does not prove rate limiting. HTTP 429/Retry-After and provider logs are distinct evidence; increasing parallelism or retries can amplify a quota problem.

Single GET streams one binary response. Parallel GET probes Range: bytes=0-0. HTTP 206 must provide an exact Content-Range, bounded total size and strong ETag. Each part sends Range and If-Match; every part must return HTTP 206, the same ETag, exact range/total and exact byte count. A changed/missing version, ignored later range, short body or limit violation aborts all work. If the initial probe returns HTTP 200, consume that same response as one stream: the server does not support Range. No extra GET is made for this fallback. Configure Single GET if the server supports ranges without strong validators.

Range part size and simultaneous requests bound work in flight. Parts finish independently but are assembled in original byte order, using locked RAM pages without disk staging. Downloaded and assembled data must pass PGMENC01 GCM/checksum and Filtering data validation before use. A larger concurrency value cannot add Range support to an endpoint.

Multipart POST sends multipart/form-data with bounded text metadata from Form fields. Enter a JSON object such as {"format":"dat","compression":"gzip"}. Field names use letters/digits/_.- (1-64 bytes), values at most 2048 bytes, at most 16 fields and 4096 bytes combined. No control characters, binary/file fields, credentials, tokens or AES keys belong here. These non-secret fields are saved in settings.json. The app generates the MIME boundary and Content-Type. Bearer authorization stays in the HTTP header. The server must explicitly support this POST contract; it is not a parallel download mode.

The OAuth deadline stays separate. The DAT deadline covers the initial request, HTTP retry waits, auth renewal after a 401, all parts and all body reads. It starts after initial fresh-job token acquisition. Body idle is the maximum wait for the next body chunk; it does not cut off slow response headers. Connect timeout remains 10 s. Transfer deadlines exclude joined in-memory decryption/decoding. Existing settings without a DAT deadline use at least 600 s and preserve a longer old request timeout.

GET responses 429, 502, 503 and 504, interrupted bodies and body-idle stalls share one retry budget for the entire DAT (default 2), including parallel parts. Exponential backoff with jitter stays inside one total deadline. Retry-After integer seconds up to 30 are honored; longer/date/invalid instructions stop the request instead of retrying too early. A part failure cannot multiply the configured retry count by the parallelism. Multipart POST bodies and transient HTTP errors are not automatically replayed; initial 401 authentication recovery remains separately bounded to one. Redirects and transparent HTTP Content-Encoding are rejected; GZip DAT is served as binary content.

When a GET body is interrupted, keep its prefix in locked RAM. With a strong ETag and known total length, request only the exact suffix using Range and If-Match; require matching 206 status, range/total, ETag and byte count. Changed versions or invalid ranges abort without publishing data. Without a safe validator/length, discard the prefix and restart the full GET. If a single-download recovery range receives a full HTTP 200, wipe the old prefix before consuming that response. Strict parallel parts reject an unexpected 200. No prefix or incomplete recovered DAT enters decryption/matching. Final AES-GCM and both checksums still apply.

Failure diagnostics include received/expected byte counts and a static transport category, without raw request errors or secrets. Exhausted recovery keeps the job UUID for R retry. The total deadline and pause/cancellation still terminate recovery and release locked buffers. Recovery 401 or changed filter/key requires correcting the session/source, then retrying that same job.

LIVE ACTIVITY shows the actual phase: input copy/scan, token request, DAT headers, transfer/retry, decryption/verification, input/checkpoint checks, matching and result publication. Header waits show a live seconds counter, attempt number and last HTTP failure even when the server sends no data. Transfer shows unique received MiB and rate; matching shows rows, percentage and rows/s every 8192-row wave, independently of durable checkpoints. Unknown totals have no percentage. A discarded transfer prefix reduces logical progress; Range probes and repeated bytes are excluded. P stops safely; the worker must finish stopping before R can retry.

HTTP 504 is a provider/gateway response; a longer local deadline cannot change its server-side deadline. After all attempts fail, Jobs keeps the prepared UUID. R retries/resumes that job; do not create it again. A failure before input preparation instead restores the New Job draft. No failed/partial DAT produces false matches."#),
    ("DAT AND SECURITY", "Download raw DAT or plain GZip DAT from the configured endpoint. Native PGMBLOOM v1 uses a 128-byte little-endian header; provider v1 uses a 64-byte big-endian header, bit array and SHA-256 trailer. Inspect reports wire_format, size, parameters and fingerprint. Unknown layouts, corrupt checksums, incompatible rules and size-limit violations fail closed. Provider snapshots are immutable.\n\nNative bloom build targets 0.1% false positives with capacity sizing margin and rejects overflow. Provider parameters come from its header and must satisfy supported bounds. False-positive rate is probabilistic, not a measured ratio for every dataset.\n\nHTTPS is required. Embedded credentials, URL query/fragment and redirects are rejected. Loopback HTTP is an explicit fixture-only option. Configure URLs externally; no service URL is built into the executable. Credentials and pending RAM downloads are bound to the exact source and OAuth profile. Never put tokens or secrets in shell arguments, logs, search queries or datasets.\n\nAfter the DAT download, matching stays local. No identifiers are sent to the API. Use the endpoint's declared digest rules and retain the exact DAT used by a resumable job."),
    ("CLI REFERENCE", r#"    tui                        Open interactive Home
    doctor                     Check source/RAM/jobs locally
    auth credentials           Hidden secret/refresh prompts (TTY)
    auth get-token             Acquire access token
    auth refresh-token         Renew at the same token URL
    bloom configure [OPTIONS]  Save external API configuration
    bloom token                Hidden bearer prompt (TTY)
    bloom download             Fetch and verify DAT
    bloom build --input FILE --output DAT [RULES]
    bloom inspect DAT [--max-memory-mib N]
    match --input FILE [--output FILE] [--threads N] [RULES]
    job create --input FILE [--threads N] [RULES]
    job list                   List jobs and progress
    job status JOB_ID          Print job JSON
    job run JOB_ID [--threads N]
    job resume JOB_ID [--threads N]
    job threads JOB_ID --threads N
    job delete JOB_ID --yes
    job checksum JOB_ID        Verify stored input/output
    job export JOB_ID [--output FILE_OR_DIRECTORY]

RULES: --key NAME or '#2' (alias --key-column), --headers true|false, --delimiter CHAR|tab, --transform none|sha256|sha256-raw, --trim true|false. Quote '#2' in shells where # starts a comment. Omitted match/job rules use saved defaults; bloom build uses native defaults. Inspect defaults to 512 MiB.

CONFIGURE: --dat-url URL (alias --url), --token-url URL, --client-id ID, --scope TEXT, --oauth [true|false], --allow-http-loopback [true|false]. The hidden legacy --reset-token-url accepts only the common token URL; distinct refresh URLs are rejected. There is no secret/token argument.

Each command and subcommand accepts --help. Root -h shows a summary; --help shows examples. Version: -V/--version. match creates and runs a durable job with its primary result in ./result-JOB_ID.txt by default. job create also pins that launch-directory destination for later run/resume. match --output selects a new primary file or an existing directory. job export makes an optional additional copy, defaulting to ./result-JOB_ID.txt in its current working directory. Output always reports an absolute path. Incomplete publication retains checkpoints for retry. Raw result lines are not streamed to stdout."#),
    ("SHELL EXAMPLES", r#"OAuth setup (replace all example URLs and identifiers):
    pgm-data bloom configure --dat-url https://data.example.com/filter.dat --token-url https://auth.example.com/token --client-id YOUR_CLIENT_ID --scope partners.read --oauth
    pgm-data auth credentials
    pgm-data auth get-token
    pgm-data bloom download
    pgm-data auth refresh-token

Optional static bearer mode instead of OAuth:
    pgm-data bloom configure --dat-url https://data.example.com/filter.dat --oauth false
    pgm-data bloom token

CSV with runtime hashing; alternate delimiter; Parquet:
    pgm-data match --input identifiers.csv --key identifier --transform sha256 --output result.txt
    pgm-data match --input keys.csv --key '#2' --headers false --delimiter ';'
    pgm-data match --input partners.parquet --key identifier

Build a native filter with the SAME key rules as its consumer:
    pgm-data bloom build --input members.csv --key identifier --transform sha256 --output native.dat
    pgm-data bloom inspect native.dat

Prepare, resume, verify and export:
    pgm-data job create --input partners.csv --key identifier
    pgm-data job run JOB_ID
    pgm-data job resume JOB_ID
    pgm-data job checksum JOB_ID
    pgm-data job export JOB_ID --output complete.txt"#),
    ("FILES AND RECOVERY", r#"PGM_DATA_HOME overrides the application data directory. On macOS the default is ~/Library/Application Support/pgm-data. Other systems use $XDG_DATA_HOME/pgm-data or ~/.local/share/pgm-data. Relative override paths resolve from the process working directory. The executable name and storage namespace remain pgm-data.
    matching/settings.json       Source, limits and rule defaults
    matching/filters/            Empty legacy cache (never read)
    matching/ram-only-v1.json     Memory-policy migration marker
    matching/locks/JOB_ID.lock   Stable run/delete lock
    matching/deleting/           Interrupted deletion cleanup
    matching/jobs/JOB_ID/        Input snapshot and job/checkpoint state
    matching/jobs/JOB_ID/result.txt  Legacy jobs only
    ./result-JOB_ID.txt          Default primary output for new jobs
    matching/events.log          Lifecycle diagnostics; no key values
    ghostty-ansi.config          Dedicated terminal font profile

Jobs pin their original input, rule and exact filter fingerprint. Checkpoints commit at the configured row interval and on pause, with ordered output and prefix verification. Restart truncates uncommitted bytes before resuming. Parquet resumes by row offset. A pause retains its job filter in RAM. After process exit, resume downloads the DAT again and requires the exact pinned fingerprint; a newer snapshot cannot replace it.

Interrupted run: select the job and R, or job resume JOB_ID. Pinned DAT unavailable or changed: restore that exact snapshot at the source, or create a new job against the current snapshot. Checksum/rule mismatch: restore the original input/filter or create a new job; do not edit checkpoint files. Disk-full/I/O error: make space and retry from the last committed checkpoint. Existing export destination: choose a new path.

HTTP 401/sign-in rejected: check ID/secret/token URL and required scope, then sign in again. DAT authentication failed: sign out and enter the correct Base64 AES key; check the source uses the confirmed GCM framing. Invalid DAT after authentication: check the plaintext is raw/GZip PGMBLOOM, not HTML/JSON. Transformation error: use None only for existing provider digests, SHA-256 for raw keys. Null/empty/unsupported key: correct the input and create a new job. Errors never become false results."#),
    ("EXIT STATUS AND NOTES", "0: command completed successfully; a gracefully paused CLI run also returns 0, so inspect job status for COMPLETED. 1: command/runtime error. 2: Clap usage/argument error, or interactive launch without a terminal. --help and --version return 0.\n\nCtrl+C requests cancellation and waits for checkpoint/worker cleanup; completion and cancellation can race, so verify final job state and result path. Use job status JOB_ID for automation. A successful token request or DAT download alone does not mean a matching job completed.\n\nSee also: pgm-data COMMAND --help and https://github.com/osvaldoandrade/matchpoint/wiki. The Help browser, plain manual and roff export share this content. No shell command or external pager is launched by Help."),
    ("RESIDENT DAT MEMORY", r#"Each job owns one immutable Filtering allocation. All its CPU workers share that allocation. Anonymous pages are locked with mlock before filling; no downloaded DAT, GZip stage or bit array is written to a file. If the OS rejects page locking, loading stops with a visible error and no disk/swap fallback.

Completion, matching error and job deletion release that job's filter after joining its workers. The mapping is zeroized before unmapping. Pause retains the filter for that SAME job. Sign-out or process exit releases all remaining filters. The next job requests a fresh client_credentials token and downloads a new DAT, even when the previous access token is valid or the new DAT has identical bytes. Existing refresh/cooldown and one-retry DAT-401 rules still apply.

D may preload the next job's DAT. A preload is claimed once; it is not a reusable cross-job cache. Old recognized disk cache files are retired on the first launch of this memory policy. Native bloom build still writes an explicitly requested producer artifact; that is not a downloaded cache.

The configured memory limit bounds resident filter bits across active/paused jobs. Peak RAM also includes locked compressed transfer chunks, decoding buffers, CSV/Parquet rows and bounded worker results. Finish or delete paused jobs to release their allocation when the budget is full.

The guarantee applies to application-owned DAT mappings during a running Unix process. HTTP/TLS/codec temporary buffers are not all locked mappings. It does not promise forensic erasure of old SSD sectors, protection from privileged memory dumps or hibernation, or wiping by an application killed without cleanup. The kernel still releases process mappings on termination."#),
    ("PARALLEL MATCHING", r#"Choose N in New Job, Settings > Rules, T on Jobs, or --threads on match/job create/run/resume. Allowed 1-256; default available CPUs capped at 32. Choose 1 for the serial baseline. Existing job metadata without threads reads as 1.

The coordinator uses the real CSV parser, so quoted delimiters, escaped quotes and embedded newlines remain intact. It distributes parsed record chunks to N persistent CPU workers that query the same read-only reference allocation. It never splits raw CSV by newline or copies the reference per thread. Parquet uses the same bounded matching pipeline.

At most 8192 records are in a wave. Each worker has a capacity-1 input queue; the result queue has capacity N. One coordinator writes completed chunks in original order, preserving duplicates, leading zeros and original output keys. Scheduling does not alter matching rules or checkpoint identity. Lookups require no per-key lock on the reference.

P/Ctrl+C stops at a committed row boundary and joins all workers. A pause keeps its filter; a runtime failure does not publish a complete result. Thread count can change between runs of a paused job. Performance depends on CPU, storage, transformation and filter size; more threads are not guaranteed to be faster.
    pgm-data match --input identifiers.csv --key identifier --transform sha256 --threads 8
    pgm-data job resume JOB_ID --threads 4
    pgm-data job threads JOB_ID --threads 2"#),
    ("DELETE JOBS", r#"Jobs > X or Delete opens a confirmation for the selected UUID/dataset. Cancel is selected initially. Esc or N cancels; Y deletes, or select Delete with arrows/Tab and Enter. /delete opens the same confirmation.
    pgm-data job delete JOB_ID --yes

Deletion removes only this job's stored input snapshot, checkpoint, metadata and internal result/partial output. It releases a paused job's DAT. The original source file, external primary result and exported copies stay intact. Deletion is irreversible without a separately owned backup. No bulk deletion is implied.

A job held by another matching process cannot be deleted or change thread count. Stop/pause it first. UUID/prefix resolution must be unambiguous; links and mismatched metadata are refused. Stable locks live outside the removed directory. The job directory is atomically staged before removal; an interrupted cleanup is retried on the next launch."#),
    ("ENCRYPTED DAT AND AES KEY", r#"Home includes DAT key: a hidden, padded standard Base64 value encoding exactly 32 AES key bytes. This is not a password or a value to hash. Tab moves through Client ID, Client secret, Token URL, DAT key and Sign in. Enter advances through the first three fields and submits from the key/button. Both secret drafts are cleared on submission/error/cancellation; Help preserves masked form drafts.

The endpoint body is PGMENC01 header[32] || nonce[12] || ciphertext || tag[16] || SHA256[32]. All header integers are big-endian: magic PGMENC01, version 1, algorithm 1, plaintext length, eight reserved zero bytes. Require body length = declared plaintext length + 92 (no overflow), then verify SHA256 over all bytes before the 32-byte trailer. The whole-file SHA256 is a different digest. Open AES-256-GCM with nonce at [32..44], ciphertext/tag at [44..len-32], and the exact header [0..32] as AAD. Empty AAD and the old headerless layout are rejected. Authenticate BEFORE GZip decoding, Filtering data parsing or publication. Wrong headers, sizes, checksums, keys, nonce/ciphertext/tag, truncation and plaintext downgrade fail closed. No failed download becomes false matches. Raw/GZip native/provider DAT payloads remain supported after decryption.

The decoded key stays in locked RAM for the login session, outside Settings, Keychain, job metadata, logs, shell arguments, environment and HTTP requests. L signs out; sign in again to change it. Source-profile changes also clear the key and require login. Each new job still requests a fresh OAuth token and encrypted DAT; its Filtering data is wiped on completion, while the login key lasts until logout/exit.

CLI match, job run/resume and bloom download request the AES key in a hidden terminal prompt for that process. Noninteractive key flags/environment injection are unsupported. Build/inspect use explicit plaintext local producer files. A trusted library caller without an installed key retains the legacy plaintext reader; authenticated application sessions require the key and never fall back.

Owned transfer/decryption/key mappings are locked and zeroized on final drop. Peak memory includes transfer coalescing and authenticated compressed plaintext plus decoded reference bits. HTTP/TLS/codec/crypto-library internals are not all locked mappings; privileged dumps, hibernation and wiping after SIGKILL/power loss are outside the process guarantee.

Troubleshooting: malformed Base64/length is rejected before a token request. OAuth failure keeps Home locked and installs no AES key. GCM authentication failure after login requires correcting the key/source; sign out to re-enter the key. PGMENC01 header/size/checksum errors require a valid source envelope. After GCM success, the original DAT checksum, memory budget, transformation and pinned fingerprint checks still apply."#),
    ("DATASETS AND SCALE", r#"Use customer IDs, device IDs, product SKUs, account keys or event identifiers. Dataset domain is unrestricted; supported input formats and key types are described in NEW JOB AND INPUT. Select one column as the lookup key. For composite keys, produce a deterministic string column before importing and use the same representation in the reference.

Example customers.csv:
    customer_id,segment
    customer-000042,retail
    customer-000099,business
    customer-000042,retail

If the reference producer hashed raw customer IDs with SHA-256:
    pgm-data match --input customers.csv --key customer_id --transform sha256 --threads 8

For a very large Parquet dataset with raw device IDs and the same SHA-256 contract:
    pgm-data match --input events.parquet --key device_id --transform sha256 --threads 8

The selected keys are streamed in bounded batches; the whole input is not loaded into RAM. Workers share one compact, immutable Filtering snapshot. Reference size depends on its key count and false-positive target and must fit the configured locked-memory budget. Throughput depends on storage, decoding, key transformation and CPU resources; no fixed processing rate is guaranteed.

Safe matching means identifiers stay local, encrypted DATs are authenticated before use, and malformed input or failed integrity checks stop the job. Errors never become false results. true means possible presence; verify positive candidates against an authoritative source when exact membership is required. false means absence from this snapshot only when producer and consumer key rules agree. Results describe the reference at snapshot time.

Exports contain original keys. Protect input files, job storage and exports according to their sensitivity; the application does not encrypt those files."#),
];

pub fn text(roff: bool) -> String {
    let mut output: String = if roff {
        ".TH MATCHPOINT 1 \"October 15, 2026\" \"pgm-data\" \"User Commands\"\n".into()
    } else {
        "MATCHPOINT(1)                 User Commands                 MATCHPOINT(1)\n".into()
    };
    for (title, body) in CHAPTERS {
        output.push_str(&if roff {
            format!(".SH {title}\n")
        } else {
            format!("\n{title}\n")
        });
        for line in body.lines() {
            if roff {
                let escaped = line.replace('\\', "\\e");
                output.push_str(&if line.is_empty() {
                    ".PP\n".into()
                } else if line.starts_with("    ") {
                    format!(".nf\n\\&{escaped}\n.fi\n")
                } else {
                    format!("\\&{escaped}\n")
                });
            } else {
                output.push_str(line);
                output.push('\n');
            }
        }
    }
    output
}

#[derive(Default)]
pub struct Pager {
    pub(crate) chapter: usize,
    offset: usize,
    query: String,
    entering: bool,
}
struct Row {
    text: String,
    start: usize,
    end: usize,
}
fn wrapped(chapter: usize, width: usize) -> Vec<Row> {
    let width = width.max(8);
    let mut result = Vec::new();
    let mut offset = 0;
    for line in CHAPTERS[chapter].1.lines() {
        let indent = if line.starts_with("    ") { "    " } else { "" };
        let mut rest = line.trim_start();
        let mut start = offset;
        let available = width - indent.len();
        // Static ASCII text keeps source offsets for matches across visual wraps.
        while rest.len() > available {
            let split = rest[..available]
                .rfind(' ')
                .filter(|at| *at > 0)
                .unwrap_or(available);
            let end = offset + line.len() - rest.len() + split;
            result.push(Row {
                text: format!("{indent}{}", &rest[..split]),
                start,
                end,
            });
            rest = rest[split..].trim_start();
            start = end;
        }
        result.push(Row {
            text: format!("{indent}{rest}"),
            start,
            end: offset + line.len(),
        });
        offset += line.len() + 1;
    }
    result
}
fn geometry(area: Rect) -> (Rect, Rect) {
    let side = if area.width >= 98 { 27 } else { 0 };
    (
        Rect::new(area.x, area.y, side, area.height.saturating_sub(3)),
        Rect::new(
            area.x + side,
            area.y,
            area.width.saturating_sub(side),
            area.height.saturating_sub(3),
        ),
    )
}
fn position(rows: &[Row], offset: usize) -> usize {
    rows.iter()
        .position(|row| row.end > offset || row.start == offset)
        .unwrap_or(rows.len().saturating_sub(1))
}
impl Pager {
    fn matches(&self, width: usize) -> Vec<(usize, usize)> {
        if self.query.is_empty() {
            return Vec::new();
        }
        let query = self.query.to_lowercase();
        let mut result = Vec::new();
        for (i, (title, body)) in CHAPTERS.iter().enumerate() {
            let offsets = body
                .to_lowercase()
                .match_indices(&query)
                .map(|(at, _)| at)
                .collect::<Vec<_>>();
            for (j, line) in wrapped(i, width).iter().enumerate() {
                if offsets.iter().any(|at| *at >= line.start && *at < line.end)
                    || (j == 0 && title.to_lowercase().contains(&query))
                {
                    result.push((i, j));
                }
            }
        }
        result
    }
    pub fn key(&mut self, key: KeyEvent, size: (u16, u16)) -> bool {
        let (_, body) = geometry(Rect::new(
            0,
            0,
            size.0.saturating_sub(2),
            size.1.saturating_sub(4),
        ));
        let width = body.width.saturating_sub(2) as usize;
        let height = body.height.saturating_sub(2).max(1) as usize;
        if self.entering {
            match key.code {
                KeyCode::Esc => self.entering = false,
                KeyCode::Backspace => {
                    self.query.pop();
                }
                KeyCode::Char(c)
                    if !c.is_control()
                        && !key
                            .modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                        && self.query.len() + c.len_utf8() <= 128 =>
                {
                    self.query.push(c)
                }
                KeyCode::Enter => {
                    self.entering = false;
                    if let Some((chapter, scroll)) = self.matches(width).first() {
                        self.chapter = *chapter;
                        self.offset = wrapped(*chapter, width)[*scroll].start;
                    }
                }
                _ => {}
            }
            return false;
        }
        let rows = wrapped(self.chapter, width);
        let end = rows.len().saturating_sub(height);
        let mut scroll = position(&rows, self.offset);
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return true,
            KeyCode::Left | KeyCode::BackTab | KeyCode::Char('h') => {
                self.chapter = (self.chapter + CHAPTERS.len() - 1) % CHAPTERS.len();
                scroll = 0;
            }
            KeyCode::Right | KeyCode::Tab | KeyCode::Char('l') => {
                self.chapter = (self.chapter + 1) % CHAPTERS.len();
                scroll = 0;
            }
            KeyCode::Up | KeyCode::Char('k') => scroll = scroll.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => scroll = (scroll + 1).min(rows.len() - 1),
            KeyCode::PageUp | KeyCode::Char('b') => scroll = scroll.saturating_sub(height),
            KeyCode::PageDown | KeyCode::Char(' ') => {
                scroll = (scroll + height).min(rows.len() - 1)
            }
            KeyCode::Home | KeyCode::Char('g') => scroll = 0,
            KeyCode::End | KeyCode::Char('G') => scroll = end,
            KeyCode::Char('/') => {
                self.query.clear();
                self.entering = true;
            }
            KeyCode::Char('n' | 'N') => {
                let hits = self.matches(width);
                let at = (self.chapter, scroll);
                let hit = if key.code == KeyCode::Char('n') {
                    hits.iter().find(|hit| **hit > at).or(hits.first())
                } else {
                    hits.iter().rev().find(|hit| **hit < at).or(hits.last())
                };
                if let Some((chapter, row)) = hit {
                    self.chapter = *chapter;
                    scroll = *row;
                }
            }
            _ => {}
        }
        self.offset = wrapped(self.chapter, width)[scroll].start;
        false
    }
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let (side, body) = geometry(area);
        if side.width > 0 {
            let lines = CHAPTERS
                .iter()
                .enumerate()
                .skip(
                    self.chapter
                        .saturating_sub(side.height.saturating_sub(3) as usize),
                )
                .map(|(i, (title, _))| {
                    Line::styled(
                        format!("{:02} {title}", i + 1),
                        if i == self.chapter {
                            Style::default().fg(p::MINT).bold()
                        } else {
                            Style::default().fg(p::MUTED)
                        },
                    )
                })
                .collect::<Vec<_>>();
            frame.render_widget(Paragraph::new(lines).block(block(" CONTENTS ")), side);
        }
        let title = format!(
            " {:02}/{:02} {} ",
            self.chapter + 1,
            CHAPTERS.len(),
            CHAPTERS[self.chapter].0
        );
        let border = block(&title);
        let inner = border.inner(body);
        frame.render_widget(border, body);
        let rows = wrapped(self.chapter, inner.width as usize);
        let hits = self.matches(inner.width as usize);
        let start = position(&rows, self.offset);
        let lines = rows
            .iter()
            .enumerate()
            .skip(start)
            .take(inner.height as usize)
            .map(|(index, line)| {
                Line::styled(
                    line.text.clone(),
                    if hits.contains(&(self.chapter, index)) {
                        Style::default().fg(p::BG).bg(p::MINT)
                    } else if line.text.starts_with("    ") {
                        Style::default().fg(p::MINT)
                    } else {
                        Style::default().fg(p::FG)
                    },
                )
            })
            .collect::<Vec<_>>();
        frame.render_widget(Paragraph::new(lines), inner);
        let status = if self.entering {
            format!(" /{}▏  Enter find / Esc cancel", self.query)
        } else if !self.query.is_empty() {
            format!(
                " /{}  {} matching lines / n next / N prev",
                self.query,
                hits.len()
            )
        } else {
            format!(
                " MANUAL  {:02}/{}  lines {}-{}/{}",
                self.chapter + 1,
                CHAPTERS.len(),
                start + 1,
                (start + inner.height as usize).min(rows.len()),
                rows.len()
            )
        };
        let keys = if area.width >= 78 {
            " j/k scroll  Space/b page  g/G ends  h/l chapters  / search\n n/N matches  Tab next chapter  Shift+Tab previous  q/Esc back"
        } else {
            " j/k scroll  h/l chapters  / search\n Space/b page  n/N matches  q/Esc back"
        };
        frame.render_widget(
            Paragraph::new(vec![
                Line::styled(status, Style::default().fg(p::MINT)),
                Line::from(keys.lines().next().unwrap_or_default()),
                Line::from(keys.lines().nth(1).unwrap_or_default()),
            ]),
            Rect::new(area.x, area.bottom().saturating_sub(3), area.width, 3),
        );
    }
}

#[cfg(test)]
mod tests;
