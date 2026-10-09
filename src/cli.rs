use std::path::PathBuf;

use clap::Parser;

use crate::requirement::Policy;

/// Cargo runs `cargo upgrade` as `cargo-upgrade upgrade`, so the binary
/// parses its arguments as if it were `cargo`.
#[derive(Debug, Parser)]
#[command(name = "cargo", bin_name = "cargo")]
pub enum Cli {
    Upgrade(Args),
}

/// Upgrade dependency requirements in Cargo.toml
#[derive(Debug, clap::Args)]
#[command(version, about)]
pub struct Args {
    /// Only upgrade these dependencies
    #[arg(value_name = "CRATE")]
    pub crates: Vec<String>,

    /// Upgrade to the latest releases, even when they break the current requirements
    #[arg(short = 'L', long)]
    pub latest: bool,

    /// Show the upgrades without writing Cargo.toml
    #[arg(long)]
    pub dry_run: bool,

    /// Path to Cargo.toml
    #[arg(long, value_name = "PATH", default_value = "Cargo.toml")]
    pub manifest_path: PathBuf,
}

impl Args {
    pub fn policy(&self) -> Policy {
        if self.latest {
            Policy::Latest
        } else {
            Policy::Compatible
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn command_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_as_a_cargo_subcommand() {
        let Cli::Upgrade(args) = Cli::parse_from(["cargo", "upgrade", "-L", "serde"]);
        assert_eq!(args.crates, ["serde"]);
        assert_eq!(args.policy(), Policy::Latest);
    }
}
