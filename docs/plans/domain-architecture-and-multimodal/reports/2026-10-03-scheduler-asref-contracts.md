# Scheduler validated-wrapper AsRef compatibility

## Decision and public surface

Move the fourteen validated scheduler wrapper accessors from inherent methods into standard AsRef implementations, preserving exported wrapper types, borrowed raw targets, lifetime elision, and direct `&self.0` references. Constructors, validation, ownership, serialization, and into_inner remain unchanged. This addresses should_implement_trait without suppressing the lint.

A Rust 1.92 standalone Clippy probe showed that retaining an inherent accessor alongside the trait still triggers the lint. A trait-only probe compiled and executed method, type-qualified, fully qualified, and typed function-pointer forms. Repository inspection found no no_implicit_prelude usage or explicit validated scheduler UFCS references. The crate is publish=false but its types are publicly re-exported, so external Git/path consumers remain relevant: a niche consumer disabling the Rust prelude must import AsRef explicitly. Trait-only calls rely on the standard prelude. No claim is made about unseen consumers.

The integration compile fixtures cover all fourteen exported wrapper/raw pairs with method calls, type-qualified calls, typed function pointers, and explicitly trait-qualified calls. The Quality workflow runs the scheduler suite and a targeted should_implement_trait Clippy check, while retaining the existing aggregate warning-deny audit unchanged.

## Verification and remaining scope

The small standalone probe passed locally; formatting and whitespace checks pass. No local workspace Rust build was run due to disk constraints. Actual scheduler tests and targeted Clippy qualification require fresh hosted execution. Independent source review is pending.

The runtime execution enum size/construction API, redundant must_use annotation, managed dependency needless borrow, and later aggregate Clippy findings remain separate. This is an accessor-contract repair, not a scheduler redesign.

Independent source review accepted tree ea62f92c5bfed9225adcf753a77c5be55d23475e. Accessor must_use annotations follow the standard trait method semantics; the validated wrapper types remain must_use. This acceptance covers the source/API migration and hosted checks, not an unperformed local workspace compilation.
