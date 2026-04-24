# Pantograph Requirements: Node System

## Status

Draft requirements note. This is not a full implementation plan.

## Purpose

Capture the agreed high-level requirements for the next-stage evolution of
Pantograph's node system.

This document exists so later proposals and implementation plans can work from
a stable set of expectations, terminology, constraints, and invariants.

## Scope

This note covers required product and architecture qualities for:

- node contracts
- port authoring
- node composition and factoring
- execution semantics
- observability and diagnostics
- compatibility and migration direction

This note does not define:

- milestone ordering
- exact crate-by-crate implementation steps
- exact API payload names for every transport surface
- exact frontend component behavior for every port editor
- exact persistence schema for all diagnostics artifacts

## Core Problem

Pantograph already has strong graph execution, typed compatibility checks, and
backend-owned workflow events. The current weakness is that node authoring,
effective port shape, and inference-node factoring are not yet strong enough
to support excellent composability without contract drift.

Pantograph needs a node system that simultaneously provides:

- first-class types
- backend-owned contracts
- strong observability
- actionable diagnostics
- disciplined architectural boundaries
- excellent node composability
- rich port authoring
- durable node factoring that avoids oversized "do everything" nodes

The system must improve authoring and composition without regressing on type
discipline, runtime traceability, or codebase standards compliance.

## Goals

- Make node and port contracts first-class backend-owned artifacts.
- Improve node composability without turning the graph into a stringly typed
  or weakly validated system.
- Make ordinary node authoring feel like defining a typed contract plus
  execution logic, not repeatedly wiring platform diagnostics by hand.
- Support finer-grained factoring for reusable workflow building blocks.
- Preserve or improve Pantograph's current execution observability and
  diagnostics quality.
- Align the node system with the coding and architecture standards already used
  by the repo.

## Non-Goals

- Rebuilding the node system around untyped runtime metadata.
- Moving business-critical contract resolution into the frontend.
- Treating generic JSON payloads as the preferred long-term answer for
  high-value workflow boundaries.
- Forcing all high-level workflows to be authored only from low-level
  primitives with no composition layer.

## Terminology

### Node Type Contract

The backend-owned contract for a node type. This includes stable node identity,
category, inputs, outputs, execution semantics, and authoring metadata needed
by consumers.

### Port Contract

The backend-owned contract for a specific input or output port. This includes
identifier, label, type, cardinality, requirement state, and any additional
metadata needed for validation, authoring, diagnostics, or display.

### Effective Node Contract

The resolved contract for a node instance after applying any permitted dynamic
or context-sensitive expansion rules. This is the contract the rest of the
system should treat as authoritative for that node instance.

### Primitive Node

A narrowly scoped node that owns one durable unit of behavior and exposes a
small, coherent contract.

### Composed Node

A higher-level node that presents a stable external contract while internally
mapping onto multiple primitive behaviors or graph elements.

### Authoring Metadata

Metadata attached to nodes or ports that exists to support graph construction,
editing, discoverability, and diagnostics, not just raw execution.

## Architectural Direction

Pantograph's node system must follow these architectural rules:

- backend-owned node and port contracts are the source of truth
- frontend code renders and edits backend-owned contracts rather than inventing
  the real contract shape
- contract types must have a single clear ownership boundary
- dynamic node shape is acceptable only when it is resolved and published by
  the backend as an authoritative effective contract
- execution, diagnostics, and authoring must consume compatible views of the
  same underlying contract model

## Required Contract Model

Pantograph must expose a first-class node contract model that is rich enough to
serve all of these needs from the same backend-owned source:

- graph validation
- connection compatibility
- execution dispatch
- palette discovery
- port editor rendering
- diagnostics labeling
- trace interpretation
- persisted workflow compatibility checks

Required properties:

- node contracts must use stable node type identifiers
- port contracts must use stable port identifiers rather than positional-only
  meaning
- node and port types must be represented as explicit domain types where the
  bug cost is meaningful
- public contract surfaces must prefer validated types and enums over raw
  strings where practical
- the contract model must be extensible without forcing breaking rewrites of
  existing consumers

## Single Source of Truth Requirements

Pantograph must reduce contract duplication and drift across crates and layers.

Required direction:

- there must be one canonical backend-owned definition family for node and port
  contracts
- adapter-facing DTOs may exist, but they must project from the canonical
  contract model rather than redefining it independently
- effective node contracts must be derived through explicit backend logic, not
  implicit frontend reconstruction
- any dynamic port expansion must be explainable and reproducible from backend
  facts

## Registry Discovery Requirements

