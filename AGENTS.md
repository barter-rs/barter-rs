# Jackbot Sensor Contributor Guide

This repository is a multi-crate Rust workspace for Jackbot Terminal. All code should remain consistent and well-tested.

## Naming
- The project is called **Jackbot**. Avoid references to the old name "Barter" in code or docs.

## Commit Guidelines
- Use short, descriptive commit messages in the present tense.
- Update documentation when you modify or add features.

## Programmatic Checks
Run these commands from the repository root **before committing any code changes**:

1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --workspace`

If your changes only modify documentation (`*.md` files) or comments, running `cargo fmt --all -- --check` is sufficient.

## Additional Conventions
- All public types and functions must be documented.
- Keep `docs/IMPLEMENTATION_STATUS.md` up to date with new features or bug fixes.
- Each exchange module should include tests. If functionality is unsupported, provide a stub with a doc comment explaining why.

## Pull Requests
When creating a PR, provide a concise summary of your changes and include the results of the checks above.
