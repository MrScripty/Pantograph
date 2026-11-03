# Two typed error return boundaries

Fresh inference diagnostics identify only two remaining oversized error return owners after the managed diagnostic payload repair. Return Box<ManagedBinaryFacadeError> from resolve_managed_binary_command and Box<TaskRegistryResolutionDiagnostic> from resolve_task_registry_entry_from_evidence. Keep the concrete error enums/DTOs, all fields, Display/Debug messages, diagnostic JSON, successful result types, and validation/branch order unchanged. No other Result is boxed.

This explicitly changes two public Rust return types and adds allocation on their failure paths. Direct owned pattern matching needs dereferencing; the existing facade test is migrated. Repository consumers otherwise inspect fields, format diagnostics, or discard errors and retain that behavior. The changelog records the source change despite inference being publish=false; unseen consumers are not claimed compatible.

Tests cover actual missing-runtime facade resolution, all facade variants/exact Display, successful command field/argument-boundary projection, full successful canonical registry output, and exact diagnostic JSON/raw-shape decoding. Existing conflict/modality error tests remain. Command projection is not a claim that a real runtime was installed/launched. Hosted checks explicitly run the facade and registry resolution tests.

Formatting and whitespace pass locally. No local Rust execution is claimed. Independent source review and fresh hosted boundary/Clippy qualification are pending; public high-arity APIs remain separate.

Independent source review accepted tree 29515000a6205a212139bcc53a6c65acd465e9d7. This covers only the two boxed concrete error boundaries and preserved payloads/branch order. Pure successful-command projection remains distinct from real runtime installation/readiness qualification; fresh hosted tests are required.