Pantograph must not require graph-authoring clients to keep an application-local
catalog of node types, port shapes, or queryable option sources that can drift
from backend reality.

Required direction:

- external graph-authoring APIs and bindings must expose backend-owned node
  definition discovery from the canonical registry
- external graph-authoring APIs and bindings must expose backend-owned port
  option discovery for ports with dynamic option providers
- graph-authoring clients must be able to discover which ports are queryable
  without trial-and-error probing
- clients must not be required to hardcode node composition rules or maintain
  out-of-band copies of node authoring metadata
- adapter-facing discovery DTOs may reshape the backend contract for transport
  purposes, but they must remain projections of backend-owned registry facts
- adding, removing, or refining node types should not require binding-surface
  changes when the existing generic graph-authoring operations remain valid
- if a host surface lacks registry-backed discovery, that gap must be treated
  as incomplete graph-authoring support rather than acceptable steady state

## Binding-Facing Node System Requirements

The node system must be usable from the native Rust API and supported
host-language bindings without each host needing a separate node model.

Required direction:

- native Rust APIs own the canonical node-system semantics
- C#, Python, and Elixir bindings must consume projected views of the same
  backend-owned node contracts rather than maintaining host-local catalogs
- binding surfaces may expose host-idiomatic wrappers, but they must preserve
  stable node type ids, stable port ids, validation outcomes, and diagnostics
  correlation ids
- graph authoring through a binding must support the same registry-backed
  discovery flow required of GUI and native Rust authoring clients
- supported binding lanes must be able to explain connection rejections,
  effective contract expansion, and runtime failures using backend-produced
  diagnostics rather than host-side inference
- binding-specific gaps must be represented as support-tier limitations, not as
  alternate semantics for the same node system
- adding a node type must not require C#, Python, or Elixir source changes when
  the existing generic discovery and graph mutation contracts remain sufficient

## Port Authoring Requirements

Pantograph must improve port authoring so ports carry enough information to
support both strong contracts and good editing ergonomics.

Port contracts must be able to express, when relevant:

- stable id
- user-facing label
- data type
- required vs optional status
- single vs multiple connection capacity
- default value semantics
- value constraints such as ranges, enums, shape rules, or format rules
- editor hints for authoring surfaces
- visibility state such as normal, advanced, or hidden
- whether the port is data-bearing, configuration-bearing, control-bearing,
  diagnostic, or another defined port kind
- diagnostics labels suitable for logs, events, and trace views

The system must avoid treating all nontrivial settings as opaque generic JSON
when a typed port or typed handle would provide stronger guarantees.

## Effective Contract Requirements

Pantograph must support effective node contracts for cases where the externally
  visible port shape depends on validated backend context.

Required rules:

- effective contracts must be resolved by the backend
- effective contracts must preserve stable identity for ports where continuity
  matters
- effective contracts must be queryable by graph editors and diagnostics tools
- effective contracts must be suitable for connection validation without
  frontend guesswork
- effective contracts must be traceable back to the node instance and the
  backend facts that caused expansion

## Node Factoring Requirements

Pantograph must improve node factoring so workflow graphs can be composed from
reusable units rather than oversized nodes that bundle too many concerns.

Required direction:

- primitive nodes should own narrow, coherent responsibilities
- inference-oriented workflows should be factored around durable workflow
  concepts rather than adapter-local implementation convenience
- large nodes with too many unrelated knobs or mixed responsibilities should be
  treated as decomposition candidates
- the system should support both primitive nodes and composed nodes without
  making the primitive layer inaccessible

Signs that a node should be reviewed for decomposition include:

- too many heterogeneous inputs
- multiple distinct lifecycle concerns in one node
- mixed configuration, execution, and resource-management responsibilities
- outputs that combine unrelated domains
- difficulty reusing only part of the node's behavior in another workflow

## Composition Requirements

Pantograph must support first-class composition without weakening contracts.

Required capabilities:

- users must be able to build larger workflows from smaller reusable nodes
- the platform must support higher-level composed nodes or equivalent graph
  abstractions when that improves usability
- composed nodes must preserve a stable external contract
- composed nodes must remain diagnosable in terms of their internal primitive
  behavior
- composition must not hide critical runtime facts needed for debugging,
  validation, or attribution
- external graph-authoring clients must be able to compose graphs from
  backend-discovered node contracts rather than frontend-maintained node lists

## Node Authoring Ergonomics Requirements

Pantograph's node ecosystem is intended to support authoring new node types as
individual node files without forcing every node author to write repetitive
mandatory platform boilerplate.

