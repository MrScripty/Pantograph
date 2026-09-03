# Issues

Current authority: [plan.md](plan.md)

Source audit: [frontend and accessibility audit](../../../audits/2026-09-03-current-standards/03-frontend-and-accessibility.md)

| ID | Severity | Finding | Planned disposition | Owner/status |
| --- | --- | --- | --- | --- |
| FE-01 | High | Tauri IPC values enter frontend state through generics, assertions, partial decoding, and guessed defaults. | One raw transport Adapter plus action-owned complete decoders and representative producer/consumer evidence. | M0-M1 / `Planned` |
| FE-02 | High | Async mount, listener, refresh, and navigation work can leak or apply stale completion. | Synchronous teardown registration, scoped invocation identity, observed terminal outcomes, and lifecycle tests across the bounded population. | M0, M3 / `Planned` |
| FE-03 | Medium | Connection policy is copied locally and fails open when authority is missing. | Backend-authored intent only in production; configured intent in tests; unavailable/invalid blocks commit. | M2 / `Planned` |
| FE-04 | Medium | Package and app graph modules, stores, helpers, and imports compete. | Package owns reusable editor; app is a thin product composition Adapter; delete duplicates/deep imports/retired projection. | M0, M2 / `Planned` |
| FE-05 | Medium | Browser-persisted values are partially decoded and can become state authority. | Record-owned schemas, bounds, migration/version behavior, injected storage, and typed outcomes. | M0, M4 / `Planned` |
| FE-06 | Medium | Accessibility tooling and pure tests do not prove real roles, focus, keyboard, pointer, or cleanup. | Repair confirmed user-task defects and require representative WebKit/Tauri interaction evidence; tooling owns checker changes. | M0, M5-M6 / `Planned` |

## External dependencies

- Verification/tooling must register and run `VP-FRONTEND-ACCESS-001`; absence blocks FE-A06, not earlier product work.
- Release/documentation must classify package consumers and compatibility before an incompatible export change.
- Architecture/security plans remain authoritative for backend execution and generated-code trust respectively.

No additional issue is open at plan creation. Add only concrete blockers, scope changes, or unresolved findings; routine progress belongs in the ledger.
