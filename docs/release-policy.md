# Release Policy

Pantograph releases must produce versioned artifacts, checksums, and a
Software Bill of Materials from pinned toolchains.

## Version Source

Release tags use `vMAJOR.MINOR.PATCH`. For the current pre-1.0 app, the tag
version is the release version used in artifact names even though the Tauri app
manifest may keep its own product version until the release workflow is fully
automated.

## Artifact Names

Release assets are flat on GitHub, so every distributable file name includes
the product, version, and platform or artifact role.

| Artifact | Pattern |
| -------- | ------- |
| Desktop app | `Pantograph-{version}-{target}.{ext}` |
| Headless native library package | `pantograph-headless-native-{version}-{target}.zip` |
| C# binding package | `pantograph-csharp-bindings-{version}.zip` |
| Checksums | `checksums-sha256.txt` |
| SBOM | `pantograph-{version}-sbom.cdx.json` |

Use the toolchain-native target names for Rust/native artifacts, for example
`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, or
`aarch64-apple-darwin`.

## SBOM Generation

`scripts/generate-release-sbom.sh` generates a CycloneDX JSON SBOM with `syft`:

```bash
scripts/generate-release-sbom.sh 0.1.0
```

The output path is:

```text
target/release-artifacts/pantograph-0.1.0-sbom.cdx.json
```

The script fails fast if `syft` is missing. Release CI must install `syft`
explicitly before running the script.

## Release CI Outline

The release workflow should trigger on `v*` tags and create a draft GitHub
Release after all build and smoke jobs pass.

Required jobs:

1. Build desktop artifacts for supported platforms using pinned Rust, Node, npm,
   and system dependencies.
2. Package headless native and C# binding artifacts from the same tag.
3. Generate checksums for every release artifact.
4. Generate the CycloneDX SBOM through `scripts/generate-release-sbom.sh`.
5. Run `./launcher.sh --release-smoke` against the built release artifact in a
   clean runner environment.
6. Upload artifacts, checksums, and SBOM to a draft GitHub Release.

Release CI must keep CI-only GUI launch flags isolated to the bounded
`--release-smoke` path described in `docs/testing-and-release-strategy.md`.

## Changelog Automation Decision

Pantograph keeps `CHANGELOG.md` as the release-note source of truth for now.
Conventional commits remain required so release notes can later be automated
with `git-cliff`, but this repository will not add a generated changelog config
until the release workflow is in place and the desired grouping template is
reviewed.
