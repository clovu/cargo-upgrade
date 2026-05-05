use clap::ArgAction;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "cargo-upgrade",
    bin_name = "cargo upgrade",
    version,
    about = "Upgrade dependency version requirements in Cargo.toml",
    long_about = "Upgrade dependency version requirements in Cargo.toml.\n\nBy default, cargo-upgrade selects the newest crates.io release that still satisfies each existing requirement. Use --latest to allow upgrades outside the current requirement range.\n\nThe current implementation scans the top-level [dependencies], [dev-dependencies], and [build-dependencies] tables and rewrites only dependency version requirements. It does not run cargo update or edit Cargo.lock.",
    after_long_help = "Examples:\n  cargo upgrade --dry-run\n  cargo upgrade --latest serde tokio\n  cargo upgrade --interactive\n  cargo upgrade --prod\n  cargo upgrade --dev"
)]
pub struct Cli {
    #[arg(
        short = 'r',
        long,
        hide = true,
        help = "Traverse workspace member manifests recursively"
    )]
    pub recursive: bool,

    #[arg(
        short = 'L',
        long,
        help = "Allow upgrades outside the current version requirement",
        long_help = "Allow upgrades outside the current version requirement.\n\nWithout this flag, cargo-upgrade chooses the newest release that still satisfies the requirement already written in Cargo.toml. With --latest, it targets the newest release available from crates.io while preserving the visible requirement style where possible."
    )]
    pub latest: bool,

    #[arg(
        short = 'g',
        long,
        hide = true,
        help = "Upgrade crates installed with cargo install"
    )]
    pub global: bool,

    #[arg(long, hide = true, help = "Use workspace-aware dependency resolution")]
    pub workspace: bool,

    #[arg(
        short = 'P',
        long,
        conflicts_with = "dev",
        help = "Limit upgrades to normal and build dependencies",
        long_help = "Limit upgrades to normal and build dependencies.\n\nToday this scans top-level [dependencies] and [build-dependencies]. Target-specific production dependency tables are planned."
    )]
    pub prod: bool,

    #[arg(
        short = 'D',
        long,
        conflicts_with = "prod",
        help = "Limit upgrades to development dependencies",
        long_help = "Limit upgrades to development dependencies.\n\nToday this scans top-level [dev-dependencies]. Target-specific development dependency tables are planned."
    )]
    pub dev: bool,

    #[arg(long, hide = true, help = "Skip dependencies marked optional = true")]
    pub no_optional: bool,

    #[arg(
        short = 'i',
        long,
        help = "Review and select upgrades in a terminal UI"
    )]
    pub interactive: bool,

    #[arg(long, help = "Show planned edits without writing Cargo.toml")]
    pub dry_run: bool,

    #[arg(
        long,
        hide = true,
        value_name = "FILTER",
        action = ArgAction::Append,
        help = "Filter workspace members to upgrade"
    )]
    pub filter: Vec<String>,

    #[arg(
        long,
        hide = true,
        value_name = "N",
        help = "Limit recursive workspace traversal depth"
    )]
    pub depth: Option<u32>,

    #[arg(
        value_name = "DEPENDENCY",
        help = "Dependency names to upgrade",
        long_help = "Dependency names to upgrade.\n\nNames are matched exactly against dependency keys in Cargo.toml, such as serde, tokio, or clap. Patterns and globs are not implemented yet."
    )]
    pub crates: Vec<String>,
}
