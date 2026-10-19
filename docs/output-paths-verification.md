# Absolute output/export paths — local verification

2026-10-16. Tier T2. Jobs now displays its actual absolute result path, and
exports default to ./result-JOB_ID.txt in the process working directory. The
default is displayed as an absolute path. Durable job storage stays in the
existing application data root; no user job/result was relocated or exported
by this task. Only tiny synthetic test/render jobs were created.

## Behavior

- `Service::result_path` returns an absolute path even for a relative data root.
  Service root identity remains unchanged, preserving profile hashing.
- The real Service root is bound to the interactive UI; the guessed relative
  matching/jobs prefix is removed. Completed-job paths wrap before secondary
  graph/help copy; compact layouts prioritize the result path.
- E prefills the absolute CWD/result-JOB_ID.txt destination. The full absolute
  value also appears below the editable field, whose text can scroll horizontally.
  Export completion reports the absolute destination with enough status rows to
  wrap it at the four supported viewport sizes.
- CLI `match` without --output exports a completed result to that CWD default.
  `job export JOB_ID` now defaults --output to '.', preserving the public Rust
  field type and explicit --output compatibility. Existing directories receive
  the UUID filename; an explicit new filename overrides it.
- The shared resolver validates the UUID. Existing no-overwrite atomic copy,
  complete-only and checksum checks remain. A failed export retains the durable
  completed job; exporting it later does not require matching again.

## Verification

| Command | Result |
| --- | --- |
| `cargo test --offline --all-targets --all-features` | 249 passed; 0 failed |
| `cargo clippy --offline --all-targets --all-features -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo run --offline --example matching_review -- target/output-paths-review` | Jobs/Export plus existing screens rendered at 40×18, 60×22, 80×24 and 120×36 |
| `cargo build --offline --release --bin pgm-data --bin pgm-bench` | Passed |
| `cargo build --offline --bin pgm-data` | Passed; launcher binary rebuilt |

Three new regression cases cover: actual CLI export from isolated child working
directories, default/directory/relative filenames with spaces, absolute stdout
paths and duplicate-export refusal; result lookup from a relative root while
preserving its identity; and a real tiny completed job rendered with its absolute
path, absolute prefilled export and full success receipt at all four viewports.
The UI test compares actual exported bytes with the durable source. macOS /var
versus /private/var aliases are handled in fixture expectations without changing
production path semantics. Tests do not alter global process cwd or contact APIs.

Logs: `/tmp/pgm-output-paths-{tests,clippy,fmt,render,release,debug}.log`.
Render artifacts: `target/output-paths-review/`. Visual inspection includes the
compact absolute result and the default Export form; text assertions reconstruct
the entire displayed path across wrapped lines, rather than checking only a prefix.

## Local T2 Definition of Done

- [x] Classification, placement/CLI contract, ordered design consultation and plan recorded before edits.
- [x] Shared path resolution and actual-root binding implemented without schema/profile identity changes.
- [x] Default/directory/explicit export, no overwrite, absolute output and four-size UI regressions passed.
- [x] Full local tests, lint, format, renders and debug/release builds passed.
- [x] README and shared terminal/Unix manual updated.
- [x] Protected animation/font/palette and dependency hashes match the source backup.
- [x] Local scoped self-review completed; no user data/config migration or API operation.

No independent review, branch-coverage floor, CI, PR/merge or publication approval
is claimed. No dependency, network, auth/crypto or concurrency change was needed.
The default export requires writable CWD; permission/collision failure does not
remove the durable result. No automatic TUI copy occurs until E/Enter.

Rollback restores only task source/docs from `/tmp/pgm-output-paths-backup-path`
and rebuilds. Preserve existing jobs, results, exports, settings and secrets.
The user's running terminal was not restarted; reopen the application to use
the rebuilt UI. Existing completed jobs need no reprocessing: E exports them.
