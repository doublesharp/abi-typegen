# Build and cache storage

Storage redirection is **off by default**. Ordinary clones use Cargo's `target/`
and each package manager's usual dependency and cache locations. `cargo build`,
`cargo test`, `make test`, and `pnpm install` need no setup, Scratch volume, or
sccache installation. Python is required only for this optional tooling and its
separate `make test-storage` tests.

Choose a storage directory once per checkout. Python 3.11+ and a POSIX system
such as macOS or Linux are required. No location is built into the project tooling.
Any writable directory outside the checkout works; a separate disk is optional.
The project's normal development commands do not require POSIX storage setup on
Windows.

```sh
python3 .cargo/setup-scratch.py --root /path/to/disposable/abi-typegen
```

For storage on a separately mounted volume, also pass `--volume /mount/path`.
Setup checks that this mount exists and contains the storage directory, and saves
that requirement for later setup runs. Use a distinct root for each checkout.
The volume's snapshot and backup exclusions are managed by the operating system
or backup application; this script does not change them.

The choice is saved in `.cargo/scratch.local.json`. Setup generates ignored Cargo,
pnpm, and Make configuration from it. After setup, use ordinary commands:

```sh
cargo build
cargo test
make coverage
cd npm && pnpm install --frozen-lockfile --ignore-scripts
```

No repeated storage flag or wrapper is required. Cargo sends builds, docs, compiler
cache, and temporary files to the saved root. Make loads the generated environment
for every recipe. The local pnpm settings route its package store, metadata cache,
and installed packages (pnpm's virtual store) to the saved root. Each project's
`node_modules` stays a real local directory, because pnpm 12 writes links into it
and rejects a symlinked one; it holds only links into storage. Existing relative
paths such as `target/debug/abi-typegen` continue to work through directory
symlinks.

For unrelated commands or plain npm, load the environment once per shell:

```sh
. .cargo/scratch.local.env
```

Alternatively, `.cargo/scratch.sh COMMAND [ARG ...]` runs an individual command
with the saved environment. Without saved settings, the wrapper runs the command
with the normal environment and creates no configuration. A bare pnpm invocation
routes its dependencies and
caches automatically; use Make or the shell environment for its temporary files.
Explicit output paths and shell redirections still write wherever you specify.

## Changing or restoring the location

Run the setup command with a different `--root` to save a new location. Stop
builds, installers, fuzzers, and this checkout's sccache server first. The script
copies the discovered fuzz corpus and coverage history to the new root. Other
build output and dependencies start fresh, so rerun dependency installation as
needed. Old data remains at the previous root until you remove it yourself.
Switching to a destination containing conflicting output is rejected.

To recreate deleted directories or links using the saved setting:

```sh
make scratch-setup
```

Tools that delete a whole output directory can remove its symlink. The benchmark
currently does this for generated bindings and TypeChain output. Reapply setup
after cleaning or benchmarking. If a tool has already created a real directory
where a link used to be, setup refuses to merge it with existing storage data;
inspect both copies before removing either disposable copy.

## What moves

`.cargo/setup-scratch.py` lists all redirected directories: Cargo and fuzz output,
coverage reports and history, discovered fuzz corpus, installed Node packages, the npm
CLI binary, and untracked Foundry/Hardhat artifacts and bindings. Existing local
disposable directories are preserved when initially moved.

Source, Git history, lockfiles, credentials, curated fuzz seeds, crash inputs, and
Foundry's existing `lib` directory stay in the checkout. Tracked generated Zod and
Solidity bindings and `out-solidity-validation` also stay. Global Cargo downloads
and tool installations retain their existing locations.

Setup refuses tracked output paths, unknown symlinks, conflicting data, or
unrelated Cargo/pnpm configuration. It does not install or enable sccache; if you
already use it, its cache is routed to the chosen storage directory.

Only reusable tooling is committed. Saved choices and generated tool settings
are ignored, so unconfigured clones and CI keep their normal storage defaults.

## Disabling

Stop active builds, package installers, fuzzers, and this checkout's sccache server,
then run:

```sh
python3 .cargo/setup-scratch.py --disable
# or: make scratch-disable
```

This removes only recognized links and unchanged generated settings, plus each
project's local `node_modules`, whose links point into storage. It copies
the discovered fuzz corpus and coverage history back into the checkout first.
All external storage data remains in place. Build output and dependencies use
normal local paths on subsequent commands; reinstall dependencies as needed.
If you sourced `.cargo/scratch.local.env`, start a new shell to clear those exports.
Disabling an unconfigured checkout is a no-op.

Re-enabling with a fresh root works directly. Reusing the previous root can expose
two copies of corpus/history: the restored local copy and the retained external
copy. Setup refuses to merge them automatically; reconcile them first or use a
fresh root. It also refuses modified generated configuration or unexpected links
instead of deleting them.

Checkouts configured before pnpm 12 support linked each `node_modules` into
storage. Rerunning `make scratch-setup` replaces those links with local
directories and keeps the installed packages; then rerun `pnpm install`. Install
with `--ignore-scripts` as shown: an install that meets an unapproved build script
adds an `allowBuilds` placeholder to the generated `pnpm-workspace.yaml`, and
rerunning setup restores the file.
