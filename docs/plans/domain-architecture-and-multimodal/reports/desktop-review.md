# Desktop fixture and bounded repair review

Date: 2026-09-08. Read-only product investigation for M0/M1; no model, browser,
build, install, or GUI execution performed. Acceptance remains pending.

Standards: Core/Router at `366c1d90a24bbfb50973f62b155a5f3396c0f107`,
Frontend application profile, Contracts, Verification and GUI Verification,
and Accessibility prerequisite. Standards checkout has an untracked
`tools/standards_engine/prototypes/a2/proportionality-routing.prototype.html`;
it was not used as policy. This report covers the existing image GUI harness,
canonical inference port projection, saved graph/template path and Inspector
artifact read projection, not a complete frontend/accessibility audit.

## Executable fixture contract

Use one saved data graph loaded by the desktop Graph selector, with:

- `text-input.text -> text-inference.prompt`;
- a Pumas model node's `pumas_model_ref -> text-inference.pumas_model_ref`;
- `text-inference.text -> image-inference.prompt`;
- a second Pumas model node's `pumas_model_ref -> image-inference.pumas_model_ref`;
- optionally `image-inference.image -> image-output.image` for the canvas output.

Both inference nodes use `llm-inference`; task kinds are `text_generation`
(or the selected model's canonical `chat_completion`) and `image_generation`.
The output is **text**, not the retired static `response` port. This is backed
by `crates/pantograph-embedded-runtime/src/inference_interface_facts_provider.rs:231`
(input/output port builders). `crates/workflow-nodes/src/processing/inference.rs:80`
exposes only bootstrap controls; resolved descriptors and authored snapshots
must supply model-specific ports. Do not manufacture a fixed full descriptor
in the fixture or assume template knobs remain supported.

The current template `src/templates/workflows/tiny-sd-turbo-text-to-image.json`
is image-only, leaves model selection empty, and authors extra ports including
negative_prompt/steps/cfg_scale/seed/width/height that the current provider does
not publish. `src/services/workflow/templateService.ts:121` registers data graphs
and saves an orchestration; it is not the saved data-graph fixture consumed by
the smoke wrapper. It cannot be treated as a ready mixed-workflow acceptance
fixture. Create/configure the actual graph through current authoring and save
contracts after descriptors resolve; retain its model identities and authored
snapshots with the fixture evidence.

Existing entry point: `npm run test:workflow-editor-image-gui`, shell wrapper
`scripts/check-workflow-editor-image-generation-gui-smoke.sh`, configuration
`tests/e2e/workflow-editor-image-generation/wdio.conf.mjs`, and scenario
`tests/e2e/workflow-editor-image-generation/workflow-editor-image-generation.e2e.mjs`.
The scenario loads a saved workflow, submits, waits for Inspector, reads the
first image card and checks that an img is displayed. It does not select/verify
both inference nodes, text, terminal success, decoded image dimensions, actual
edge value, task/attempt identity, model selection, or cold reopen. Inspector
cards are scoped to the selected run-graph node; automatic selection preserves
the first existing node (`IoInspectorPage.svelte:290`). Select each required
node explicitly rather than relying on the first media card.

DA-03 procedure must capture one workflow_run_id, distinct task/attempt IDs,
actual generated nonempty text, image-node resolved prompt equal to that text,
and graph-edge provenance from the text node. Read the text and image outputs
through Inspector; assert browser image decode plus positive naturalWidth and
naturalHeight, not merely visibility. Use existing typed run-inspection
projections for attribution; never parse raw diagnostic payload JSON or submit
through a test-only executor. No token-streaming promise is added: terminal
retained text and complete image are the minimum acceptance output.

## Prerequisites and availability

The existing wrapper requires a saved `.pantograph/workflows/<id>.json`,
`PANTOGRAPH_WORKFLOW_EDITOR_IMAGE_SMOKE_WORKFLOW_ID`,
`PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID`,
`PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_ARTIFACT_ID`, and executable
`PANTOGRAPH_PYTHON_EXECUTABLE`. It rejects retired direct model path variables.
Model/artifact env variables currently act only as preflight presence checks:
the scenario never configures or asserts them. Fixture model identities are
therefore authoritative; future acceptance must verify agreement explicitly.
A text model/artifact selection must also be established; no text-smoke env
contract exists in this harness today.

Read-only observation: tauri-driver, WebKitWebDriver, node/npm, executable
node_modules/.bin/wdio and target/debug/pantograph exist. DISPLAY is set;
WAYLAND_DISPLAY and all four required smoke variables above are unset.
Only coding-agent.json and README.md were listed under saved workflows; no
configured mixed or image fixture exists there. Presence of a desktop binary
is not evidence it matches this revision. The config always builds via
`npm run build:desktop -- --debug --no-bundle --features backend-pytorch`.

Linux is the sole supported smoke target. A qualified isolated display/session,
working WebKit rendering, graphics/shared-memory/sandbox facts, available port
4444, bounded driver/app cleanup, and model/runtime dependencies remain
unverified. DISPLAY being set does not authorize silently borrowing an operator
session. Root reports nvidia-smi cannot communicate with the driver; this does
not establish that CPU or another execution device is unsupported. Runtime
review owns actual model families, package load-target facts, model-code trust,
Python environments and hardware admission. No real model pair is claimed ready.
The existing Mocha timeout is 240000 ms while waits can be 300000 ms, so the
actual total inference budget must be selected before running the mixed fixture.

## Persistence and output lifetime

`crates/pantograph-workflow-service/src/graph/persistence.rs:213` owns saved graph
canonicalization and workflow identity. Existing
`graph/persistence_tests.rs:321` proves model_id and selected_binding_ids survive
save while derived model paths/dependency facts are removed. Cold reopen must
resolve fresh model/dependency facts and preserve authored edge/port intent;
it must not persist stale readiness as authority.

Artifact bodies/descriptors have a durable store manifest, tested by
`crates/pantograph-workflow-service/tests/artifact_store.rs:147` (reopen and
missing-body reconciliation) and `:357` (descriptor/body reopening).
`workflow/artifact_contracts.rs:149` defines TTL, disk/memory/single-artifact
limits and delete_on_consume policy. Retention is policy-qualified, not forever.
For DA-05 record the actual policy, keep this fixture inside its retention
interval and limits, avoid consume acknowledgement that deletes bodies, cold
restart the same isolated project root, reselect the historical run, and read
both outputs again. ADR-014 explicitly does not persist active run selection
across restart. Historical run discoverability and payload reread need desktop
proof; artifact-store unit tests alone do not establish them. The current smoke
creates a new root each invocation and has no reopen phase.

## Two bounded reversible repair candidates

### D-01: GUI smoke loses ownership of its temporary project cleanup

Confirmed local lifecycle defect, medium severity. Wrapper lines 73–77 install
an EXIT trap, then line 86 uses exec to replace Bash with wdio. Successful exec
does not execute the Bash EXIT trap; normal wdio completion leaks the isolated
project and retained model outputs. This is independent of real model readiness.

Proposed exact write set: `scripts/check-workflow-editor-image-generation-gui-smoke.sh`
and a focused `scripts/check-workflow-editor-image-generation-gui-smoke.test.mjs`.
Keep a shell owner until the child completes, preserve success/failure status,
and remove only the allocated temporary root. If adding signal forwarding,
make child termination/reaping explicit and test it rather than implying that
removing exec alone proves cancellation. Do not change interactive startup or
runtime architecture.

Deciding evidence: Node test creates a temporary minimal repo-shaped harness
with fake required commands/workflow and a fake wdio; executes the real copied
wrapper; asserts the child sees an existing isolated workflow, wrapper exit
status matches success and a chosen nonzero child status, and that exact root
is gone afterwards. No actual desktop/model run is needed for this claim.
Run `node --test scripts/check-workflow-editor-image-generation-gui-smoke.test.mjs`
and `bash -n scripts/check-workflow-editor-image-generation-gui-smoke.sh`.

### D-02: Inspector supplies partial image files to the image decoder

Confirmed projection defect, medium severity. `ioInspectorPresenters.ts:67,798`
caps every preview request at 65536 bytes. `IoInspectorPage.svelte:336,460`
builds an image Blob from that response regardless of response.complete, and
`:848` renders it as the image source. The backend deliberately returns a byte
range and flags incompleteness (`workflow/artifact_store.rs:216–264`). A valid
image whose compressed body exceeds this limit is therefore supplied as a
truncated encoded file; successful complete-image decoding is not guaranteed.
The current GUI visibility assertion can falsely accept a broken img.

Proposed exact write set: `src/components/workbench/ioInspectorPresenters.ts`,
`src/components/workbench/ioInspectorPresenters.test.ts`,
`src/components/workbench/IoInspectorPage.svelte`, and the existing
`tests/e2e/workflow-editor-image-generation/workflow-editor-image-generation.e2e.mjs`.
Use the existing full-body read contract for retained image Read (Download
already calls readArtifactBody with only artifact_id), retain bounded text
previews, and never label/display incomplete image bytes as a successfully
loaded image. Preserve URL replacement/unmount cleanup and surface read/decode
errors. No new artifact storage or conversion layer is needed. If a UI image
size cap is desired it needs an explicit product policy, not the text byte cap.

Deciding tests: presenter request selection for text and image; integration in
real WebKit with a retained valid PNG larger than 64 KiB asserting complete
payload and successful decode; malformed-image/read-failure outcome. Existing
small image smoke alone cannot prove the regression. Run the focused Node
presenter tests and typecheck; real-WebKit evidence remains blocked until the
qualified harness/fixture is admitted. Do not claim DA-03 from a synthetic
large-image regression fixture; it only proves the image projection boundary.

These are proposals for integrator admission, not source-write authorization.
The harness's mixed-flow assertions and configured graph remain M3 work after
canonical runtime prerequisites are met. Investigation stops here; no broader
frontend or source repair was attempted.
