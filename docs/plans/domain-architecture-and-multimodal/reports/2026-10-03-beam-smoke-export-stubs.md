# BEAM Smoke Export Stub Repair

Hosted Quality run 37099224216 at PR #9 head
`e1bf2d8d4426566308fb9ce72f4c4f1895512568` builds the Rustler NIF successfully,
then job 111135294915 fails all five BEAM smoke tests. The loader reports
`Function not found 'Elixir.Pantograph.Native':pumas_model_package_facts_summary_snapshot/3`.

The default Rust NIF surface has added three exports since the smoke shim was
written: `pumas_model_package_facts_summary_snapshot/3`,
`pumas_resolve_model_package_facts_summary/2`, and
`pumas_list_model_library_updates_since/3`. Add their matching generated Elixir
stub declarations so the BEAM loader can replace them with the native functions.
No native implementation, feature selection, loader error handling, or assertions
change. The existing five tests exercise real loading and workflow/registry
round trips and will fail if export registration remains inconsistent.

Source inspection finds 67 default Rust exports and 67 matching shim names after
this change, with the three new arities checked directly against their Rust
signatures. Seven additional Rust exports are guarded by `frontend-http`; the
canonical smoke runner uses the default feature set and does not include them.
This is bounded source evidence, not an ABI execution claim or a new permanent
source-parser gate. No broader model-library query behavior is claimed.

Staged whitespace passes. Elixir/Mix are unavailable locally and no Rust rebuild
was attempted under the current disk constraint. Fresh hosted Rustler BEAM smoke
qualification and independent source review are pending. PR #9's workflow-service
tests already pass, while its downstream Headless qualification is still running.
Existing lint, Clippy and dependency audit failures remain separate; no merge is
qualified by this repair alone.

Independent integrator source review accepted staged tree
`ad33e068e8057e70e8552fdb6e9f122d3ac82667` after comparing all three new
names/arities against the Rust NIF definitions and reading the unchanged generated
stub behavior. Actual BEAM loading remains a hosted qualification requirement.
