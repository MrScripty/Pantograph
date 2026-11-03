# Public ready inference projection payload

Fresh PR #38 Clippy retains the public Ready projection at 528 bytes versus the Blocked record at 80. Box only Ready and migrate four existing constructors: current validation state, executable snapshot projection and two workflow fixture helpers. Raw fields, equality, borrowed readers, freshness/admission checks and blocked behavior remain unchanged. The enum has no Serde implementation; the downstream scheduler intent wire contract is the serialization boundary to preserve.

Tests cover owned raw-record equality and layout, plus exact downstream intent JSON and deserialization while retaining existing path-free projection assertions. CI explicitly discovers and runs task-graph, task-binding and executable-snapshot modules; inherited validation-owner and Headless tests retain ready/stale/missing-estimate/blocked behavior and the embedded-runtime consumer.

This is a public Rust construction/owned-binding change despite publish=false, documented with the extra ready-record allocation and no measured performance claim. Root approved the bounded design. Root source review accepted all eight files at frozen tree 42349ac0a39dd876e5fca06ca4b285f3cc6eaca8. Hosted execution remains pending; no local Rust execution is claimed.
