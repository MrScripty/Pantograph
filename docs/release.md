# Release

Pantograph does not currently have an accepted release workflow or a verified
multi-platform support matrix. Existing packaging and SBOM scripts are
development scaffolding; the current audit found that they do not yet prove the
identity or behavior of final artifacts.

## Target Contract

A future release candidate must bind every artifact to:

- product version and source revision;
- Rust target triple and architecture;
- artifact role and format;
- native ABI and generated-binding cohort where applicable;
- checksums, provenance, required notices, and an SBOM generated from the final
  collected bytes; and
- the consumer and supported platform claim it is intended to satisfy.

Expected artifact roles include the desktop application, the headless native
library, generated host-language bindings, metadata/notices, checksums, and
SBOM output. Naming and composition are not authoritative until the artifact
plan introduced by the
[dependency/release remediation](plans/current-standards-remediation/dependencies-release-and-documentation/plan.md)
is accepted.

## Acceptance

Source checks do not prove a release artifact. Candidate acceptance requires
the exact packaged unit from the same candidate run to be loaded or launched in
the environment named by its support claim, with a bounded observable result.
Unavailable target runners or missing artifact identity block the claim; they
do not fall back to a source-tree smoke.

Publication is a separate operation and is not authorized by the current
remediation portfolio. A destination, channel, credentials owner, withdrawal
procedure, and final candidate identity must be explicitly supplied before any
external release mutation.

## Current Mechanisms

- `scripts/package-uniffi-csharp-artifacts.sh` packages development binding
  output but does not yet establish trustworthy target/version identity.
- `scripts/generate-release-sbom.sh` currently relies on ambient tooling and
  does not inspect only final collected artifact bytes.
- `./launcher.sh --release-smoke` currently labels source checks as artifact
  evidence and must not be used as release acceptance.
- `CHANGELOG.md` is the human-maintained change summary.

Track remediation and required-real evidence in the
[current standards portfolio](plans/current-standards-remediation/plan.md).
