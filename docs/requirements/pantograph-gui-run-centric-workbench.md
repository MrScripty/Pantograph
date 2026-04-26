# Pantograph Requirements: Run-Centric GUI Workbench

## Status

Draft requirements note. This is not a full implementation plan.

## Purpose

Capture the agreed high-level requirements for Pantograph's next GUI direction:
a run-centric workbench that replaces the current split between the
drawing-to-Svelte tool and graph/panel UI with a coherent set of pages for
scheduling, diagnostics, graph inspection, I/O inspection, library management,
system/network visibility, and future node authoring.

This document exists so later frontend, API, scheduler, diagnostics, and
storage plans can work from a stable product model.

## Scope

This note covers required product behavior and architectural direction for:

- top-level GUI pages and navigation
- selected-run context across pages
- scheduler table, estimates, queues, buckets, and scheduler event visibility
- workflow identity and workflow versioning
- node and workflow semantic version/fingerprint rules
- run immutability and auditability
- I/O inspection and global retention policy behavior
- Library and Pumas asset/audit behavior
- local-first Network page requirements that can grow into Iroh-based peer
  discovery and distributed execution
- future Node Lab direction

This note does not define:

- exact component hierarchy
- exact database schema
- exact API payload names
- milestone ordering
- migration sequencing
- styling implementation
- detailed Iroh protocol design
- node-authoring agent implementation

## Core Problem

Pantograph currently has effectively separate GUI surfaces: the
drawing-to-Svelte UI tool that the app defaults to, and the graph view with
panels for other workflow functionality. Pantograph needs a single workbench
organized around how users and developers operate workflows over time.

The primary GUI experience should answer:

- what is scheduled, queued, running, delayed, completed, or failed
- why the scheduler made or is making a decision
- which workflow version, node versions, models, runtimes, inputs, policies,
  and devices were involved in a run
- what data entered, moved through, and exited a workflow
- which library assets were used by a run
- what local or future network execution capacity exists

Pantograph must preserve enough versioned and audited history to avoid mixing
diagnostics from different workflow/node/model/runtime conditions as if they
were the same.

## Goals

- Make the Scheduler page the default landing page.
- Treat a selected run as the primary GUI context shared by other pages.
- Preserve run history as auditable execution records, not transient logs.
- Version workflows by execution-relevant graph topology and node versions.
- Use semantic versions for human-facing workflow/node versions while using
  server-computed fingerprints for correctness.
- Reject semantic version/fingerprint conflicts strictly.
- Keep workflow identity stable, user named, and validated.
- Record execution context that does not define workflow version, including
  model choice, runtime choice, scheduler policy, graph settings, inputs, and
  target device.
- Make scheduler estimates available before execution through the GUI and API.
- Record model load/unload and scheduler decisions as visible, auditable
  scheduler events.
- Support global retention policy now while leaving room for more granular
  policies later.
- Make the Library both a browser/manager and a source of operational facts for
  scheduling and diagnostics.
- Make the Network page useful before P2P exists by showing the local instance
  as the first execution node.

## Non-Goals

- Making workflow-authored resource declarations authoritative.
- Letting workflows decide reservations, model load/unload, placement, or
  execution order.
- Persisting the active selected run across GUI restarts.
- Creating a separate stored graph for every run when the execution-relevant
  workflow version did not change.
- Treating UI/display metadata as part of diagnostics identity.
- Implementing Iroh networking or agentic node authoring in the first GUI pass.
- Making Pumas content read-only in the Library.

## Terminology

### Workflow Identity

A stable, user-named workflow identifier. The identity is the logical workflow
name across versions.

Workflow identities must be validated before acceptance. If a submitted
workflow identity does not satisfy Pantograph's naming rules, the submission
must be rejected with an explicit error that identifies the invalid field and
reason.

### Workflow Version

An execution-relevant version of a workflow. A workflow version is defined by:

- executable graph topology
- node identities
- node versions

Workflow versions do not include node placement, visual layout, model choice,
runtime choice, workflow inputs, scheduler policy, priority, target device, or
other editable execution context.

### Execution Fingerprint

