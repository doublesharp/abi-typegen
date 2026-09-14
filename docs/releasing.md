# Releasing

The native release and package publication are separate workflows. Build and
publish matching binary assets before publishing npm packages: npm postinstall
uses the package version to select a GitHub release archive.

## Versions and validation

Keep the CLI, workspace crates, internal dependency requirements, and both npm
packages on the intended release version. Update Cargo.lock along with Rust
manifest changes. Move the relevant Unreleased changelog entries into the release
section and use that version for the `v<version>` tag.

Run the [development checks](development.md) and the generated-binding checks
appropriate to the changes. Release and publication workflows are defined in
[release.yml](../.github/workflows/release.yml) and
[publish.yml](../.github/workflows/publish.yml).

## Native release

Pushing a `v*` tag triggers the Release workflow. It builds archives for every
[supported platform](installation.md#prebuilt-platforms), collects those archives,
and runs:

```sh
node npm/abi-typegen/scripts/release-checksums.mjs generate release-assets release-assets/checksums.json
```

The generator requires `GITHUB_REF_NAME` to equal `v` followed by the npm package
version and requires every supported archive to exist. It hashes the archives
and writes a manifest with this shape:

```json
{
  "version": "<release-version>",
  "files": {
    "<platform-archive-name>": "<64-character SHA-256 digest>"
  }
}
```

The actual manifest contains exactly the supported archive names. The workflow
uploads the archives and manifest to the tagged GitHub release.

## npm publication

Once the native release is available, run the Publish workflow for the matching
source revision. Its npm job prepares the binary package before publishing:

```sh
node npm/abi-typegen/scripts/release-checksums.mjs prepare
```

Preparation downloads the manifest for the package version, validates its version
and complete target set, and downloads and hashes every listed archive. Only after
all hashes match does it write `npm/abi-typegen/checksums.json`. A failure removes
stale pinned hashes and prevents the workflow from publishing that package.

Ordinary `npm pack` validates the local manifest through `prepack`. The publication
workflow uses `--ignore-scripts`, so it runs preparation explicitly first. Do not
skip that step when reproducing the workflow manually.

The npm package's file list includes the manifest, launcher, and installer scripts.
It excludes local `bin/` output. Publish the binary package before the Hardhat
plugin, which depends on it. Hashes remain fixed in that npm package even if a
GitHub release asset is later replaced.

## Rust crate publication

The Publish workflow runs semantic-version compatibility checks for the library
crates, then publishes the core, configuration, code generation, and CLI crates in
dependency order. It uses the configured crates.io token. The npm job uses the
workflow's npm publishing identity and provenance configuration.

## Verifying a release

Use a fresh directory to install the exact published package version, then run its
`abi-typegen --version` command and generate bindings from representative artifacts.
Check that the version, release tag, platform archive, and npm manifest agree.
Installing an exact version avoids accidentally testing a previously cached CLI
or a different dist-tag.

A failed checksum comparison is a publication or artifact-integrity problem.
Investigate the manifest and archive; do not disable the installer check. Existing
releases that have no checksum manifest cannot satisfy this preparation flow
without completing the corresponding release assets.
