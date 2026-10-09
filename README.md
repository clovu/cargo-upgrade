# cargo-upgrade

`cargo-upgrade` is a Cargo subcommand for refreshing dependency version requirements in `Cargo.toml`.

It is inspired by the ergonomics of `pnpm upgrade`: inspect the dependencies in a project, resolve newer crate releases from crates.io, then rewrite manifest requirements when an upgrade is available.

The tool changes dependency requirements in `Cargo.toml`. It is not a wrapper around `cargo update`, and it is not limited to updating `Cargo.lock`.

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

After installation, Cargo can run the binary as a subcommand:

```bash
cargo upgrade
```

## Quick Start

```bash
cargo upgrade                 # upgrade within current semver requirements
cargo upgrade --dry-run       # show the plan, leave Cargo.toml untouched
cargo upgrade --latest        # also take breaking releases
cargo upgrade serde tokio     # only these dependencies
```

## What It Does

1. Reads `Cargo.toml` (or `--manifest-path`).
2. Collects crates.io dependencies from `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, and their `[target.<platform>]` variants.
3. Looks up releases in the crates.io sparse index.
4. Rewrites only the version requirement, preserving comments, formatting, and other fields such as `features`.

Dependencies declared with `workspace`, `path`, `git`, or `registry` are skipped, as are requirements that are ranges (`>=1, <2`), wildcards (`1.*`), or `*`. Renamed dependencies (`package = "..."`) are looked up by their real crate name.

## Version Requirement Behavior

By default, an upgrade follows semver: the requirement moves to the newest release it already accepts, so nothing breaking is introduced.

| Requirement | Default | `--latest` |
|---|---|---|
| `1.0.100` | `1.0.229` | `1.0.229` |
| `0.22.0` | `0.22.27` | `0.25.17` |
| `~1.38.0` | `~1.38.2` | `~1.53.2` |
| `=4.5.0` | unchanged | `=4.6.7` |

The operator and precision you wrote are kept: `1.44` becomes `1.52`, not `1.52.0`. Build metadata is dropped. Pre-releases are only offered when the current requirement is already a pre-release. Requirements never move backwards. With `--latest`, upgrades that leave the current range are marked `(breaking)`.

## Command Options

| Option | Description |
|---|---|
| `[CRATE]...` | Only upgrade these dependencies, matched by manifest key. |
| `-L`, `--latest` | Target the newest stable release, ignoring the current range. |
| `--dry-run` | Print the plan without writing `Cargo.toml`. |
| `--manifest-path <PATH>` | Path to `Cargo.toml`. Defaults to `./Cargo.toml`. |
| `-h`, `--help` | Print help. |
| `-V`, `--version` | Print the version. |

A crate that cannot be looked up is reported as a warning and does not stop the run.

## Project Direction

The goal is to make upgrading Rust dependency requirements feel as natural as `pnpm upgrade` does in JavaScript projects, while respecting Cargo manifest structure and Rust workflow expectations.

Long-term areas include:

- Workspace traversal.
- Workspace-aware filtering.
- Global Cargo binary crate upgrades.
- Optional dependency filtering.
- Better source handling for non-crates.io dependencies.
- Interactive review.

## License

MIT License © 2026 [Clover You](https://github.com/clovu)