A server-computed canonical fingerprint for execution-relevant workflow or node
content. Fingerprints are the authoritative way to detect whether semantic
versions are being reused for different executable behavior.

### Presentation Revision

A revision of workflow display metadata that is useful for restoring and
editing the graph visually but is not part of diagnostics grouping.

Presentation metadata may include node placement, layout, comments, display
labels, editor state, visual grouping, and other non-execution metadata.

### Run

An immutable submitted execution record for a workflow version. Runs include
future, scheduled, queued, delayed, running, completed, failed, cancelled, and
historic executions.

Once a run is submitted, changing it requires cancellation and submission of a
new run.

### Active Run

The run selected in the Scheduler or another run-aware surface during the
current GUI session. The active run drives the context displayed by other
top-level pages. The active run selection does not need to persist across app
restarts.

### Scheduler Estimate

A pre-execution record describing what the scheduler currently expects for a
queued or future run, including likely timing, placement, resource needs,
blocking conditions, candidate runtimes/devices, model load cost, and
confidence.

### Scheduler Event

A persistent observable event emitted by the scheduler, including queue
decisions, estimates, delays, reservations, model loads/unloads, placement
decisions, retries, cancellations, and admin/client actions that affect
scheduling.

### Library Asset

An asset managed or known by Pantograph or Pumas, including models, runtimes,
nodes, workflows, templates, connectors, and future local or remote additions.

### Network Node

An execution-capable Pantograph instance. Before P2P support exists, the local
Pantograph instance is the only network node. Future Iroh support will allow
trusted peer instances to be discovered and used for distributed execution.

## Top-Level GUI Pages

Pantograph's GUI should use distinct pages rather than treating the current
tools as modes inside one view.

Required top-level pages:

- Scheduler
- Diagnostics
- Graph
- I/O Inspector
- Library
- Network
- Node Lab

The drawing-to-Svelte tool should no longer be the default landing surface. It
may later become a page, Library tool, Graph tool, prototype importer, or UI
builder surface, but it is not the organizing center of the application.

## Default Landing Page

Pantograph GUI startup must open to the Scheduler page.

The Scheduler page must present a dense spreadsheet-like run list showing:

- future runs
- scheduled runs
- queued runs
- delayed runs
- running runs
- completed runs
- failed runs
- cancelled runs
- historic runs

The Scheduler page is the primary place to see what is happening and inspect
key metrics such as status, scheduled time, queue state, duration, progress,
runtime/device, workflow version, and error summary.

## Active Run Navigation Model

Selecting a run in the Scheduler sets the active run for the current GUI
session.

When an active run exists:

- Diagnostics shows diagnostics for that run.
- Graph shows the workflow version for that run.
- I/O Inspector shows the workflow inputs, workflow outputs, and node-to-node
  data for that run subject to retention state.
- Library highlights assets used by that run.
- Network highlights the local or future network nodes, runtimes, models, and
  scheduler decisions relevant to that run.

Each page must also have a useful no-active-run state:

- Scheduler shows all visible runs.
- Diagnostics shows system or aggregate diagnostics.
- Graph allows workflow browsing/editing.
- I/O Inspector allows general retained artifact browsing where supported.
- Library shows the full asset library.
- Network shows local system/node state.
- Node Lab shows future authoring entry points or an unavailable/experimental
  state until the feature exists.

## Scheduler Page Requirements

The Scheduler page must be optimized for scanning many runs.

Required table behavior:

- dense rows similar to a spreadsheet
- sortable columns
- filterable status, workflow, workflow version, date, runtime, model, device,
  session, bucket, and retention state where data exists
- searchable run identifiers, workflow names, and error summaries
- selected-row active run context
- current/future/historic runs in the same operational surface
- visible queued and future scheduled runs

Recommended run columns:

- status
- run id
- workflow identity
- workflow semantic version
- workflow execution fingerprint/version id or abbreviated equivalent
- scheduled time
- queue position
- priority
- session/bucket
- estimated start
- estimated duration
- actual start
- actual duration
- progress
- target/candidate runtime
- target/candidate network node
- models used or expected
- nodes completed/failed
- output/retention summary
- blocking or delay reason
- error summary

