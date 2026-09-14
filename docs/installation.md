# Installation

Choose an npm installation for a JavaScript project's local toolchain, or a native
installation for direct use from a shell. Generated bindings may require their
own SDK dependencies; see [generated output](generated-output.md).

## npm package

```sh
npm install --save-dev @0xdoublesharp/abi-typegen
npx abi-typegen --version
```

Equivalent package managers:

```sh
pnpm add -D @0xdoublesharp/abi-typegen
pnpm exec abi-typegen --version

yarn add -D @0xdoublesharp/abi-typegen
yarn exec abi-typegen --version
```

The npm package uses a Node launcher and a postinstall script. Installation needs
`curl` and `tar` on PATH and HTTPS access to GitHub release downloads. The Rust
compiler is not needed for a prebuilt binary. If your package manager blocks
lifecycle scripts, enable the package's postinstall script through that manager
before running the command.

### Prebuilt platforms

| Platform         | Architecture  | Release target              |
| ---------------- | ------------- | --------------------------- |
| macOS            | Intel x64     | `x86_64-apple-darwin`       |
| macOS            | Apple Silicon | `aarch64-apple-darwin`      |
| Linux with glibc | x64           | `x86_64-unknown-linux-gnu`  |
| Linux with glibc | ARM64         | `aarch64-unknown-linux-gnu` |
| Windows          | x64           | `x86_64-pc-windows-msvc`    |

Linux downloads are GNU/glibc builds. The installer does not select a musl binary
for Alpine. Use a compatible glibc environment or build from source for your
platform.

### Download and integrity checks

The installer reads `checksums.json` from the installed npm package and verifies
that it matches the package version and supported archive names. It then:

1. Downloads the platform archive into a temporary directory over HTTPS.
2. Computes SHA-256 for the complete archive and compares it with the pinned hash.
3. Extracts the expected binary and checks that it is a nonempty regular file.
4. Marks it executable and moves it into the package's `bin/` directory.

Downloads retry transient failures, including HTTP 504, up to three times after
the initial attempt, with exponential backoff. The connection timeout is 15
seconds, each transfer is limited to 60 seconds, and the download process has a
180-second deadline. HTTP redirects cannot downgrade to plain HTTP.

Missing hashes, mismatches, and extraction failures stop installation. Temporary
files are removed, and a failed attempt preserves any existing binary. Re-running
postinstall verifies a fresh archive rather than trusting an existing file.

The pinned hash detects corruption or archive replacement after npm publication.
It trusts the release and npm publishing process; it is not a signature or
independent proof of build provenance. See [releasing](releasing.md) for how the
manifest is produced and included in the package.

### Troubleshooting

| Failure                                  | What to check                                                                                                                                 |
| ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Repeated HTTP 502/503/504 or timeout     | GitHub availability and your runner's proxy/network connection. Retry the install after the connection recovers.                              |
| HTTP 404                                 | The package version must have a matching release and platform archive. A package version bump cannot substitute for missing release assets.   |
| Missing or invalid checksum manifest     | The package was packed without release preparation, or its contents are incomplete. Reinstall a correctly published package.                  |
| SHA-256 mismatch                         | The archive differs from the pinned package manifest. Stop using that download and investigate the release asset; do not bypass verification. |
| Command link exists but binary is absent | Check whether postinstall ran and completed successfully.                                                                                     |
| Binary will not run on Linux             | Check the CPU architecture and glibc compatibility, especially in Alpine containers.                                                          |

## Native installation

With a supported Rust toolchain:

```sh
cargo install abi-typegen --locked
abi-typegen --version
```

For a source checkout, see [development](development.md). Native CLI use does not
require Node; Node is used by the npm launcher and JavaScript project tooling.

Archives are also available on [GitHub Releases](https://github.com/doublesharp/abi-typegen/releases).
Match the tag and platform, verify the archive against that release's checksum
manifest when provided, then extract the executable into a directory on PATH.
