# Private runtime selection diagnostic errors

Fresh PR #39 aggregate Clippy clears workflow-service and stops at five private runtime-registry Result paths whose error is the same 224-byte RuntimeTechnicalFitDeviceDiagnostic DTO. Box only these errors in automatic_selection_policy_trace, automatic_candidate_set_summary, checked_candidate_count, active_reserved_bytes and checked_add_claim_bytes. Unwrap at the existing automatic decision-vector and resource-budget context adapters. Preserve public signatures, the raw DTO, diagnostic fields/JSON, ranking/accounting decisions and success values.

Boundary tests retain zero/u32-max candidate counts and exact overflow diagnostic JSON (the usize overflow case is explicitly a 64-bit test). The existing public selector resource-overflow test now checks the entire contextualized payload, while full registry tests retain successful selection, budget and underflow behavior. Only failing paths add a box allocation. No broad Result or public layout change is made.

Root approved this bounded registry branch using the existing checkout. Root source review accepted all four files at frozen tree e88e208b5567da1dda64974dd1c65bb4524d5801. Hosted full-crate/aggregate qualification remains pending; no local Rust execution is claimed.
