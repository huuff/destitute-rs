# destitute

Rust derive macro: `#[derive(Destitute)]` on a struct `S` generates `DestituteS`
(every field `Option<T>` unless `#[destitute(mandatory)]` or already an `Option`), `S::into_destitute`,
`DestituteS::try_undestitute -> Result<S, MissingField>`, matching `From`/`TryFrom`
impls, and a `MaybeDestituteS { Complete, Destitute }` enum (opt out with `no_enum`).
Full attribute reference lives in the crate docs at `destitute/src/lib.rs`.

Workspace: `destitute` (public facade, `no_std`, owns `MissingField`) and
`destitute-derive` (proc-macro; keep all logic in `expand()` so it stays unit-testable).

- Dev shell: `devenv shell` (or `direnv allow`). Check: `devenv test` runs `cargo test --workspace --all-features`.
- The `serde` feature is opt-in per struct via `#[destitute(serde)]`; its tests live in `destitute/tests/serde.rs`.
- Behaviour tests go in `destitute/tests/derive.rs`; error-message tests in the derive crate's `mod tests`.
- Commits follow Conventional Commits; git hooks (clippy, rustfmt, ripsecrets, ...) are installed on shell entry. Never bypass with `--no-verify`.
- devenv `scripts.*` with real logic: use nushell (`package = pkgs.nushell`); CLI args need `def --wrapped main [...args]`, interpolation is `$"(...)"`.
