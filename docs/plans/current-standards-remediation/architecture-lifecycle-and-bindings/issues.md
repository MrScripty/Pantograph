# Issues

Current authority: [plan.md](plan.md)

Source audit: [architecture, lifecycle, and bindings audit](../../../audits/2026-09-03-current-standards/02-architecture-lifecycle-and-bindings.md)

| ID | Severity | Finding | Disposition | Owner/status |
|---|---|---|---|---|
| ARC-01 | High | `node-engine` and binding paths retain a second direct inference authority. | Route supported runtime work through scheduler/runtime host; delete direct executor branches and feature edges. | M1-M3 / Planned |
| ARC-02 | High | Rustler creates and blocks private runtimes, and detached demand can report success before completion. | Remove unproved async/direct surfaces; retain only host-safe operations proven by BEAM. | M2 / Planned |
| ARC-03 | High | Binding boundaries contain permissive JSON/default conversion and silent/unbounded event behavior. | Checked typed rejection on retained surfaces; delete obsolete UniFFI event bridge. | M2, M4 / Planned |
| ARC-04 | High | Process spawning blocks async paths and detaches IO/monitor task ownership. | Deepen the process handle across standard and Tauri adapters with bounded observable lifecycle. | M5 / Planned |
| ARC-05 | High | Shutdown paths erase cleanup, readiness, child-task, or adapter failures. | Propagate operation-specific complete/incomplete/failed outcomes through every lifecycle owner. | M5-M6 / Planned |
| ARC-06 | Medium | Transition-era compatibility machinery increases authority and deletion cost. | Perform deletion review in each milestone; defer further decomposition unless complexity remains after removal. | M1-M7 / Planned |

## External dependencies

- Dynamic-code trust and real GPU/image validation are out of scope here and remain a blocking dependency of the security/image remediation before trusted image execution can be accepted.
- Repository-wide verification failures remain owned by the verification/tooling remediation; failures in files changed by this plan remain in scope.
- Discovery of a supported external Rustler or legacy UniFFI consumer changes deletion semantics and requires re-planning before implementation.

No additional issue is open at plan creation. Add an entry only for a concrete execution blocker, scope change, or unresolved finding; do not duplicate routine ledger activity.
