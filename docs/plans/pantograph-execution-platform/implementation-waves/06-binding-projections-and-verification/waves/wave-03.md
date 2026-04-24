# Wave 03: Host-Language Verification

## Objective

Add or update language-native tests that load the real native/generated
artifact for every supported host lane.

## Workers

| Worker | Primary Write Set | Report |
| ------ | ----------------- | ------ |
| csharp-host-tests | C# smoke or acceptance fixtures assigned in wave `01` | `reports/wave-03-worker-csharp-host-tests.md` |
| python-host-tests | Python package/smoke fixtures assigned in wave `01` | `reports/wave-03-worker-python-host-tests.md` |
| beam-host-tests | BEAM smoke fixtures assigned in wave `01` | `reports/wave-03-worker-beam-host-tests.md` |

## Worker Boundaries

- Each worker owns only its language-native test fixture, package metadata
  assigned in wave `01`, and report file.
- Workers may read binding crates but must not change Rust projection code in
  this wave.

## Shared Files

Generated artifacts, native library build scripts, release manifests, and root
workspace manifests are host-owned.

## Verification

Each worker runs or documents the lane-specific command recorded in wave `01`.
A lane without a real command cannot be reported as supported.

## Integration Order

1. Integrate C# host tests.
2. Integrate Python host tests.
3. Integrate BEAM host tests.
4. Host runs all supported host-lane smoke commands together.

## Escalation Rules

- Stop if a host test cannot load the real generated/native artifact.
- Stop if a language fixture needs host-local node semantics.
