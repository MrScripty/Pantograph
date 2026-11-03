# Private gateway lifecycle and warmup context grouping

Replace eight repeated attribution parameters in two private lifecycle helpers with the existing InferenceRequestLifecycleEventContext. All thirteen existing call sites populate the same eight values; previously absent fields remain None via Default. The lower diagnostic recorder is unchanged. The context is moved through the helper, avoiding extra attribution clones.

Group the one-caller private record_start_result inputs into RuntimeWarmupStartContext while leaving its Result separate. Destructuring restores the same local names before the byte-identical success/rollback/timing body. No public generation/server signature, lock scope, timestamp source, runtime identity, lifecycle selection, or error behavior changes.

A complete-event regression checks both helper routes, all attribution values, detail, phase/kind, and explicit absence of variant/network/artifact fields; a default-context event confirms no invented attribution. Hosted steps also run existing warmup success, normalized failure and failed-restart rollback tests.

Formatting/whitespace checks and a source comparison of the unchanged warmup behavior block pass locally. No local Rust execution is claimed. Independent source review and fresh hosted tests/Clippy remain pending. Existing unrelated high-arity helpers and public APIs are untouched; no new lint suppression is added.

Independent source review accepted tree 03e481cd24fd8765c6b11c865ec73ff8619dd1e8. The reviewed PR #26 feature-gated projection-test correction was merged cleanly as the publication base; gateway source/test contents are unchanged by that inheritance. Actual hosted lifecycle/warmup execution remains required.
