# cargo-upgrade

`cargo-upgrade` is a Cargo subcommand inspired by `pnpm upgrade`, designed for Rust manifests and Cargo workflows.

It refreshes dependency requirements in `Cargo.toml` by checking crates.io, selecting newer releases, and rewriting the manifest when needed.

## Installation

```bash
cargo install cargo-upgrade
```

Or install from source:

```bash
git clone https://github.com/clovu/cargo-upgrade.git
cd cargo-upgrade
cargo install --path .
```

## Usage

```bash
cargo upgrade
cargo upgrade --latest
cargo upgrade --dry-run
```

## Current behavior

The current implementation:

1. Reads `Cargo.toml` from the current directory.
2. Scans these dependency sections:
   - `dependencies`
   - `dev-dependencies`
   - `build-dependencies`
3. Queries crates.io for available releases.
4. Chooses the newest compatible release for each dependency by default.
5. Rewrites the dependency requirement in `Cargo.toml`.

With `--latest`, the command ignores the current requirement and rewrites each dependency to the latest available release.

With `--dry-run`, the command prints the planned changes without modifying `Cargo.toml`.

## Command design

This project aims to make `cargo upgrade` feel like `pnpm upgrade`, while still fitting Cargo manifest structure and Rust workflow expectations.

The codebase is organized around that shape: command flow, manifest ownership, release resolution, and requirement-rewrite policy.

## Requirement behavior

By default, `cargo-upgrade` keeps the style of the current requirement while selecting the newest compatible release it can use.

Examples:

- `serde = "1.0"` is refreshed within the current requirement logic.
- `tokio = "~1.0"` keeps the `~` operator.
- `clap = "=4.5.0"` keeps the `=` operator.
- `foo = "*"` becomes a concrete caret requirement for the selected release.

## Manifest behavior

`cargo-upgrade` rewrites `Cargo.toml` only when changes are needed.

It preserves both of these forms:

```toml
serde = "1.0.219"
```

```toml
tokio = { version = "1.44.2", features = ["rt-multi-thread", "macros"] }
```

For inline-table dependencies, only the `version` field is rewritten.

## Current limitations

The current implementation does not yet cover the full long-term command vision.

Notable limitations today:

- It must be run from a crate root containing `Cargo.toml`.
- Inherited dependencies are skipped.
- Dependencies without an explicit version field are skipped.
- crates.io lookup failures are reported, but do not stop the whole run.
- Workspace-recursive flows, interactive upgrades, filtering, and package targeting are not implemented yet.

## Product direction

The long-term goal is a `cargo upgrade` experience that feels as natural in Rust projects as `pnpm upgrade` does in JavaScript projects.

## License

MIT License © 2026 [Clover You](https://github.com/clovu)
