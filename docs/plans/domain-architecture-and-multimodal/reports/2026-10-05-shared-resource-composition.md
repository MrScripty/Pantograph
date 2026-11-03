# Shared resource domains in desktop startup composition

Successor branch `feat/scheduler-shared-resource-composition` starts from frozen
`27af8aa3cf452cd1abfe9e21bb2df6368d05854c`, tree
`64f766057bd0facc4e1e72597528189e73920f2b`. It connects that registry admission
feature through existing application configuration and startup. Parent owns
review, PRs and hosted execution. Frozen sampling qualification and PR54 source
remain untouched; no dependency pins or native runtime are substituted.

## Concrete gap and acceptance

`src-tauri/src/config.rs` owns persisted application/device settings;
`app_setup.rs` creates the shared registry passed to hosted workflow composition.
Previously startup always created an empty, unconfigured registry. The device
selector and offload count are not physical backing-domain facts. The current
runtime-identity owner already defines canonical known runtime names and aliases.

Acceptance: persisted explicit declarations compose the registry used by actual
startup; an absent section retains existing behavior; no topology or readiness is
inferred; two member runtime requests compete for one capacity; unified memory has
one backing capacity; replacement/custody rollback preserves shared accounting;
invalid declarations cannot silently disable constraints. Live domain edits have
an explicit restart requirement.

## Resulting composition

AppConfig flattens the registry-owned `RuntimeResourceDomainConfig` into optional
`runtime_resource_domains` in existing `config.json`. The JSON binding/domain
types reject unknown fields and resource kinds. Its portable startup factory
canonicalizes known runtime aliases, registers identities without capabilities,
readiness or model facts, then installs all domains before returning the registry.
An invalid declaration returns an error without publishing a partial registry.
Ordinary producer reconciliation remains the capability/readiness owner.

Desktop setup loads configuration and composes/manages this registry before
creating the gateway and hosted workflow startup consumers. It passes that same
Arc through the existing resource-backed composition. A missing file defaults;
a present unreadable or invalid file fails startup. The former broad fallback
could discard explicit capacity constraints, so it is deliberately removed.
The app config getter/save round-trip includes the new section, and the setter
rejects changing it while the current process uses another declaration. Operators
edit the existing file and restart. No new UI, command, config file, discovery
service, runtime allocator or scheduling policy is added.

One domain names a unified backing capacity once and can bind both logical
resource kinds. Duplicate logical bindings and overlapping pools reject.
Declared distinct allocation envelopes are summed; physical allocation aliases
or missing claims are not guessed. No measured hardware/allocator safety claim.
Canonical usage and the illustrative JSON live in
[runtime operations](../../../runtime-operations.md#shared-resource-admission).

## Executed evidence and limitations

The portable public composition suite parses the same production config
projection/factory, reads a persisted declaration into fresh registries, and
executes actual cross-runtime contention, failed replacement, provisional
rollback/transfer, producer reconciliation, unified-memory accounting and
invalid/absent configuration. It does not emulate inference or replace native
commands. Eight new composition tests pass. Full portable suites pass 114 registry
plus 133 scheduler tests, zero failed or ignored; the final affected composition
suite also passes after adding persisted-file coverage. All-target registry and
scheduler Clippy with `-D warnings`, frontend TypeScript checking, formatting,
whitespace, critical and scheduler-only surface gates pass. Staged/range
traceability is checked before source upload.

Two AppConfig tests additionally cover the full desktop struct's flattened wire
round-trip and legacy default. They are authored but not executed locally:
GTK/WebKit development prerequisites are unavailable (`pkg-config --exists
gtk+-3.0 webkit2gtk-4.1` exits 1). No GTK installation or native desktop test/build
is attempted. The portable tests qualify the production configuration factory
and registry composition; they do not qualify Tauri setup/IPC execution, device
topology, actual model inference or a desktop process cold reopen.

Logs: `/tmp/shared-composition-focused.log`, `/tmp/shared-composition-tests.log`,
`/tmp/shared-composition-clippy.log`, `/tmp/shared-composition-typecheck.log`.
Pinned ONNX HTTP 403 still blocks native sampling qualification at `6aa6b717`;
this successor neither retries that download nor claims those tests executed.
