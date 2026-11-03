# Diagnostic event artifact payload indirection

Fresh PR #35 Clippy job 111211956000 identifies IoArtifactObserved as the remaining 640-byte largest DiagnosticEventPayload variant, compared with SchedulerRunAdmitted at 360 bytes. Box only this artifact variant. Preserve the raw DTO, validation, event-kind mapping, Serde tag and projection readers. Migrate the four existing constructors in embedded-runtime, workflow-service and their ledger/diagnostics fixtures. No other enum variant or Result type changes.

Add explicit minimal tagged JSON serialization/deserialization and SQLite append/read round-trip coverage, plus an enum-size bound. Inherited full ledger tests retain artifact projection and retention behavior, while Headless qualifies production callers. Public Rust variant construction and owned pattern binding now involve Box; the documentation records the added allocation despite publish=false and makes no measured performance claim.

Root approved this bounded follow-up from fresh evidence. Staged whitespace and new-test formatting are checked locally; root source review accepted all seven files at frozen tree 9271a602ebe593d0f45d29674ebc99ca7cb3a1cc; actual hosted execution remains pending. No local Rust execution is claimed.
