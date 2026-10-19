# ADR-0013: MATCHPOINT identity and shared terminal manual

Status: accepted for the user-requested local implementation, 2026-10-15.

The active terminal repeated its partner title and clipped a short Help
paragraph. Use MATCHPOINT once in the screen frame and describe the product
as Ultra-fast dataset matching. The binary/storage names remain pgm-data;
the product still performs probabilistic Bloom membership, not exact database
verification. The tagline is positioning, not a newly measured throughput SLO.

Keep one authored, static English manual in the matching presentation adapter.
A concrete pager supplies chapters, word wrapping, less-style keys and literal
search. F1 opens the relevant chapter and q/Esc restores the prior form. Export
the same chapters as plain text or roff through an additive `manual [--roff]`
command, before application/Keychain initialization. Keep existing commands.

Alternatives: an external `less` process introduces terminal/process handling;
duplicated Markdown/man/TUI sources drift; one large Help paragraph clips.
The in-process pager requires explicit resize/search tests, but no dependency,
network call, persistence migration or new worker. A feature flag would add
configuration for the immediate local presentation change explicitly requested
by the user; no production rollout is part of this work.

Threat delta: search is bounded to 128 UTF-8 bytes and control characters are
ignored. Only static trusted text renders in the manual; queries are inert and
never logged. Help retains the existing form buffer without copying credentials
and never executes slash commands. Printing documentation does not open user
storage. Apply the existing shutdown path on Ctrl+C. T-6 telemetry exception:
logs/metrics/traces for local documentation keystrokes add no operational value.

Blast radius: terminal headings and documentation only. No changes to matching,
auth checks, DAT codecs, stored settings/jobs/results or animation implementations.
Rollback restores the task-only backup named by `/tmp/pgm-manual-backup-path`,
removes new task modules and rebuilds. The same datasets/jobs remain readable.
Verification commands and honest local/publication gate status live in
`docs/manual-ui-plan.md`; no release, merge or independent approval is implied.
