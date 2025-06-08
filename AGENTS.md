# Codex Agent Instructions for `barter-rs`

Barter is an ecosystem of Rust libraries for building high-performance live, paper and back testing trading systems. The project is organized as a unified Cargo workspace with several core crates:
**barter**: the main trading engine
**barter-instrument**: types and utilities for financial instruments and assets
**barter-data**: real-time market data streaming (WebSocket)
**barter-execution**: trade execution, order and account handling
**barter-integration**: REST/WebSocket adapters for exchange connectivity
**barter-macro**: procedural macros for code generation and ergonomics

See the [`Barter`](https://crates.io/crates/barter) family on crates.io for comprehensive documentation.

## Contributor Guide
### Dev Environment
* Install Rust with [rustup](https://rustup.rs/). The project targets the 2024 edition and works on nightly or the latest stable release.
* Clone and build:
  ```sh
  git clone <repo_url>
  cd barter-rs
  cargo build --workspace
  ```
* Format & lint:
  ```sh
  cargo fmt --all
  cargo check
  ```
* Run examples:
  ```sh
  cargo run --example <example_name>
  ```

### Testing
* Run the full test suite:
  ```sh
  cargo test --workspace
  ```
### Pull Requests
* **Title:** `[crate] <Short Description>` (e.g. `[barter-data] Add Bybit stream support`)
* **Description:** Summarise the problem, solution and link relevant issues. Include examples for new features.
* **Format:** ensure `cargo fmt` pass with no warnings and public APIs are documented with `///` comments.
* **Tests:** add or extend unit/integration tests. All tests must pass.
* **Acceptance:** PRs are merged once CI passes and the code follows project style.

## Additional Notes
* For details on AGENTS.md behaviour see <https://platform.openai.com/docs/codex/overview#using-agents-md>.