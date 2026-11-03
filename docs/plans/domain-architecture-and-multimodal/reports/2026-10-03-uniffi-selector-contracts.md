# UniFFI Selector Contract Fixtures

PR #12 Headless run 37101754525 compiled the repaired capability contract and
ran the frontend-http UniFFI configuration: 18 passed and five failed. Two
failures were stale selector fixtures referring to `puma-lib:model_path`, which
has been replaced by `pumas_model_ref` in the canonical node and options provider.

Update discovery to require `pumas_model_ref` and explicitly reject the retired
queryable port. Query options through the canonical port and require an object
with model-ref contract version 1 and the exact indexed model ID. Verify that the
value matches the canonical reference included in selector metadata; preserve
the old fixture's exact file association assertion against `display_entry_path`,
where display filesystem paths now belong. Cursor and model-presence assertions
remain unchanged. The provider source is workflow-nodes/src/input/puma_lib.rs:
selector_row_value, selector_row_option_metadata and port_option_from_selector_row.
These tests do not claim inference readiness or model execution.

Only two test functions and this report change. Rustfmt and whitespace pass;
independent source review and hosted tests remain required. No local build was
attempted. The three other runtime-test failures are separate: two require a
legitimate embedding validation/publication API bridge, and managed-runtime
readiness requires current installation evidence rather than legacy empty files.
Neither admission checks nor readiness semantics are weakened here.

Independent integrator source review accepted staged tree
`f106e00bcb9b43a7903baab5403c88a38ee9b7a8`: canonical reference identity,
version and metadata equality remain asserted, while display path and cursor
checks are preserved. Hosted qualification is still required.