Selecting a run should expose actions appropriate to the caller's authority,
such as:

- cancel
- clone/resubmit
- push to front of the current client session queue where allowed
- inspect diagnostics
- open graph
- inspect I/O
- view library assets used by the run

Admin/developer actions available only to the Pantograph GUI may include:

- cancel any run
- reorder across sessions
- pause/resume queues or buckets
- override priority
- force estimate recomputation
- force reschedule where supported
- inspect all scheduler events
- update global retention policy
- manage library assets

## Scheduler Estimates

Scheduler estimates must be produced before a workflow runs.

Estimates must be visible in the Scheduler page and queryable through the API.

Scheduler estimates should include, when known:

- estimated start time
- estimated duration
- estimated memory and VRAM use
- estimated disk/cache needs
- likely runtime/device/network node
- candidate runtimes/devices/network nodes
- queue position
- blocking conditions
- delay reason
- missing assets
- expected model load cost
- expected cache benefit
- confidence or quality level
- estimate timestamp and version

The estimate record must be distinguishable from observed execution facts so
Pantograph can compare scheduler expectations against actual results.

## Scheduler Authority and Resource Inference

Workflows must not explicitly declare authoritative resource requirements.

The scheduler derives resource needs from facts and observations, including:

- node definitions
- node versions
- graph settings such as context length, image resolution, batch size, or other
  execution-relevant settings
- model metadata
- model file sizes
- runtime metadata
- library asset metadata
- local and future network node capabilities
- current load
- queue state
- past diagnostics
- previous measured resource usage

Workflows may contain settings that affect execution behavior, but only the
scheduler decides:

- execution order
- reservations
- placement
- runtime selection
- model loading
- model unloading
- retries
- fallback behavior
- distributed assignment
- delay for better cache/model state

The scheduler may intentionally delay a run when it expects a better cache,
model, runtime, or fairness outcome.

## Scheduler Events and Auditability

Scheduler decisions must be observable and auditable.

The scheduler must record visible events for:

- run submitted
- estimate created or updated
- run queued
- run delayed
- run promoted
- run cancelled
- resource reservation created, changed, or released
- runtime selected
- device/network node selected
- model load requested
- model load started
- model load completed
- model load failed
- model retained in cache
- model unload scheduled
- model unload cancelled
- model unload started
- model unload completed
- run execution started
- node execution started
- node execution completed
- run completed
- run failed
- retry scheduled
- fallback selected
- client queue action applied
- admin override applied

Scheduler events should record:

- timestamp
- event type
- run id when applicable
- scheduler decision id when applicable
- session/bucket when applicable
- runtime/device/network node when applicable
- model/runtime/node references when applicable
- reason
- facts considered where practical
- estimates before/after where practical
- observed result when applicable

Some scheduler events are system-level and not tied to one run, such as model
unload due to memory pressure, runtime restart, cache pruning, device/node
availability changes, queue policy changes, or future network node disconnects.

## Client, Session, Bucket, and Admin Control

Normal clients may influence only their own submitted work.

Allowed client/session-scoped actions should include:

- submit run
- cancel own run
- query estimates for own runs
- change priority within allowed session/bucket policy
- push a run to the front of the client's own session queue
- clone/resubmit own run

Normal clients must not be able to:

- reorder global queues
- starve other sessions
- change global scheduler policy
- reserve shared resources directly
- force placement onto a device/runtime used by other sessions
- cancel runs owned by other sessions

The Pantograph GUI is a privileged developer/admin surface. It may expose
actions that affect all sessions, including global queue management, retention
policy changes, library management, and scheduler overrides.

The scheduler remains the internal authority even when clients or admins
influence inputs to scheduling.

## Workflow Identity Requirements

Workflow identity must be user named and stable.

Pantograph must validate workflow identities before accepting submitted
workflows or runs. Validation requirements should be strict enough that
workflow identities are safe for Pantograph's storage, API, diagnostics, and
display systems.

The exact naming grammar may be defined later, but it should likely support a
restricted character set such as lowercase letters, numbers, hyphen,
underscore, and dot, with a maximum length and reserved-name rejection.

