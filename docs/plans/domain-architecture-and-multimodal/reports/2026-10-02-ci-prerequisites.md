# CI Build Prerequisite Repair

Date: 2026-10-02. Status: `Verifying`.
Parent authority: [active domain architecture plan](../plan.md), CI-01 baseline
repair scope. Branch: `fix/ci-build-prerequisites-2026-10-02`, admitted from gate
`e22ebb2fe52efc8cfcaac6b23c4a902c7bc6a571`. Initial PR target is the gate branch;
retarget `main` after its prerequisite qualifies. The tooling owner implements
this isolated proposal; the domain integrator owns review and integration.
The checkout is retained for review until integrated, rejected or superseded.
Standards inspected: `MrScripty/Coding-Standards@dcc56f26e884ade260770beceba2501d3746200d`,
Core/Router, Implementation, Verification, Commit, Documentation, Tooling, Build,
Dependencies/Licensing and applicable Rust tooling guidance.

## Diagnosed Obstructions

- The headless workflow asks crates.io for generator `uniffi-bindgen-cs` 0.9.0,
  which is unavailable there. This reproduces on docs-only PR #1 and gate PR #2
  before any workflow-service or C# smoke runs. Source:
  [headless run](https://github.com/MrScripty/Pantograph/actions/runs/37066045057).
- Runtime-separation CI builds the actual Tauri binary but never provisions the
  GLib/GTK/WebKit native prerequisites, stopping in `glib-sys`. Source:
  [separation run](https://github.com/MrScripty/Pantograph/actions/runs/37066045102).
- Workspace check already provisions desktop libraries but Lance's build needs
  `protoc`; the runner has no compiler. Source:
  [baseline quality run](https://github.com/MrScripty/Pantograph/actions/runs/37059945912).

These are build prerequisites, not permission to skip checks, reduce features,
relax warnings, or accept the runtime source. Other failing baseline contracts,
Clippy, frontend lint and dependency-audit findings remain separate work.

## Coherent Repair

Use the official generator source rather than an unavailable registry release.
NordSecurity's [tag](https://github.com/NordSecurity/uniffi-bindgen-cs/tree/v0.9.0%2Bv0.28.3)
resolves to `2f4880f03ed08d960ad0ee14d11cf95444eee540`; its manifest and lockfile
identify generator `0.9.0+v0.28.3` and UniFFI 0.28.3, matching this repository's
0.28 consumer. Its README documents Git installation and Rust 1.81 minimum.
Pin that exact commit with `--locked`, retaining the intended generator version.
No application Cargo declaration, Cargo lockfile, generated C# or Pumas pin changes.
The C# binding guide owns local installation instructions; missing-tool messages
link that owner instead of repeating a registry/version guess. The generator is
an unchanged development executable, not vendored or packaged in this proposal;
its [MPL-2.0 source notices](https://github.com/NordSecurity/uniffi-bindgen-cs/blob/2f4880f03ed08d960ad0ee14d11cf95444eee540/LICENSE)
remain upstream. Existing binding/release obligations are not waived.

Consolidate the two existing identical Ubuntu native package lists into
`scripts/install-ubuntu-build-dependencies.sh`, add `protobuf-compiler`, and use
that same owned list for the runtime-separation build. The only permitted system
mutation is the existing hosted Ubuntu provisioning step; no local or user
computer system packages were installed. Failed package installation fails the
step. The existing Rust checks, features, ordering and failure behavior remain.

Write set: three affected workflow YAML files, the shared Ubuntu installer,
the scripts and C# guides, two C# scripts' missing-tool messages, and this report.
No runtime source, test fixture, severity, broad dependency upgrade, published
branch history or presentation artifact changes.

## Evidence And Remaining Claims

- Official tag resolution, source manifest, lockfile and README checked through
  GitHub at the exact generator revision.
- All workflow YAML parses; all three affected shell scripts pass `bash -n`.
- A disposable `sudo` substitute verified the exact apt update/install argument
  sequence, retained GLib/Desktop packages, added protobuf compiler and fail-fast
  propagation of an installation error. This proves orchestration only; it does
  not establish successful apt provisioning.
- Staged whitespace, staged traceability (nine paths, no ownership impact) and
  tooling lint passed before publication.
- Actual generator installation, produced binding compatibility, native compile
  and all downstream CI claims remain pending on an Ubuntu hosted runner.
- Independent integrator review accepted staged tree
  `a7567c4563dc09a145ee897983c31c8a8b3ec36f`: all nine files inspected,
  Bash syntax and whitespace independently checked, and upstream generator and
  workspace manifests verified at the exact pinned revision. This bounded source
  review found no blocker; actual provisioning and native/generator builds still
  require hosted qualification. This report records that acceptance after review.
- This proposal remains a draft, not a green-CI or release claim. No merge until
  required exact-head CI and applicable hosted review qualify it and its prerequisite.
