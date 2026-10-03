# UniFFI Managed Runtime Readiness Fixture

The current hosted UniFFI runtime suite exposes a false readiness expectation:
`create_temp_root` writes legacy fixture files under `app-data/runtimes/llama-cpp`,
but the managed-runtime owner resolves `app-data/third-party/runtimes/llama-cpp`
or a selected managed version. See inference/src/managed_runtime/paths.rs and
operations/state_transitions.rs:runtime_install_dir_for_projection. On the
supported test platforms the current installation validator reports required
files missing; operations/projection.rs and neutral_contracts.rs preserve that
Missing state rather than marking a runtime Ready.

Keep the fixture's legacy files and assert the honest neutral projection:
llama_cpp/runtime_sidecar exists, install and readiness states are Missing,
available is false, and missing_files is nonempty. All media dependency listing,
staging/install/remove and conversion-capability assertions remain. This repairs
the test's evidence claim without fabricating a managed installation or weakening
production readiness rules.

Rustfmt and whitespace pass. Independent source review and hosted execution are
pending; no local Rust build was attempted. The validation/publication API bridge
is separately reviewed, and native/C# plus aggregate lint/Clippy/audit qualification
remain open.

Independent integrator source review accepted staged tree
`2e98f08e0c780bd5becc8a74925dac9bbd252ec6` as an explicit regression that
legacy-location files cannot establish a current managed installation. Hosted
execution remains required.
