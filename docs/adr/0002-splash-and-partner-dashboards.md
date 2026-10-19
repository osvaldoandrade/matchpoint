# ADR-0002 — Splash identity and static partner import dashboards

## Status and context

Accepted for local implementation on 2026-10-14 from the user's explicit six-part correction. The large PGM wordmark, generic geometry and numeric Home table do not match the requested product experience. The user specifies **Import tools for PicPay Partners** and rejects PGM.DATA as visible identity. This supersedes the Home/animation decisions in ADR-0001 for the local application, without inheriting its completed Nielsen evaluation goal.

## Decision

Use **PicPay Partners / PartnerGetMember** as display identity, a one-time interactive Splash and an operational Home of static ANSI charts. The splash shows a custom double-P payment-chip/circuit motif in the established green ramp, with **Enter to open workspace** and explicit quit/motion controls. It performs no API call and represents no connection or loading status. Interactive launch enables an ephemeral flag; Enter clears it and preserves the loaded job selection. Returning Home does not replay Splash.

Home contains record-outcome distribution, job lifecycle histogram, selected-job batch slice and recent activity with readable counts/legends. Graphics use actual loaded state only; no throughput trend or estimated batch completion is fabricated. Queued includes pending/retry-wait, active means in-flight, and UNKNOWN stays separately colored/labelled. Appearance preview is static; motion/speed choices govern Splash. Saved scene values remain readable while their displayed motifs change. Source/binary/data identifiers are not renamed by a display-brand change.

## Context and threat delta

Dependency direction stays TUI → App/engine; charts add no database/network call. Splash gating ignores paste/commands until explicit Enter and preserves Q/Escape/Ctrl+C restoration. No new credential, auth, storage, tenant, parser or provider boundary is created. Existing tabbed-settings threat models and guarded worker shutdown remain applicable. Tests exercise the new local input gate and retained safety contracts.

## Consequences and alternatives

The dashboard stays stable as decorative animation phases advance, and the splash gives the animation a distinct purpose without delaying access. Numeric legends remain necessary to interpret charts. Empty/filter-limited data is explicitly shown. A single giant animated Home logo, arbitrary product renaming and fake sparkline histories were considered and rejected because they contradict the requested product context or observable data.

## Rollback and delivery scope

Restore only task-owned source from the pre-task snapshot, remove the two new private view modules and rebuild offline within ten minutes. Never replace job/preference/Keychain data. Local tests, renders, PTYs, scoped performance, leak scanning and review apply. No publication, CI, staging migration, canary, on-call notification, external observability collector, production probe or provider contract validation is part of this local request. Coverage/mutation floors and independent reviews remain unverified unless measured; this scope does not turn them into passing gates.

References: `docs/splash-dashboard-plan.md`, `docs/domain/processing/glossary.md`, `docs/threat-models/0001-tabbed-terminal-settings.md`.
