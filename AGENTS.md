# Repository Guidelines

## Project Structure & Module Organization
This is a Cargo workspace. Top-level configuration lives in `Cargo.toml` and shared formatting rules in `rustfmt.toml` and `.editorconfig`.
- Workspace crates: `crates/*` (e.g., `crates/nade/src/main.rs`, `crates/core/src/lib.rs`).
- Runnable examples: `examples/*` (e.g., `examples/gizmos`, `examples/playground_font`).
- Assets: `assets/`.
- Build output: `target/` (do not edit or commit).

Prefer adding new reusable logic to a library crate under `crates/` and keeping binaries thin.

## Build, Test, and Development Commands
Run commands from the repository root.
- `cargo build --workspace`: Build all crates and examples.
- `cargo check --workspace`: Fast type-check across the workspace.
- `cargo run -p nade`: Run the main application crate.
- `cargo run -p gizmos`: Run the `gizmos` example.
- `cargo test --workspace`: Run all tests (add tests alongside code in each crate).
- `cargo fmt --all`: Format code using workspace rules.
- `cargo clippy --workspace --all-targets -D warnings`: Lint strictly and fail on warnings.

## Coding Style & Naming Conventions
Formatting is enforced via `rustfmt` and `.editorconfig`.
- Indentation: tabs (`hard_tabs = true`, tab width 4).
- Always run `cargo fmt --all` before submitting changes.
- Names: crates/modules/functions in `snake_case`, types/traits in `CamelCase`, constants in `SCREAMING_SNAKE_CASE`.
- Keep modules focused; prefer small, explicit functions over clever abstractions.

## Testing Guidelines
There is no dedicated top-level `tests/` directory today. Use Rust’s standard patterns:
- Unit tests in the same file under `#[cfg(test)] mod tests { ... }`.
- Integration tests in `crates/<name>/tests/` when needed.
- Test names should describe behavior, e.g., `fn draws_glyph_outline()`.
- Run: `cargo test --workspace`.

## Commit & Pull Request Guidelines
Commit history follows Conventional Commits.
- Format: `feat: ...`, `fix: ...`, `refactor: ...`, `chore(deps): ...`, `feat(example): ...`.
- Use scopes when helpful: `feat(core): add glyph cache`.
- Keep commits small and cohesive.

Pull requests should include:
- A short description of the change and motivation.
- Linked issues or TODO references when relevant.
- Reproduction steps or commands (e.g., `cargo run -p nade`).
- Screenshots or short recordings for UI/graphics changes.
