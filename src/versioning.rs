use crate::error::Result;
use semver::Version;
use semver::VersionReq;

pub(crate) fn choose_target_release(
    requirement: &str,
    mut releases: Vec<Version>,
    use_latest: bool,
) -> Result<Option<Version>> {
    releases.sort();

    if use_latest {
        return Ok(releases
            .into_iter()
            .rfind(|release| rewrite_requirement(requirement, release) != requirement.trim()));
    }

    let version_req = requirement.parse::<VersionReq>()?;
    if version_req.comparators.is_empty() {
        return Ok(None);
    }

    Ok(releases.into_iter().rfind(|release| {
        let Ok(release_req) = release.to_string().parse::<VersionReq>() else {
            return false;
        };

        if version_req == release_req {
            return false;
        }

        version_req.matches(release)
    }))
}

pub(crate) fn rewrite_requirement(current_requirement: &str, release: &Version) -> String {
    let trimmed = current_requirement.trim();

    if trimmed == "*" {
        return format!("^{release}");
    }

    if let Some(prefix) = trimmed
        .chars()
        .next()
        .filter(|character| matches!(character, '~' | '^' | '='))
    {
        return format!("{prefix}{release}");
    }

    release.to_string()
}

#[cfg(test)]
mod tests {
    use super::choose_target_release;
    use super::rewrite_requirement;
    use semver::Version;

    #[test]
    fn chooses_latest_compatible_release_by_default() {
        let release = choose_target_release(
            "~1.2.0",
            vec![
                Version::parse("1.2.1").unwrap(),
                Version::parse("1.2.9").unwrap(),
                Version::parse("1.3.0").unwrap(),
            ],
            false,
        )
        .unwrap();

        assert_eq!(release, Some(Version::parse("1.2.9").unwrap()));
    }

    #[test]
    fn chooses_latest_release_when_requested() {
        let release = choose_target_release(
            "~1.2.0",
            vec![
                Version::parse("1.2.1").unwrap(),
                Version::parse("1.2.9").unwrap(),
                Version::parse("2.0.0").unwrap(),
            ],
            true,
        )
        .unwrap();

        assert_eq!(release, Some(Version::parse("2.0.0").unwrap()));
    }

    #[test]
    fn preserves_requirement_style_when_rewriting() {
        assert_eq!(
            rewrite_requirement("~1.2.0", &Version::parse("1.2.9").unwrap()),
            "~1.2.9"
        );
        assert_eq!(
            rewrite_requirement("*", &Version::parse("3.1.4").unwrap()),
            "^3.1.4"
        );
    }
}
