# Releasing

Pushing a `v<version>` tag runs one workflow,
[release.yml](../.github/workflows/release.yml). It verifies versions, builds the
native archives, creates the GitHub release, and then publishes to crates.io and
npm. npm publishing waits for the release because npm postinstall uses the package
version to select a GitHub release archive.

To release, commit the version bump and changelog on `main`, then:

```sh
git tag -a v<version> -m "Release <version>"
git push origin v<version>
```

## Versions and validation

Keep the CLI, workspace crates including `abi-typegen-runtime`, internal dependency requirements, and both npm
packages on the intended release version. Update Cargo.lock along with Rust
manifest changes. Move the relevant Unreleased changelog entries into the release
section and use that version for the `v<version>` tag.

Run the [development checks](development.md) and the generated-binding checks
appropriate to the changes before tagging. The workflow's first job fails the
release, before anything is built, if the tag disagrees with any crate version,
either npm package version, the Hardhat plugin's dependency range, or a
`## [<version>]` heading in `CHANGELOG.md`.

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

After the GitHub release job succeeds, the npm job prepares the binary package
before publishing:

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
It excludes local `bin/` output. The job publishes the binary package before the
Hardhat plugin, which depends on it. Hashes remain fixed in that npm package even
if a GitHub release asset is later replaced.

npm publishing uses trusted publishing (OIDC) with provenance. The trusted
publisher entry for both packages on npmjs.com must name `release.yml` as the
workflow.

## Rust crate publication

The release workflow runs semantic-version compatibility checks for the library
crates. After those checks and the GitHub release succeed, it publishes the core,
configuration, code generation, runtime, and CLI crates in dependency order with the
`CARGO_REGISTRY_TOKEN` secret.

## Re-running a failed release

Both publish jobs skip versions that are already on their registry. If a job fails
partway through, fix the cause and use "Re-run failed jobs" on the same workflow
run. Don't delete and re-push the tag to retry.

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

## C/C++ runtime compatibility

Release the runtime crate at the same version as the generator. The generated
`abi_typegen.h` and runtime header must match. Validate both C and C++ consumer
builds and ownership tests before publishing. Native CLI archives contain the
generator; C/C++ applications build and link the runtime separately.
