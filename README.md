# cargo-upgrade

`cargo-upgrade` is a Cargo plugin-style tool for upgrading Rust dependencies, with a CLI surface inspired by `pnpm upgrade`.

## Installation

```bash
cargo install cargo-upgrade
```

OR

```bash
git clone https://github.com/clovu/cargo-upgrade.git
cd cargo-upgrade
cargo install --path .
```

## Usage

```bash
cargo upgrade # or cargo upgrade --latest
```

## Options

Status legend:
- **Implemented**: has runtime effect now
- **Defined (not implemented)**: parsed by CLI but not wired to runtime yet

| Option | Description | Status |
|---|---|---|
| `[PACKAGE]...` | Upgrade only selected packages | Defined (not implemented) |
| `-r, --recursive` | Run recursively across workspace packages | Defined (not implemented) |
| `-L, --latest` | Ignore current version requirements | Defined (not implemented) |
| `-g, --global` | Upgrade globally installed crates | Defined (not implemented) |
| `--workspace` | Prefer workspace packages when available | Defined (not implemented) |
| `-P, --prod` | Only upgrade production dependencies | Defined (not implemented) |
| `-D, --dev` | Only upgrade development dependencies | Defined (not implemented) |
| `--no-optional` | Skip optional dependencies | Defined (not implemented) |
| `-i, --interactive` | Choose upgrades interactively | Defined (not implemented) |
| `--no-save` | Do not write updated requirements to manifest | Defined (not implemented) |
| `--filter <FILTER>` | Filter target workspace packages (repeatable) | Defined (not implemented) |
| `--depth <N>` | Set recursion depth for package traversal | Defined (not implemented) |

## Behavior

Current runtime behavior:

1. Load manifest file.
2. Collect dependencies from:
   - `dependencies`
   - `dev-dependencies`
   - `build-dependencies`
3. Query crates.io for available versions.
4. Choose the highest version that still matches the current semver requirement.
5. Write updated requirements back to the manifest.

## License

MIT License © 2026 [Clover You](https://github.com/clovu)
