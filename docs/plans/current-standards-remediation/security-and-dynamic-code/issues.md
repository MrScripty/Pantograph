# Security And Dynamic Code Issues

Current dispositions for the [active plan](plan.md); baseline details remain in
the [focused audit](../../../audits/2026-09-03-current-standards/01-security-and-dynamic-code.md).

| ID | Severity | Finding | Owner / disposition | Evidence | Revisit trigger |
| --- | --- | --- | --- | --- | --- |
| DYN-001 | Critical | Diffusers unconditionally enables model code without a Rust trust decision. | Inference; fix Milestone 0. | SDC-A1/A2 | New loader or policy owner |
| DYN-002 | High | Frontend and backend component validation fail open before generated-module import. | Tauri/frontend; fix Milestones 1–2. | SDC-A3/A5 | New write/import path |
| DYN-003 | High | Boa timeout leaves native validation work alive. | Tauri; delete in Milestone 1. | SDC-A4 | Future cancellable validator with unique value |
| DYN-004 | High, threat-dependent | Generated modules execute in the main renderer; command/network reach is unproved. | Desktop composition; remove execution Milestone 2, prove Milestone 3. | SDC-A5/A6 | Live preview or isolated executor proposed |
| DYN-005 | Medium | Persisted generated source can outlive prior unversioned validation state. | Tauri/frontend; retain source, discard proof, re-admit current bytes. | SDC-A3/A5 | State/history format changes |

## Deferred

- Live generated-component execution: future desktop architecture/security plan
  after a capability-free execution boundary is proven.
- General CSP, custom-command, and localhost posture: future desktop/network
  security plan if untrusted renderer content or listener exposure changes.