Required direction:

- mandatory platform functionality must not require explicit diagnostics or
  observability code in ordinary node implementations
- creating a new node type should primarily require defining the node contract
  and the node's execution logic
- baseline platform behavior such as execution tracing, failure capture,
  attribution, and required observability must be injected by the runtime
  boundary rather than manually repeated in each node
- node authors should not have to manually attach stable node/run/session
  attribution to normal diagnostics emitted by the platform
- node authors should not have to manually measure standard model outputs when
  execution occurs through framework-managed model/runtime capabilities
- the platform may require node authors to use framework-owned capabilities for
  important side effects where mandatory system behavior depends on those calls
- ergonomics improvements must not weaken Pantograph's ability to enforce
  backend-owned diagnostics and attribution requirements

## Managed Capability Requirements

Pantograph must make the high-guarantee path the easy path for node authors.

Required direction:

- node execution should receive framework-owned capabilities for model
  invocation, file/resource access, progress reporting, cancellation, and other
  side effects whose use affects diagnostics or attribution guarantees
- framework-owned capabilities must automatically carry node, run, session, and
  lineage context where those facts are known
- direct model execution that bypasses managed capabilities must be detectable
  or explicitly unsupported for normal nodes
- capability APIs must be designed so node authors can remain focused on domain
  execution logic while the runtime preserves observability, cancellation, and
  output-measurement invariants
- capability contracts used by nodes must remain backend-owned and testable
  independently from GUI, C#, Python, or Elixir binding layers

## Implicit Baseline Observability Requirements

Pantograph must provide mandatory baseline observability for normal nodes
implicitly through the framework-owned runtime boundary.

Required baseline behavior for standard nodes includes, where relevant:

- backend-owned execution start, completion, and failure capture
- stable node and run attribution
- timing and lifecycle observability
- automatic input and output summaries derived from contract and runtime facts
- automatic model usage attribution for framework-managed model execution
- automatic output measurement when the framework has sufficient typed runtime
  facts to do so
- automatic lineage context sufficient for diagnostics and attribution surfaces

This baseline must exist without requiring node authors to emit diagnostics
events, measure outputs manually, or attach required attribution metadata in
ordinary node code.

## Custom Diagnostic Hook Requirements

Pantograph may allow node authors to supply optional custom hooks that enrich
diagnostics beyond the mandatory implicit baseline.

Required rules:

- custom hooks are optional
- custom hooks may add richer summaries, progress information, annotations, or
  other node-specific diagnostic detail
- custom hooks must augment the implicit baseline rather than replace it
- the absence of custom hooks must not weaken mandatory observability or
  attribution guarantees for normal nodes
- custom hooks must remain within backend-owned execution and diagnostics
  boundaries

## Escape Hatch Requirements

Pantograph may provide a lower-level escape hatch for advanced node authors, but
only under strict safety conditions.

Required rules:

- escape hatch use is acceptable only when Pantograph can detect that the
  escape hatch path was used
- detected escape hatch use must be explicitly marked as reduced-guarantee or
  unsafe in diagnostics and any relevant compliance-facing observability
  surfaces
- undetectable bypass paths are not acceptable as a supported node authoring
  model
- escape hatch use must not silently appear equivalent to framework-managed
  execution when guarantees have been reduced
- the system must be able to classify the effect of escape hatch use on
  observability, attribution, and output measurement guarantees

Required direction:

- framework-managed execution is the normal supported path
- custom hooks enrich the managed path
- escape hatches remain runtime-mediated rather than invisible arbitrary bypass
  paths
- any reduction in guarantees must be explicit and queryable

## Execution Semantics Requirements

Node contracts must be able to describe execution-relevant behavior, not only
static port shape.

The node system must support explicit modeling of:

- execution mode
- streaming behavior
- cacheability and invalidation behavior
- side-effect expectations
- resource or runtime affinity when relevant
- requirements for explicit triggers or control signals when relevant

Execution semantics must be backend-owned and available to diagnostics and
authoring consumers where appropriate.

## Type System Requirements

Pantograph must continue moving expensive bug classes into the type system.

Required direction:

- validated domain types should be preferred for high-value contracts
- generic JSON should be reserved for cases where the shape is genuinely open
  or low-risk
- persisted workflow artifacts must not depend on ambiguous or unstable
  type interpretation
- compatibility decisions should use explicit type semantics and well-defined
  coercion rules

When compatibility is not simple equality, the rules must still be explicit,
documented, and testable.

## Observability Requirements

