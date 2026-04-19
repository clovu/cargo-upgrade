use clap::ArgAction;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "cargo-upgrade",
    version,
    about = "Refresh dependency requirements in Cargo.toml.",
    long_about = "Refresh dependency requirements in Cargo.toml.\n\nThe current implementation checks crates.io, scans [dependencies], [dev-dependencies], and [build-dependencies], and rewrites dependency requirements in place when updates are available."
)]
pub struct Cli {
    #[arg(
        short = 'r',
        long,
        hide = true,
        help = "Run recursively across workspace packages"
    )]
    pub recursive: bool,

    #[arg(
        short = 'L',
        long,
        help = "Ignore the current requirement and use the latest available release"
    )]
    pub latest: bool,

    #[arg(
        short = 'g',
        long,
        hide = true,
        help = "Upgrade globally installed crates"
    )]
    pub global: bool,

    #[arg(long, hide = true, help = "Prefer workspace packages when available")]
    pub workspace: bool,

    #[arg(
        short = 'P',
        long,
        hide = true,
        conflicts_with = "dev",
        help = "Only upgrade production dependencies"
    )]
    pub prod: bool,

    #[arg(
        short = 'D',
        long,
        hide = true,
        conflicts_with = "prod",
        help = "Only upgrade development dependencies"
    )]
    pub dev: bool,

    #[arg(long, hide = true, help = "Skip optional dependencies")]
    pub no_optional: bool,

    #[arg(short = 'i', long, hide = true, help = "Choose upgrades interactively")]
    pub interactive: bool,

    #[arg(long, help = "Print the planned changes without modifying Cargo.toml")]
    pub dry_run: bool,

    #[arg(
        long,
        hide = true,
        value_name = "FILTER",
        action = ArgAction::Append,
        help = "Filter target workspace packages (repeatable)"
    )]
    pub filter: Vec<String>,

    #[arg(
        long,
        hide = true,
        value_name = "N",
        help = "Set recursion depth for package traversal"
    )]
    pub depth: Option<u32>,

    #[arg(
        hide = true,
        value_name = "PACKAGE",
        help = "Only upgrade selected packages"
    )]
    pub crates: Vec<String>,
}