Invalid workflow identity submissions must be rejected with an explicit error,
for example:

```json
{
  "error": "invalid_workflow_identity",
  "message": "Workflow identity must start with a letter and contain only supported identifier characters.",
  "field": "workflow.identity"
}
```

## Workflow Versioning Requirements

Workflow versions must be created automatically when submitted executable graph
topology and node versions do not match an existing version for the same
workflow identity.

Workflow version identity is based on:

- normalized executable topology
- node identities
- node versions

Workflow version identity excludes:

- model choices
- runtime choices
- scheduler policy
- inputs
- priority
- scheduled time
- target device
- node placement
- visual layout
- display metadata

Queued runs must reference workflow versions immediately. Once a run is queued,
it is immutable. If a queued run needs changes, the old run should be cancelled
and a new run submitted.

Pantograph may trust clients to submit a changed workflow under the intended
stable workflow identity. If a client submits a changed workflow as an entirely
new workflow identity, diagnostics will treat it as a separate workflow. Later
tooling may detect possible duplicates, but this is not required initially.

## Semantic Version and Fingerprint Requirements

Workflows and nodes should have semantic versions for human-facing version
management.

Pantograph must also compute execution fingerprints to remove ambiguity.

Required behavior:

- semantic versions are visible and filterable
- fingerprints are authoritative for detecting executable content changes
- the same semantic version must not point to different execution fingerprints
- if a semantic version/fingerprint conflict is detected, the submission must
  be rejected strictly

Example conflict response:

```json
{
  "error": "version_fingerprint_conflict",
  "message": "Workflow 'image-pipeline' version '1.2.0' already exists with a different execution fingerprint.",
  "workflow": "image-pipeline",
  "version": "1.2.0"
}
```

Pantograph may assist with semantic version suggestions, but it cannot always
know user intent well enough to determine correct major/minor/patch semantics.
Fingerprints are required to cover that uncertainty.

## Presentation Revision Requirements

Display metadata should be versioned or revised separately from execution
versions.

Presentation revisions may include:

- node placement
- visual layout
- graph annotations
- comments
- display labels
- editor state
- visual grouping that does not affect execution

Diagnostics must group by execution version, not presentation revision.

Graph views for historic runs should show the workflow as it existed at run
time. When presentation metadata is available for that version, Graph should
use it to restore the user-facing layout. If presentation metadata is missing,
Graph may render the executable topology using a generated layout.

## Run Immutability and Audit Snapshot

Every future, queued, running, or historic run must reference:

- workflow identity
- workflow semantic version
- workflow execution version/fingerprint
- presentation revision when available
- node identities
- node semantic versions
- node behavior fingerprints where available
- model choices and versions
- runtime versions
- scheduler policy/version
- retention policy/version
- graph settings that affected execution
- immutable input summary or payload references
- session and bucket attribution

Execution context such as model choice, inputs, runtime choice, scheduler
policy, and graph settings must be recorded and filterable for diagnostics even
when those fields do not define workflow version identity.

## Diagnostics Requirements

Diagnostics must support version-aware filtering so metrics are not mixed
across materially different execution conditions.

Required filters should include:

- workflow identity
- workflow semantic version
- workflow execution fingerprint/version
- node identity
- node version
- model identity/version
- runtime identity/version
- network node/device
- scheduler policy
- graph settings where relevant
- input profile or input characteristics where available
- session/bucket/client
- date range
- status
- retention completeness

Diagnostics views should make mixed-version result sets visible, for example
by indicating when multiple workflow versions, node versions, model versions,
or runtime versions are present in the current result set.

Run diagnostics should include a scheduler decision section showing:

- facts considered
- estimates made
- selected runtime/device/network node
- rejected alternatives where available
- delay reasons
- model load/unload behavior
- client or admin queue actions
- actual observed result

Comparison is a likely future concern. The data model and GUI should not block
future run-vs-run, workflow-version-vs-workflow-version, runtime-version, model
version, device, or input-profile comparisons.

## Graph Page Requirements