Pantograph's node system must provide first-class observability for both graph
authoring and runtime execution.

Required capabilities:

- node execution must remain observable through backend-owned workflow events
- port-level and node-level contract information must be visible in diagnostics
  tooling
- contract expansion, invalidation, and compatibility decisions must be
  inspectable
- composed-node execution must remain traceable to primitive runtime behavior
- runtime events must identify nodes and ports using stable contract identities
  suitable for diagnostics correlation

## Diagnostics Requirements

Diagnostics must be treated as a first-class design concern of the node system,
not as an afterthought.

Required outcomes:

- connection rejections must be explainable in contract terms
- validation failures must identify the affected node, port, and rule
- effective contract changes must be diagnosable
- invalidation and rerun behavior must be explainable from backend facts
- runtime failures must preserve enough contract context to support debugging
- diagnostics surfaces must be able to present meaningful labels and grouped
  information without reconstructing semantics from ad hoc strings

## Model Usage Observability Requirements

The node system must support durable backend-owned observability for direct
model usage, model output measurement, and model/license attribution without
requiring explicit diagnostics nodes in ordinary workflows.

Required direction:

- nodes that directly invoke model execution must be identifiable as such
  through backend-owned contract or execution semantics
- direct model-output-producing execution must be instrumentable in a stable way
  even when the workflow author did not add any special observability node
- execution attribution must preserve the graph node identity that initiated the
  model execution
- composed nodes must not hide the primitive execution facts needed for model
  usage attribution
- the node system must support additive lineage metadata rather than requiring
  exact downstream ownership claims in the first pass
- the contract and execution model must leave room for typed output-measurement
  rules across text, image, audio, video, embeddings, and structured outputs
- model usage diagnostics must remain backend facts derived from runtime
  execution, not frontend inference over static graph structure
- custom hooks must not be required for compliance-grade model usage
  observability on the normal framework-managed path
- if escape hatch execution bypasses any framework-managed model observability
  path, Pantograph must explicitly mark the resulting attribution or measurement
  guarantees as reduced or unsafe rather than silently presenting them as full
  guarantees

This requirement exists so the node system can serve as a reliable foundation
for persistent model/license diagnostics, workflow-run attribution, and durable
output measurement over time.

## Standards Compliance Requirements

The node system redesign must align with the repository coding standards and
architecture standards.

Required direction:

- architectural roles and ownership boundaries must be explicit
- backend-owned data must remain backend-owned
- contract models must follow parse-at-boundary and correct-by-construction
  principles where the bug cost justifies them
- module and crate boundaries must reflect responsibility rather than
  convenience
- directories and modules introduced by this redesign must carry the required
  documentation and rationale

The redesign must not improve graph ergonomics by creating architectural drift,
hidden ownership, or duplicate contract definitions.

## Compatibility and Migration Requirements

Pantograph must preserve a viable migration path for existing workflows and
consumers.

Required direction:

- workflow persistence compatibility must be considered part of the design
- stable node and port identifiers must be preserved where feasible
- any intentional contract break must be explicit and documented
- migration or regeneration rules must exist for persisted artifacts affected
  by contract changes
- diagnostics and trace consumers must not silently lose meaning across
  contract upgrades

## Invariants

- The backend owns the authoritative node and port contracts.
- The effective contract for a node instance must be derivable from backend
  facts.
- Connection compatibility must remain explicit, validated, and testable.
- Node decomposition must not reduce diagnostics quality.
- Better authoring ergonomics must not come at the cost of weaker type
  discipline.
- Composed nodes must remain explainable in terms of runtime behavior.
- Mandatory baseline observability must not depend on per-node boilerplate.
- Custom hooks must never be required to preserve normal mandatory system
  guarantees.
- Escape hatch execution must be detectable and explicitly classified when it
  reduces guarantees.

## Revisit Triggers

- A proposed implementation requires the frontend to invent effective port
  shape that the backend cannot independently reproduce.
- A new node family repeatedly needs semantics that the contract model cannot
  express without falling back to generic JSON.
- Diagnostics consumers cannot explain contract resolution or invalidation
  behavior from backend artifacts.
- Persisted workflows become fragile because contract identity is underspecified
  or unstable.
- Primitive-node factoring becomes so granular that ordinary workflow authoring
  is no longer practical without composition support.

## Next Step

The next artifact should be an implementation-oriented design memo or plan that
translates these requirements into:

- proposed canonical contract types
- ownership boundaries across crates
- effective contract resolution flow
- factoring strategy for existing coarse nodes
- migration rules for existing workflows and diagnostics consumers
