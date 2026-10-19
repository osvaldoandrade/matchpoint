# ADR-0001 — Numeric terminal workspace and tabbed settings

## Status

Accepted for local implementation on 2026-10-14. The user requested this replacement; release and production promotion are outside this task.

## Context and decision

The user rejected the hooded ANSI figure and the current Jobs/Batches layout on 2026-10-14. The application maintainer will replace that figure with rotating geometry constructed from numeric terminal cells, put API, secrets, processing and appearance controls inside one Settings screen, and render application copy in English. Previous heuristic scores describe the rejected interface and cannot approve this version. The target remains three independent Nielsen evaluations with an average strictly greater than 9.8 on the same source revision.

The TUI remains an adapter over the existing application engine. Home, Jobs and Settings provide navigation; a job owns its batches, counts, events and integrity results. Settings owns staged edits and applies them explicitly. Existing job configuration remains associated with its job. A reserved record includes queued or executing work and does not count concurrent network calls. Threads remain the upper bound on concurrent calls; RPS remains a separate request rate limit.

Numeric tesseract and lattice scenes use deterministic rotations, perspective projection and depth ordering. They share the terminal palette and support reduced motion. The scene uses no image assets, external renderer, additional dependency or network request. Jobs uses an always-visible list beside selected-job details at normal widths, and a list/detail switch at 40 columns. Details group Overview, Batches, Events and Integrity; batch pagination uses the actual number of visible rows.

## Contracts and invariants

HTTP/OAuth contracts, record state semantics, SQLite schemas, checksum algorithms and worker cancellation remain authoritative in the engine. Secrets remain masked, bounded and zeroized in the form, stored through the existing secret store, and excluded from history, rendering and diagnostics. API and Secrets tabs share a staged profile. Changed identity/destination requires explicit credentials. Optional processing and appearance preferences must read old JSON files and preserve CLI override precedence. Remote requests still require explicit authorization and the existing endpoint checks.

## Reversibility

If the replacement breaks local interaction or data invariants, the maintainer will restore task-owned source from the pre-redesign snapshot and rebuild within ten minutes. Source restoration never replaces job directories or Keychain data. Optional preference additions use defaults when missing. No database migration, backfill, production deployment or credential rotation is part of this decision.

## Alternatives

Further polishing of the hooded figure does not satisfy the user's rejection. Keeping separate API/appearance entries in the home menu does not satisfy tabbed Settings. A bitmap or web view does not provide the requested numeric ANSI animation. A full engine rewrite would expand the failure surface without serving the UI request.

## Scope of delivery gates

Local tests, builds, secret-safety checks, rendered evidence, runtime fixture flows, performance measurements and independent reviews apply. PR/CI approval, canary deployment, on-call notification, infrastructure dashboards, signed releases and staging migration are not applicable: this checkout has no Git repository and the task authorizes a local terminal application redesign, not publication or production changes. This is a scope statement, not a claim that those gates passed.

## References

- `docs/tui-redesign-plan.md`
- `docs/ux-evaluation.md`
- Existing application boundary: `src/engine.rs`, `src/engine/configuration.rs`
- Existing terminal boundary: `src/tui.rs`, `src/tui/terminal.rs`

## T4 scope amendment

LOC volume promotes the replacement from T3 to T4. The implementation stays local and reversible; neither code size nor a tooling instruction authorizes a production release. The plan records the full ordered design re-review and local pre-mortem. The saved original source is the retained rollback path for this explicit replacement. Runtime activation of a parallel old UI, production staged ramp, incident staffing, tenant-isolation probes, migration reconciliation and signed release/SBOM publication do not apply to this terminal-only local delivery. Existing authorization checks remain unchanged. Code coverage and mutation floors, when not measured, remain unverified; no exception turns them into passing evidence.
