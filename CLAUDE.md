# Agent guidance

See [CONTRIBUTING.md](CONTRIBUTING.md) for development and
[docs/releasing.md](docs/releasing.md) for releases.
`AGENTS.md` links here; keep the shared file rather than duplicating it.

## Code

- No `unwrap()` in library code. Use `?` or `expect("reason")` for a justified invariant.
- No `unsafe` without a `// SAFETY:` comment.
- Use `alloy-primitives` types (`Address`, `B256`, `U256`), not raw `[u8; 32]` substitutes.
- Use `thiserror` for library errors and doc comments on every public item.
- Library logging uses `tracing`, never `println!` or `eprintln!`. Binary `println!`
  is for user-facing output only.
- Reject invalid inputs at parse boundaries, not during rendering.
- Inspect structured types directly; never infer types by searching rendered strings.
  Prefer exhaustive enum matches so new variants receive compiler checks.
- Flags must do exactly what their names say. Disabling wrappers must retain ABI
  modules, schemas, and other primary output.
- Add a regression test for each bug fix. Fix failing tests and errors even when
  preexisting. Run relevant formatting and linting before committing.

## Working preferences

- Preserve unrelated local work and commit by cohesive change.
- Keep `docs/` evergreen; exclude execution plans, task checklists, and one-off
  validation reports. Release history belongs in `CHANGELOG.md`.
- Local storage redirection is optional. Keep ordinary development independent of
  Scratch and sccache, keep machine-specific settings untracked, and preserve the
  developer's active storage setup unless the task requires changing it.
