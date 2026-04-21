# Pass 03: Runtime, Security, and Concurrency Findings

Audit date: 2026-04-21

## Scope
This pass reviewed boundary validation, listener safety, path handling, process
lifecycle, task spawning, runtime shutdown, and async/concurrent behavior.

## Standards Applied
- `SECURITY-STANDARDS.md`: validate once at boundaries, centralized path validation, network transport safety.
- `CONCURRENCY-STANDARDS.md`: bounded queues, task lifecycle, non-blocking async paths, mutex selection, stale async response guards.
- `CROSS-PLATFORM-STANDARDS.md`: platform checks in thin modules, path APIs, spaces in paths.
- `INTEROP-STANDARDS.md`: symmetric init/shutdown and boundary validation.

## Findings

### P03-F01: Development Server Binds to All Interfaces
Severity: High

Evidence:
- `vite.config.ts` sets `server.host` to `0.0.0.0`.

Standards conflict:
- Local-only dev/IPC services must bind to loopback unless intentionally exposed.

Required direction:
- Default Vite to `127.0.0.1`.
- Add an explicit environment variable or launcher flag for LAN exposure with
  visible logging and documentation.

### P03-F02: Production Startup Uses `expect(...)` in Runtime Paths
Severity: High

Evidence:
- `src-tauri/src/main.rs` calls `expect(...)` for project-root resolution, stale
  cleanup worker startup, app data dir, runtime config application, and final
  Tauri app run.

Standards conflict:
- `CODING-STANDARDS.md` forbids `unwrap`/`expect` in production request/runtime
  paths. Startup and shutdown are runtime lifecycle paths.

Required direction:
- Convert startup to a fallible setup function that logs context and returns
  typed or envelope-style errors where Tauri allows it.
- Keep only invariant-only expects with immediately adjacent documentation.

### P03-F03: Spawned Runtime Tasks Are Not Uniformly Tracked
Severity: High

Evidence:
- `src-tauri/src/main.rs` starts extension initialization with
  `tauri::async_runtime::spawn` without retaining a handle.
- `src-tauri/src/llm/process_tauri.rs` spawns stdout, stderr, and monitor tasks
  without a visible shutdown/abort owner.
- `src-tauri/src/llm/health_monitor.rs` starts a loop with `tokio::spawn`; the
  monitor has a running flag, but task ownership should be audited end to end.

Standards conflict:
- Every spawned task should have a lifecycle owner and a join/abort/shutdown path.

Required direction:
- Introduce a runtime task supervisor or register `JoinHandle`s in owned service
  resources.
- Document lifecycle ownership in relevant READMEs.

### P03-F04: Process PID Records Are Too Weak for Instance Coordination
Severity: Medium

Evidence:
- `src-tauri/src/llm/process_tauri.rs` writes only the PID to the PID file.
- `ARCHITECTURE-PATTERNS.md` recommends PID plus process start time to avoid PID reuse.

Required direction:
- Replace bare PID files with structured PID records containing pid, start time,
  version/mode, and owning runtime identity where applicable.
- Use platform abstractions for start-time verification.

### P03-F05: Legacy Path and Workflow Persistence Boundaries Need Consolidation
Severity: Medium

Evidence:
- `crates/node-engine/src/path_validation.rs` is a good centralized path validator.
- `src-tauri/src/workflow/workflow_persistence_commands.rs` uses it for loading,
  but dead-code warnings suggest this module may be legacy while
  `pantograph-workflow-service::FileSystemWorkflowGraphStore` is the active owner.

Risk:
- Two path-boundary implementations can drift or leave tests attached to a
  non-runtime path.

Required direction:
- Identify the single active workflow persistence boundary.
- Move all external path validation tests to that boundary and delete or archive
  superseded code.

### P03-F06: Listener Limits and Shutdown Are Not Fully Evident
Severity: Medium

Evidence:
- Test HTTP listeners bind to loopback, which is good.
- The audit did not find a documented maximum connection count and graceful
  shutdown contract for every local listener or HTTP adapter path.

Required direction:
- For product listeners, document bind address, max concurrent connections,
  read timeout/heartbeat behavior, and graceful shutdown in module READMEs.
- Add tests for shutdown/release of bound ports where practical.

### P03-F07: Critical Frontend DOM-Mutation Gate Fails
Severity: High

Evidence:
- `npm run lint:critical` fails:
  `src/components/nodes/workflow/ImageOutputNode.svelte:62:18 [no-append-remove-child]`

Standards conflict:
- Frontend standards prefer declarative rendering and the repo's own critical
  anti-pattern gate blocks `appendChild`.

Required direction:
- Replace the download anchor append/remove path with a Svelte-managed anchor,
  Blob URL plus direct click through a bound element, or another compliant
  browser API wrapper accepted by the critical gate.

## Additional Issues Outside Pure Standards Compliance
- `cargo check` succeeds but emits broad dead-code and unused warnings. Treat
  warnings as cleanup work because the tooling standards prefer fail-on-warning
  gates once debt reaches zero.
- Some process/runtime code has valid cross-platform cleanup improvements from
  earlier work, but PID identity and task supervision remain separate risks.

## Pass 03 Remediation Themes
1. Make local services loopback-only by default.
2. Replace production lifecycle panics with typed startup errors.
3. Add task supervision for background tasks and process stream readers.
4. Consolidate path-boundary ownership around the active workflow persistence implementation.
5. Fix the remaining critical frontend anti-pattern before broader lint cleanup.
