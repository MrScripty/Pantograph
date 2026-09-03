# Current-Standards Remediation Portfolio Issues

Cross-plan issues for [the active portfolio](plan.md). Domain findings and implementation detail remain in child issue logs.

## PORT-I01

- **Status:** `open`; blocks dependency/release/documentation Milestone 2 and release acceptance.
- **Issue:** Project license authority conflicts between Apache-only and MIT-or-Apache descriptions.
- **Owner/action:** Repository owner selects terms; the dependency/release/documentation plan records metadata, obligations, and artifact evidence.
- **Revisit:** Before license metadata or release candidate work.

## PORT-I02

- **Status:** `open discovery`; not a blocker for the current security slice.
- **Issue:** Supported external Rustler, UniFFI, or `@pantograph/svelte-graph` consumers and compatibility promises are not fully established.
- **Owner/action:** Architecture and frontend Milestone 0/precondition inventories classify consumers. A discovered supported consumer triggers atomic migration re-planning before incompatible deletion.
- **Revisit:** Before the affected binding/export change.

## PORT-I03

- **Status:** `open capability`; blocks only claims selecting an unavailable environment.
- **Issue:** Representative desktop/browser/host and required-real GPU/model/target evidence was not run by the audit.
- **Owner/action:** Verification and release owners provision the declared lanes and record exact unavailable/unsupported outcomes. Lower-fidelity evidence cannot satisfy the claim.
- **Revisit:** At each child gate selecting those environments and before portfolio closeout.

## PORT-I04

- **Status:** `controlled coordination risk`.
- **Issue:** Child plans overlap runtime-host, Tauri, lockfile, package, workflow, launcher, CI, and documentation paths; unrelated user work is also present in shared documentation.
- **Owner/action:** The portfolio admits one slice at a time, records explicit handoffs, and preserves user changes. An unresolved overlap blocks only the affected slice.
- **Revisit:** Before every implementation admission.
