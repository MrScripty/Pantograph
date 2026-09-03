# Rust Crate Map

The root `Cargo.toml` is the authoritative workspace-member and dependency
list. [ARCHITECTURE.md](../ARCHITECTURE.md) owns responsibility and dependency
direction. This page is only a compact navigation map.

| Area | Crates |
| --- | --- |
| Workflow domain | `pantograph-workflow-service`, `pantograph-scheduler`, `node-engine`, `workflow-nodes` |
| Runtime composition | `pantograph-embedded-runtime`, `pantograph-runtime-host-contracts`, `pantograph-runtime-registry`, `pantograph-runtime-identity` |
| Inference and interfaces | `inference`, `pantograph-inference-interface-contracts` |
| Dependency and media services | `pantograph-dependency-planning`, `pantograph-dependency-environment-service`, `pantograph-managed-dependencies`, `pantograph-media-conversion` |
| Durable records | `pantograph-runtime-attribution`, `pantograph-diagnostics-ledger` |
| Shared contracts and safety | `pantograph-node-contracts`, `pantograph-timing-contracts`, `pantograph-path-security` |
| Adapters | `pantograph-frontend-http-adapter`, `pantograph-uniffi`, `pantograph-rustler`, `src-tauri` |

Crates are internal workspace units and currently declare `publish = false`.
Their Rust modules and public types are the exact API authority. Add a crate
README only when it explains a real external consumer contract that rustdoc and
the repository architecture entry point cannot express more directly.
