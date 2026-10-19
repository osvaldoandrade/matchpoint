# Local matching recovery

Jobs shows a LIVE ACTIVITY panel while work runs. Read its phase before assuming
matching is slow: copying/scanning input, requesting a token, waiting for DAT
headers, downloading, retry backoff, decrypting/verifying or matching/publishing.
The header-wait clock advances even when the server sends nothing; attempts and
the last HTTP status remain visible after a retry. A 504 with zero rows means the
provider/gateway has not delivered a usable DAT. P cancels the wait safely; R
retries the prepared UUID after the worker has stopped. Download bytes/rate and
matching rows/rate update live, independently of durable checkpoint frequency.
Unknown totals never display a fabricated percentage. Retransmitted/probe bytes
are excluded, and a discarded prefix removes its bytes from logical progress.

If download/CRC/checksum/header validation fails, do not start matching. Check the configured source and DAT metadata with `pgm-data bloom inspect`; retry an explicit download after repairing the source. Do not replace a pinned job's filter with a different snapshot. No keys are sent to the source.

If a job pauses, the same running TUI keeps its locked filter. R resumes it without token/download. After process exit, `pgm-data job resume ID --threads N` obtains a fresh token/DAT; the pinned input/filter/rule and durable prefix are verified. Restore the exact snapshot at the configured API source on a fingerprint mismatch or create a new job. No disk cache is available for offline restart. Do not interpret an incomplete output as a negative result. On disk failure, free space and resume after verifying the preserved checkpoint.

Rollback restores only the task source snapshot and rebuilds. Preserve both `jobs/` and `matching/`, preferences and Keychain. This runbook covers local execution; no deployed canary, paging or production recovery has been performed.

Home login takes client ID, fresh hidden client secret, common token URL and a hidden Base64 AES-256 DAT key; only a valid issued token unlocks the interactive session. Settings > API edits DAT/scope/limits; Settings > Secrets is read-only metadata plus Renew token. To change client ID, secret or token URL, sign out with L and log in again. Both client_credentials and refresh_token POST to token_url. Legacy reset_token_url is ignored; distinct historical profiles require fresh login. No secret migration is performed.

A 2xx response without a usable access_token/Bearer type/positive expiry, denial or cancellation keeps Home locked. After provider success, Home keeps client secret and rotating refresh token in zeroizing session RAM; it never calls Keychain during login, new jobs or renewal. Sign-out/profile change/exit drops them. Renew token uses the session refresh token at the same URL; invalid_grant permits one acquisition fallback there. Errors stop before matching. Explicit CLI auth credentials/get-token/refresh-token retains saved-credential support in Keychain.

DAT transport: Settings > API selects Single GET, Parallel GET (HTTP Range) or
Multipart POST. DAT deadline defaults to 600 seconds, OAuth to 60, and body idle
to 60. A header timeout differs from a body stall and an HTTP gateway error.
DAT now uses HTTP/1.1 over verified TLS; OAuth is unchanged. Repeated HTTP/2 body
resets were observed after successful 200/206 headers in Single and Parallel
modes. The scoped HTTP/1.1 check delivered and authenticated the complete
169,768,248-byte DAT in 7.254s without recovery. This is a transport mitigation,
not proof of a provider quota policy. Preserve actual protocol/status/offset/time
evidence and distinguish 429 from a reset or idle timeout. User connection limits
remain configured; do not increase fan-out to diagnose a possible rate limit.
GET 429/502/503/504, interrupted bodies and body-idle stalls share 2 retries by
default across the whole DAT, including concurrent parts, bounded by one deadline
with backoff/jitter. A strong ETag and known size permit Range/If-Match suffix
recovery; otherwise discard/wipe the prefix and restart the GET. A full 200 to
single-download recovery replaces the prefix; parallel parts require exact 206.
Changed version/range/total aborts. Diagnostic byte counts and static cause codes
identify where recovery failed; raw request errors and secrets are never logged. POST has no transient-error retries; initial 401 recovery remains bounded to one. A persistent HTTP 504
requires provider/gateway repair; extending the client deadline alone cannot
fix its server deadline. The prepared job remains under Jobs: R retries its UUID
instead of creating another job. Existing duplicates require explicit deletion.