The Graph page must display the active run's workflow version when a run is
selected.

Historic runs must show the workflow as it existed at the time of the run.
Pantograph should not store a unique graph for every run unless the executable
workflow version actually changed.

The Graph page should distinguish:

- run view: read-only or replay-oriented view of the workflow version used by
  the active run
- edit view: current editable workflow definition

The first implementation may keep this distinction simple, but the product
model must avoid silently showing the current editable workflow as if it were
the historic executed workflow.

## I/O Inspector Requirements

The I/O Inspector must show the data supplied to and produced by the active
run, including:

- workflow inputs
- workflow outputs
- data passed between nodes
- node inputs
- node outputs
- intermediate artifacts
- final artifacts
- raw payload views where needed

The I/O Inspector should support a gallery-like browsing model for object
types such as:

- text
- images
- audio
- video
- tables
- JSON/structured data
- files/artifacts
- unknown/raw data

The Inspector should support both:

- node-centric debugging, where selecting a node shows inputs and outputs
- artifact-centric browsing, where generated objects can be reviewed as a
  gallery or library of outputs

## Retention Policy Requirements

Retention policy is global for the first implementation.

The system should be architected so more granular policy can be added later,
but no per-workflow, per-run, or per-artifact policy is required initially
except for explicit pinning if supported.

Retention policy changes are retroactive. Changing the global policy may affect
old runs by deleting or expiring retained payloads.

Payload data may expire or be removed by policy, but audit metadata should
remain. Pantograph should still be able to explain what existed, when it was
produced, and why the payload is no longer available.

I/O artifacts should expose retention state such as:

- retained
- expiring
- expired
- deleted by policy
- metadata only
- external reference only
- truncated
- too large to retain
- pinned/exempt where supported

The I/O Inspector should expose global retention settings relevant to
inspected data, such as:

- keep final outputs duration
- keep workflow inputs duration
- keep intermediate node I/O duration
- keep failed-run data duration
- maximum artifact size
- maximum total storage
- media retention behavior
- compression/archive behavior
- cleanup trigger/status

Retention actions and deletions must be auditable with policy version,
timestamp, and reason.

## Library Requirements

The Library page must combine Pumas library content and Pantograph-owned
additions.

Pumas content does not need to be read-only. Pantograph may use Pumas APIs for
operations such as:

- Hugging Face model search
- model download
- model deletion
- model metadata inspection
- cache/library management

The Library should expose asset categories such as:

- models
- runtimes
- workflows
- nodes
- templates
- connectors
- local additions
- Pumas assets
- Pantograph-owned assets

For an active run, Library should highlight assets used by that run, including:

- workflow version
- node versions
- models
- runtimes
- templates/connectors where applicable
- local or Pumas source
- asset versions and fingerprints where available

The Library must also serve as a source of operational facts used by the
scheduler, including concrete facts like model file sizes, formats, local cache
state, compatible runtimes, and observed historical performance.

## Pumas Audit Requirements

Pantograph should audit Pumas and Library activity, including:

- model search
- model download
- model deletion
- model access by a run
- runtime access
- node/library asset access
- asset version used in a workflow version
- network traffic volume where available
- cache hits and misses
- failed access or download
- accessing run, session, bucket, client, or GUI actor where available

Library usage telemetry should support views such as:

- used by active run
- used by N runs
- last accessed
- total access count
- downloaded size
- network bytes
- linked workflow versions
- linked node versions
- failures by model/runtime/node version

## Network Page Requirements

The Network page should exist even before P2P networking is implemented.

The first version should show the local Pantograph instance as the only
execution node and expose local system information useful to the scheduler and
developer/admin users.

Local-only Network page data should include:

- local instance identity
- CPU, memory, GPU, and disk information where available
- available runtimes
- available models
- current load
- active runs
- queued work
- model/cache state
- capability summary
- scheduler/system health events

Future Iroh-based networking should extend the same model to multiple nodes:

- local node
- discovered peer nodes
- trusted/pairing state
- available runtimes per node
- available models per node
- capabilities per node
- load per node
- active assignments
- transfer state
- health/latency
- historical reliability

