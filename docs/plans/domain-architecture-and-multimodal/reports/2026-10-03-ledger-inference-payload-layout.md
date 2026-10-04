# Diagnostic event inference payload indirection

Fresh PR #33 aggregate Clippy identifies InferenceExecutionDiagnosticObserved as the 872-byte largest DiagnosticEventPayload variant, with IoArtifactObserved next at 640 bytes. Box only the inference variant; preserve the raw payload type, tagged Serde representation, validation, event-kind mapping and projection readers. Migrate the five existing constructors: three embedded-runtime recorders, one ledger fixture and one workflow-service fixture. Other variants and Result types remain unchanged.

An explicit minimal tagged JSON test deserializes the public enum, checks exact serialization and appends/reads it through the SQLite ledger. A layout test ensures the enum remains smaller than its raw inference payload without claiming throughput or allocation improvements. The inherited full ledger suite retains detailed inference append, projection, rejection and resource-rollup behavior coverage, and Headless qualifies the embedded-runtime callers.

This is a public Rust construction and owned-pattern-binding change despite publish=false: callers wrap raw values in Box::new. It adds one allocation for this variant, with unchanged wire format and raw field semantics. Root source review accepted all six files at frozen tree e7a57e8d1bc9db3100c75dcb45ae9ef4dad8d67a. Hosted execution remains pending. No local Rust execution is claimed.