Parallel mode uses a one-byte Range probe, exact Content-Range and strong ETag.
Every part must match the version/length/range; any mismatch aborts. A 200 probe
uses that same response as a single stream. Default parts: 16 MiB and concurrency
4; limits 1-128 MiB and 1-16 requests. Multipart POST accepts a JSON object of
non-secret text fields in Settings; it needs explicit server support. No secrets
in form fields: these values are persisted. See DOWNLOAD REQUESTS in F1/manual.

Changing URLs/client ID/scope selects a new credential/resident profile. Re-enter credentials for that profile rather than copying old items automatically. A new manual bearer discards an unused resident preload. Locks fail fast if another source/auth operation is active; wait for it to finish and retry. Do not edit a running job's filter pin. There are no production URL defaults or implicit endpoint paths.

The reader distinguishes native PGMBLOOM from the provider's 64-byte big-endian PGMBLOOM, in raw/plain GZip delivery. A bad checksum, unknown flags or mismatched length stops before publishing the snapshot. Provider input already in SHA-256 hex requires transformation None; raw identifiers require SHA-256. Invalid digest strings stop rather than becoming false. Preserve provider DAT bytes and fingerprint on resume; do not rewrite them through the native builder or replace a pinned native filter. The native builder/protocol remains unchanged.


DAT loading fails closed when `mlock` is denied. Check the OS locked-memory
limit and configured download/decoded MiB. Allow space for compressed staging
plus the decoded filter and paused jobs. Do not enable a disk/swap fallback.
Completion/error/delete releases the job filter; a new OAuth job always acquires
a fresh client_credentials token and DAT. Normal exit/sign-out joins workers
before zeroizing and unmapping. Privileged dumps/hibernation and abrupt-kill
wiping are outside this process-level guarantee. HTTP/TLS/codec temporary
buffers are not all locked mappings.

T on Jobs or `job threads ID --threads N` changes a prepared/paused job (1–256).
N workers share one Bloom. Lower N if thread creation fails or overhead dominates;
resume preserves the filter pin and ordered checkpoint. A worker failure stops
publication, and all worker handles are joined. No partial result means `false`.

X/Delete on Jobs opens confirmation (default Cancel). `job delete ID --yes`
removes the selected stored snapshot/results/checkpoint, never the original or
external export. A running job is refused by stable and legacy locks. If cleanup
fails after staging, fix storage permissions/space and relaunch to retry only
recognized `matching/deleting/UUID-UUID` stages. Do not recreate/edit job IDs or
follow symlinks. Unknown stage names are left untouched.

The first memory-policy launch retires recognized legacy filters/current.json
and writes `ram-only-v1.json`. Stop all old binaries before this one-time
transition; do not run an old caching binary concurrently. Deleting old cache
files is not forensic erasure of SSD sectors. See ADR-0014 for rollback limits.


Encrypted DAT: require PGMENC01 header[32] + nonce[12] + ciphertext + tag[16]
+ SHA256[32]. Validate BE version/algorithm=1, reserved zeros, checked plaintext
length+92 equal to the body length, then SHA256 over bytes before the trailer.
Use the exact 32-byte header as AES-256-GCM AAD; nonce is at [32..44], and
ciphertext/tag at [44..len-32]. Empty AAD/headerless fallback is not supported.
Home validates padded standard Base64 encoding exactly 32 key bytes. It does not
send the AES key to OAuth/DAT endpoints or persist it in Keychain/settings/jobs.
GCM authenticates before GZip/Bloom decoding. On authentication failure, sign out
with L and re-enter the correct key; confirm server framing and transport bytes.
Do not bypass tag checks, guess CBC/password derivation, or retry plaintext.
The login key lasts for the session; each completed job still releases its Bloom.
CLI download/matching prompts for a key without echo. No key flags/env injection.
Existing local plaintext producer build/inspect are unchanged. The key is wiped
on final drop after logout/profile change/exit; crypto-library temporary buffers
are not all locked application mappings.

PGMENC01 checksum/header/size failure: repair the source envelope or transport;
do not blame/rotate the AES key before these checks pass. A public recomputed
SHA does not authenticate changes: GCM must still verify the header and payload.
Do not replace a job pin with ciphertext/whole-file SHA. It remains the verified
plaintext Bloom trailer digest. The documented sample size and shortened hashes
are examples, not application constants. See ADR-0016 and the envelope contract.
