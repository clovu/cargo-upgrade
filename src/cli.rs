use clap::ArgAction;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "cargo-upgrade",
    version,
    about = "Updates packages to their latest version based on the specified range. You can use \"*\" in package name to update all packages with the same pattern.",
    long_about = None
)]
pub struct Cli {
    #[arg(short = 'r', long, help = "Run recursively across workspace packages")]
    pub recursive: bool,

    #[arg(
        short = 'L',
        long,
        help = "Upgrade by ignoring current version requirements"
    )]
    pub latest: bool,

    #[arg(short = 'g', long, help = "Upgrade globally installed crates")]
    pub global: bool,

    #[arg(long, help = "Prefer workspace packages when available")]
    pub workspace: bool,

    #[arg(
        short = 'P',
        long,
        conflicts_with = "dev",
        help = "Only upgrade production dependencies"
    )]
    pub prod: bool,

    #[arg(
        short = 'D',
        long,
        conflicts_with = "prod",
        help = "Only upgrade development dependencies"
    )]
    pub dev: bool,

    #[arg(long, help = "Skip optional dependencies")]
    pub no_optional: bool,

    #[arg(short = 'i', long, help = "Choose upgrades interactively")]
    pub interactive: bool,

    #[arg(long, help = "Do not write updated requirements to Cargo.toml")]
    pub no_save: bool,

    #[arg(
        long,
        value_name = "FILTER",
        action = ArgAction::Append,
        help = "Filter target workspace packages (repeatable)"
    )]
    pub filter: Vec<String>,

    #[arg(
        long,
        value_name = "N",
        help = "Set recursion depth for package traversal"
    )]
    pub depth: Option<u32>,

    #[arg(value_name = "PACKAGE", help = "Only upgrade selected packages")]
    pub packages: Vec<String>,
}
