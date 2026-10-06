# Native default-output harness contract

The preserved native run 37545703782 completed the graph and retained the normal
`vectors.vector` workflow output. Production GUI submission uses
`output_targets:null`; the runtime exports host workflow I/O, not internal
`infer.embedding` or `infer.metadata` ports. The previous harness timed out
because it required those internal artifacts. That complete failed harness run
is preserved in the adjacent post-runtime-progress evidence before this change.

The corrected harness waits for completion of the same single submitted run,
reads its retained workflow-output vector through the canonical artifact body
API and checks all eight finite values against the unchanged synthetic oracle.
It separately requires that run's actual completed Candle CPU inference attempt
from the public scheduler timeline. The saved model reference is explicitly
identified as graph data; it is not represented as a runtime metadata body.
The raw vector body is written before later assertions to preserve evidence of
any subsequent failure. GUI save/reopen, Resolve, Submit, inspector navigation
and final saved graph checks remain in place.

Three Node contract tests pass. They use captured native artifact rows for the
output boundary and controlled timeline DTOs only for the helper's unit tests;
the real harness always queries the live timeline. Wrong run, node, port, role,
retention, transition, execution class, runtime, variant and device are rejected.
ESLint with declared Node/browser/Webdriver globals, syntax checks, critical
patterns and whitespace checks pass. Initial lint invocations omitted or
mis-specified runtime globals and are preserved alongside the successful check.
No Cargo build was repeated for this JavaScript-only correction.

Actual numerical native output remains pending a new GUI run. Internal metadata
body export, pretrained models, GPU execution and general production loader
qualification are outside this test's claim. Fixtures, weights, metadata,
production output defaults, Pumas pin and source history are unchanged.
