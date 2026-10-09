//! Version requirements and the semver rules for moving them forward.

use std::fmt;

use semver::{BuildMetadata, Comparator, Op, Prerelease, Version, VersionReq};

/// How far an upgrade may move a requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// Stay within the range the current requirement already accepts.
    Compatible,
    /// Move to the newest stable release, even across breaking versions.
    Latest,
}

/// A requirement made of a single `^`, `~` or `=` comparator, such as `1.2`,
/// `^1.2.3`, `~0.4` or `=2.0.0`.
///
/// Upgrades keep the operator and precision the author wrote, so `1.44`
/// becomes `1.52` and `~1.2.0` becomes `~1.2.9`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    comparator: Comparator,
    /// Cargo reads `1.2` and `^1.2` alike; remember which one was written.
    explicit_caret: bool,
}

impl Requirement {
    /// Parses a requirement, or returns `None` for forms this tool leaves
    /// alone: ranges, wildcards and `*`.
    pub fn parse(text: &str) -> Option<Self> {
        let req = VersionReq::parse(text).ok()?;
        let [comparator] = req.comparators.as_slice() else {
            return None;
        };
        matches!(comparator.op, Op::Caret | Op::Tilde | Op::Exact).then(|| Self {
            comparator: comparator.clone(),
            explicit_caret: text.trim_start().starts_with('^'),
        })
    }

    /// Moves the requirement to the newest release `policy` permits, or
    /// returns `None` when that would not change it.
    pub fn upgrade(&self, releases: &[Version], policy: Policy) -> Option<Self> {
        let newest = releases.iter().filter(|v| self.permits(v, policy)).max()?;
        let upgraded = self.retarget(newest);
        (upgraded.floor() > self.floor()).then_some(upgraded)
    }

    /// Whether `other` lies outside the range this requirement accepts.
    pub fn is_breaking(&self, other: &Self) -> bool {
        !self.comparator.matches(&other.floor())
    }

    fn permits(&self, version: &Version, policy: Policy) -> bool {
        match policy {
            Policy::Compatible => self.comparator.matches(version),
            // Pre-releases are only offered to those already tracking one.
            Policy::Latest => version.pre.is_empty() || !self.comparator.pre.is_empty(),
        }
    }

    /// The same requirement pointed at `version`, at the same precision.
    fn retarget(&self, version: &Version) -> Self {
        let Comparator {
            op, minor, patch, ..
        } = self.comparator;
        let comparator = Comparator {
            op,
            major: version.major,
            minor: minor.map(|_| version.minor),
            patch: patch.map(|_| version.patch),
            pre: match patch {
                Some(_) => version.pre.clone(),
                None => Prerelease::EMPTY,
            },
        };
        Self {
            comparator,
            explicit_caret: self.explicit_caret,
        }
    }

    /// The lowest version the requirement accepts.
    fn floor(&self) -> Version {
        let c = &self.comparator;
        Version {
            major: c.major,
            minor: c.minor.unwrap_or(0),
            patch: c.patch.unwrap_or(0),
            pre: c.pre.clone(),
            build: BuildMetadata::EMPTY,
        }
    }
}

impl fmt::Display for Requirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = &self.comparator;
        let op = match c.op {
            Op::Caret if self.explicit_caret => "^",
            Op::Tilde => "~",
            Op::Exact => "=",
            _ => "",
        };
        write!(f, "{op}{}", c.major)?;
        if let Some(minor) = c.minor {
            write!(f, ".{minor}")?;
        }
        if let Some(patch) = c.patch {
            write!(f, ".{patch}")?;
        }
        if !c.pre.is_empty() {
            write!(f, "-{}", c.pre)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RELEASES: &[&str] = &[
        "0.9.0",
        "1.0.0",
        "1.2.0",
        "1.2.9",
        "1.4.1",
        "2.0.0",
        "2.1.3",
        "3.0.0-beta.1",
    ];

    fn upgrade(requirement: &str, policy: Policy) -> Option<String> {
        let releases: Vec<_> = RELEASES.iter().map(|v| v.parse().unwrap()).collect();
        Requirement::parse(requirement)
            .expect("supported requirement")
            .upgrade(&releases, policy)
            .map(|r| r.to_string())
    }

    #[test]
    fn compatible_stays_within_the_requirement() {
        assert_eq!(
            upgrade("1.0.0", Policy::Compatible).as_deref(),
            Some("1.4.1")
        );
        assert_eq!(
            upgrade("^1.0.0", Policy::Compatible).as_deref(),
            Some("^1.4.1")
        );
        assert_eq!(
            upgrade("~1.2.0", Policy::Compatible).as_deref(),
            Some("~1.2.9")
        );
        assert_eq!(upgrade("0.9", Policy::Compatible), None);
    }

    #[test]
    fn compatible_never_moves_exact_pins() {
        assert_eq!(upgrade("=1.2.0", Policy::Compatible), None);
        assert_eq!(upgrade("=1.2", Policy::Compatible), None);
    }

    #[test]
    fn latest_crosses_breaking_versions() {
        assert_eq!(upgrade("1.0.0", Policy::Latest).as_deref(), Some("2.1.3"));
        assert_eq!(upgrade("~1.2.0", Policy::Latest).as_deref(), Some("~2.1.3"));
        assert_eq!(upgrade("=1.2.0", Policy::Latest).as_deref(), Some("=2.1.3"));
    }

    #[test]
    fn precision_is_preserved() {
        assert_eq!(upgrade("1", Policy::Compatible), None);
        assert_eq!(upgrade("1.0", Policy::Compatible).as_deref(), Some("1.4"));
        assert_eq!(upgrade("1", Policy::Latest).as_deref(), Some("2"));
    }

    #[test]
    fn pre_releases_are_opt_in() {
        assert_eq!(upgrade("2.1.3", Policy::Latest), None);
        assert_eq!(
            upgrade("3.0.0-alpha.1", Policy::Latest).as_deref(),
            Some("3.0.0-beta.1")
        );
    }

    #[test]
    fn never_downgrades() {
        assert_eq!(upgrade("5.0.0", Policy::Latest), None);
    }

    #[test]
    fn build_metadata_is_dropped() {
        let releases = ["0.25.17+spec-1.1.0".parse().unwrap()];
        let upgraded = Requirement::parse("0.25.12+spec-1.1.0")
            .and_then(|r| r.upgrade(&releases, Policy::Compatible))
            .unwrap();
        assert_eq!(upgraded.to_string(), "0.25.17");
    }

    #[test]
    fn ranges_and_wildcards_are_unsupported() {
        for text in ["*", "1.*", ">=1, <2", "<2", "not a version"] {
            assert_eq!(Requirement::parse(text), None, "{text}");
        }
    }

    #[test]
    fn breaking_means_leaving_the_current_range() {
        let parse = |text| Requirement::parse(text).unwrap();
        assert!(!parse("1.2.0").is_breaking(&parse("1.4.1")));
        assert!(parse("1.2.0").is_breaking(&parse("2.0.0")));
        assert!(parse("0.2.0").is_breaking(&parse("0.3.0")));
        assert!(parse("~1.2.0").is_breaking(&parse("~1.4.1")));
    }
}
