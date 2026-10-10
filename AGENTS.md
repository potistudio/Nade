# Repository Guidelines

## Project Structure & Module Organization
This repository is a Cargo workspace. Workspace settings live in `Cargo.toml`, with shared formatting rules in `rustfmt.toml` and `.editorconfig`.
- Core crates: `crates/*` (for example, `crates/core/src/lib.rs`).
- Main binary: `crates/nade/src/main.rs`.
- Runnable examples: `examples/*` (for example, `examples/gizmos`).
- Assets: `assets/`.
- Build output: `target/` (do not edit or commit).

Prefer adding reusable logic to a library crate under `crates/` and keeping binaries thin.

## Build, Test, and Development Commands
Run all commands from the repository root.
- `cargo check --workspace`: Fast type-check across all crates.
- `cargo build --workspace`: Build the entire workspace.
- `cargo run -p nade`: Run the main application crate.
- `cargo run -p gizmos`: Run the `gizmos` example.
- `cargo test --workspace`: Run all tests.
- `cargo fmt --all`: Format code using workspace rules.
- `cargo clippy --workspace --all-targets -D warnings`: Lint strictly and fail on warnings.

## Coding Style & Naming Conventions
Formatting is enforced by `rustfmt`.
- Indentation: tabs (`hard_tabs = true`, tab width 4).
- Names: modules/functions in `snake_case`, types/traits in `CamelCase`, constants in `SCREAMING_SNAKE_CASE`.
- Prefer small, explicit functions over clever abstractions.

Always run `cargo fmt --all` and `cargo clippy --workspace --all-targets -D warnings` before submitting changes.

## Testing Guidelines
Use Rust’s standard testing patterns.
- Unit tests: inline under `#[cfg(test)] mod tests { ... }`.
- Integration tests: `crates/<name>/tests/` when needed.
- Test names should describe behavior, for example: `fn draws_glyph_outline()`.

Run tests with `cargo test --workspace`.

## Commit & Pull Request Guidelines
Commit history follows Conventional Commits.
- Examples: `feat: add glyph cache`, `fix(core): handle empty atlas`.
- Keep commits small, focused, and cohesive.

Pull requests should include:
- A short description and motivation.
- Linked issues or TODO references when relevant.
- Reproduction commands (for example, `cargo run -p nade`).
- Screenshots or short recordings for UI/graphics changes.

## Agent-Specific Instructions
When making changes as an automated agent:
- Do not revert user changes you did not make.
- Avoid editing `target/`.
- Follow this guide and existing local conventions first.
