# Proposal: Built-in Node Registration

## Problem

`NodeRegistry::new()` creates an empty registry. Pantograph defines 23 built-in nodes via `TaskDescriptor` implementations in `workflow-nodes`, but there is no mechanism to bulk-register them. Consumers (e.g. puma-bot) call `node_registry_new()` via NIF and get an empty palette — no built-in nodes appear.

Currently:
1. There is no function to collect all descriptors dynamically
2. There is no NIF to bulk-register built-in nodes
3. Each consumer would need to manually construct and register every node's metadata JSON

## Proposed Solution

### Part 1: Auto-collect descriptors with `inventory` crate

Use the [`inventory`](https://docs.rs/inventory) crate to automatically collect all `TaskMetadata` at link time. Any node that implements `TaskDescriptor` and calls `inventory::submit!` is automatically included — no hardcoded list to maintain.

**`crates/node-engine/Cargo.toml`** — add dependency:
```toml
inventory = "0.3"
```

**`crates/node-engine/src/descriptor.rs`** — make `TaskMetadata` collectable:
```rust
inventory::collect!(TaskMetadata);
```

**Each node implementation** — submit its descriptor (e.g. in `text_input.rs`):
```rust
inventory::submit!(TextInputTask::descriptor());
```

Adding a new node in the future only requires this one line — no central registry file to update.

### Part 2: Add `register_builtins()` to `NodeRegistry`

**`crates/node-engine/src/registry.rs`**:
```rust
impl NodeRegistry {
    /// Register all built-in node types collected via `inventory`.
    pub fn register_builtins(&mut self) {
        for metadata in inventory::iter::<TaskMetadata> {
            self.register_metadata(metadata.clone());
        }
    }
}
```

### Part 3: Expose via NIF

**`crates/pantograph-rustler/src/lib.rs`** — new NIF function:
```rust
#[rustler::nif]
fn node_registry_register_builtins(
    resource: ResourceArc<NodeRegistryResource>,
) -> NifResult<Atom> {
    let mut registry = resource.registry.blocking_write();
    registry.register_builtins();
    Ok(atoms::ok())
}
```

Add `node_registry_register_builtins` to the `rustler::init!` function list.

## Alternative if `inventory` is rejected

A manual `all_descriptors() -> Vec<TaskMetadata>` function in `workflow-nodes/src/lib.rs` would work, but must be kept in sync whenever nodes are added or removed. The `inventory` approach is preferred because it scales automatically.

## Files to modify

| File | Change |
|------|--------|
| `crates/node-engine/Cargo.toml` | Add `inventory = "0.3"` |
| `crates/node-engine/src/descriptor.rs` | Add `inventory::collect!(TaskMetadata)` |
| `crates/node-engine/src/registry.rs` | Add `register_builtins()` method |
| `crates/workflow-nodes/src/**/*.rs` | Add `inventory::submit!()` to each node |
| `crates/pantograph-rustler/src/lib.rs` | Add `node_registry_register_builtins` NIF |

## Consumer usage (e.g. puma-bot)

After this ships, consumers add one NIF binding and one init call:

```elixir
# lib/pantograph/native.ex
def node_registry_register_builtins(_registry), do: :erlang.nif_error(:nif_not_loaded)

# lib/puma_bot/workflow/node_registry.ex
defp init_registry do
  registry = Native.node_registry_new()
  Native.node_registry_register_builtins(registry)
  registry
end
```
