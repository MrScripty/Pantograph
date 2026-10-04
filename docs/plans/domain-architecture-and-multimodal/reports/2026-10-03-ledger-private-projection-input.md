# Private ledger projection input and ordered timeline formatting

Fresh PR #32 Clippy reports one private eight-argument projection writer and one bool::then filter_map in diagnostics-ledger. Reuse the existing ProjectionStateWrite at the sole failure-writer caller, preserving validation, transaction order, Failed status, all eight stored values, exact expect messages and the lower SQL writer. Remove only the redundant private wrapper.

Replace the timeline filter_map with ordered filter/map. Exact all-category and sparse text tests preserve label order and omission of zero counts; an empty-count test preserves None. The existing failure-health/recovery test now checks every stable stored field and reads the failed state back before recovery. Hosted CI runs the complete ledger crate suite. The public payload enum layout remains a separate change.

Whitespace and Rust syntax formatting are checked locally. Root source review accepted the exact four-file frozen tree ab465f369c064fc8c4d966e9fa99ae7f1ec1255f. Actual hosted Rust execution remains pending.