Distributed devices should require explicit first-time pairing, likely through
a code or key that lets devices find each other and verify node identity before
trusting each other for distributed execution.

## Node Lab Requirements

Node Lab is a future top-level page for authoring new or existing nodes.

The expected direction is a local-agent-assisted UI specialized for generating,
editing, testing, and publishing node definitions for the runtime ecosystem.

Node Lab depends on other Pantograph features that are not ready yet, so the
first GUI reorganization should reserve a page/route and product slot without
requiring a full implementation.

## API Requirements

The GUI requirements imply API support for:

- listing future, queued, running, completed, failed, cancelled, and historic
  runs
- selecting/querying a run by id
- querying scheduler estimates before execution
- querying scheduler events by run and globally
- querying workflow version and presentation revision used by a run
- querying I/O artifacts and retention state for a run
- updating global retention policy from privileged/admin surfaces
- querying Library assets used by a run
- querying Library/Pumas audit events
- querying local Network/system node state
- submitting immutable runs
- cancelling own runs from client scope
- cancelling any run from privileged GUI scope
- moving a run to the front of the owning client session queue where allowed
- performing privileged admin queue actions from the GUI

Exact endpoint names and payload shapes are implementation-plan concerns.

## Invariants

- The Scheduler page is the default GUI landing page.
- Runs are the primary context object that connects Scheduler, Diagnostics,
  Graph, I/O Inspector, Library, and Network.
- Active run selection does not need to survive app restart.
- A queued run is immutable.
- Changing a queued run means cancelling it and submitting a new run.
- Workflow identity is stable and user named.
- Invalid workflow identities are rejected explicitly.
- Workflow execution versions are defined by executable topology and node
  versions.
- Model choice, runtime choice, inputs, scheduler policy, graph settings, and
  target device are run context, not workflow-version identity.
- Semantic versions are human-facing labels, not sufficient correctness
  identifiers by themselves.
- Fingerprints are required for strict correctness.
- Semantic version/fingerprint conflicts are rejected.
- Presentation metadata may be versioned separately but must not contaminate
  diagnostics grouping.
- Scheduler estimates exist before execution.
- Only the scheduler decides reservations, model load/unload, runtime
  placement, execution order, retry, and delay behavior.
- Model load/unload decisions are visible scheduler events.
- Retention policy is global initially and retroactive.
- Payload retention may expire, but audit metadata remains.
- Pumas/library activity is auditable.
- The Network page begins local-only and grows into distributed execution
  visibility.

## Revisit Triggers

- Iroh peer discovery and trust design begins.
- Node Lab implementation begins.
- Per-workflow, per-run, or per-artifact retention policy becomes necessary.
- Multiple user/admin roles beyond the Pantograph GUI special case are
  introduced.
- Scheduler learns enough from diagnostics that estimate confidence and
  recommendation semantics need a formal model.
- Workflow duplicate-detection across separately named workflow identities
  becomes important.
- Presentation revision history needs storage, pruning, or merge semantics.

## Dependencies

**Internal:**

- `docs/requirements/pantograph-client-sessions-buckets-model-license-diagnostics.md`
- `docs/requirements/pantograph-node-system.md`
- `docs/plans/pantograph-execution-platform/README.md`
- `docs/plans/diagnostics-run-history-projection/plan.md`
- `docs/plans/scheduler-only-workflow-execution/plan.md`

**External:**

- Pumas APIs for model/library operations and metadata.
- Future Iroh integration for peer discovery and trusted distributed execution.

## Related ADRs

- `docs/adr/ADR-011-scheduler-only-workflow-execution.md`
- `docs/adr/ADR-012-canonical-workflow-run-identity.md`
- `docs/adr/ADR-008-durable-model-license-diagnostics-ledger.md`
- `docs/adr/ADR-007-managed-runtime-observability-ownership.md`

## Structured Producer Contract

- This is a human-authored requirements note.
- Implementation plans may reference this document as a source of product and
  architecture requirements.
- Headings and prose are not machine-readable API contracts.
- Any implementation plan consuming this document should preserve the listed
  invariants or explicitly record why an invariant changed.
