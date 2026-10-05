# AppConfig filesystem repair and startup composition qualification

Successor branch `qualification/shared-resource-app-config` starts from frozen
`73211ddc8a6fb6c1c758e1e1aab7a4474b42e73a`, tree
`348ed08771b7da26fffa4e0a4815b659e8c06266`. Parent owns review, PRs and hosted
execution. Earlier source milestones, PR54 work and dependency versions remain
unchanged.

## Concrete defect and production boundary

The frozen AppConfig loader tests `Path::exists` before reading `config.json`.
That API hides filesystem errors: denied directory traversal, a non-directory
parent and a broken symlink can all select unconstrained default configuration.
Startup must preserve a present configuration's failures instead of silently
discarding its shared resource declarations.

`pantograph-app-config` now owns the existing full application settings types,
device defaults and Tokio persistence implementation. The desktop config module
reexports its consumed types and retains the inference-owned server-mode alias.
The authoritative device defaults remain `auto` and `-1`; no persisted JSON
shape or dependency version changes. The root lockfile adds only the local crate
and its desktop dependency edge.

The production `load_with_runtime_registry` method loads the actual full AppConfig
and invokes the existing registry composition owner before returning either.
Tauri setup calls this method and manages/passes the same returned registry Arc
to its existing startup consumers. Errors publish neither fallback configuration
nor a partially composed registry. The focused CI job executes this production
crate's tests before inference suites.

Loading starts with `read_to_string`. Read errors other than `NotFound` are
returned, as are JSON parse failures. A `NotFound` result triggers fallible
symlink metadata checks, walking missing ancestors to a readable existing entry.
A present configuration symlink or dangling ancestor is an error, and metadata
errors cannot prove absence. A genuinely absent file or app-data directory still
uses defaults. No hardware topology, capacities or runtime readiness are inferred.

## Executed evidence

Using Rust 1.92.0 and the repository's workspace lockfile:

```bash
source /workspace/pantograph-tools/activate.sh
cargo test --locked --offline -p pantograph-app-config -p pantograph-runtime-registry -p pantograph-scheduler
cargo clippy --locked --offline -p pantograph-app-config -p pantograph-runtime-registry -p pantograph-scheduler --all-targets -- -D warnings
cargo fmt --all -- --check
node scripts/check-critical-antipatterns.mjs
bash scripts/check-scheduler-only-workflow-execution.sh
git diff --check
```

The actual suites pass 10 AppConfig tests (three unit and seven integration),
114 registry and 133 scheduler tests: 257 total, zero failed or ignored. The
AppConfig tests execute full settings wire round-trip, actual disk load/save and
fresh startup composition. Real threads contend for one declared backing pool;
existing reservations exercise replacement rejection, provisional rollback and
transfer. Genuine absence, nested missing directories, legacy/empty declarations,
malformed JSON, invalid domains, an unreadable directory, inaccessible parent,
non-directory parent, and broken/looping configuration or ancestor symlinks are
covered. The permission test in this unprivileged environment confirms both the
raw filesystem read and production startup return `PermissionDenied`.

For a regression baseline, an isolated temporary workspace copies these three
production crates and the same tests. Only `AppConfig::load` is replaced with
its exact frozen `73211ddc` source. The integration command exits 101 with four
passed and three failed: inaccessible parent, non-directory parent and broken
symlink. The fixed loader passes all seven. This baseline uses the real filesystem
and registry, with no desktop or native runtime claim. The workspace source and
frozen branches are never modified for this comparison.

Logs: `/tmp/app-config-all-portable-tests.log`,
`/tmp/app-config-qualified-tests.log` (separate target, including permission
evidence), `/tmp/app-config-loader-baseline.log`,
`/tmp/app-config-final-tests.log` and `/tmp/app-config-all-clippy.log`.

## Exact dependency attempts and remaining blocker

The owner authorized installing the documented official dependencies from
`scripts/install-ubuntu-build-dependencies.sh`. The environment is unprivileged
Debian, with no `sudo`. `apt-get update` was run through the supported escalation
request and returned exit 100:

```text
W: Unable to read /etc/apt/apt.conf.d/80-applied-apt-retries - open (13: Permission denied)
E: List directory /var/lib/apt/lists/partial is missing. - Acquire (13: Permission denied)
```

This is an OS permission failure, not an automatic approval-review rejection.
No packages were installed and no security flags, APT sources or denial bypasses
were changed. The subsequent native prerequisite check was:

```bash
pkg-config --modversion gtk+-3.0 webkit2gtk-4.1 javascriptcoregtk-4.1 libsoup-3.0
```

It exits 1; all four package declarations are absent. The first actual desktop
test-build attempt was:

```bash
cargo test --locked --offline -p pantograph --bin pantograph --no-default-features config::tests --no-run
```

It exits 101 before compilation because locked `aligned-vec v0.6.4` was not
cached. The ordinary online desktop attempt, retaining the existing no-download
restriction on the unavailable ONNX runtime, was:

```bash
ORT_SKIP_DOWNLOAD=1 cargo test --locked -p pantograph --bin pantograph --no-default-features --no-run
```

It downloads ordinary locked Rust dependencies and exits 101 in the real
`glib-sys v0.18.1` build script. Its actual pkg-config invocation cannot find
`glib-2.0 >= 2.70` / `glib-2.0.pc`. An independent native GTK dependency check
`cargo check --locked -p gtk-sys` at a separate target directory reaches the
same GLib error. Its initial `--offline` attempt stopped at uncached
`askama v0.12.1`. Logs are `/tmp/app-config-desktop-attempt.log`,
`/tmp/app-config-desktop-build.log`, `/tmp/app-config-desktop-prerequisites.log`,
`/tmp/app-config-gtk-build.log` and `/tmp/app-config-gtk-build-online.log`.

Zero native desktop tests executed. The production AppConfig/settings and
composition implementation is qualified at its real crate boundary; Tauri
startup/IPC, window creation, desktop-process cold reopen, actual model inference
and physical allocator safety are not qualified. This does not close DA-03/DA-07
or the earlier native sampling execution gap. A supported privileged desktop
runner must install the documented dependencies and qualify the final source;
the pinned native-runtime blocker remains separate.
