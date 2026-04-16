use crate::error::Result;

pub(crate) fn select_target_version(
    current_version: &str,
    mut versions: Vec<String>,
) -> Result<Option<String>> {
    let current_version_req = current_version.parse::<semver::VersionReq>()?;
    if current_version_req.comparators.is_empty() {
        return Ok(None);
    }

    versions.sort_by(|a, b| {
        let version_a = semver::Version::parse(a).ok();
        let version_b = semver::Version::parse(b).ok();
        version_a.cmp(&version_b)
    });

    let Some(matching_version) = versions.into_iter().rfind(|version_num| {
        let Ok(version) = version_num.parse::<semver::Version>() else {
            return false;
        };
        let Ok(version_req) = version.to_string().parse::<semver::VersionReq>() else {
            return false;
        };

        if current_version_req.eq(&version_req) {
            return false;
        }

        current_version_req.matches(&version)
    }) else {
        return Ok(None);
    };

    Ok(Some(format_requirement(current_version, &matching_version)))
}

fn format_requirement(current_version: &str, matching_version: &str) -> String {
    let trimmed = current_version.trim();

    if trimmed == "*" {
        return format!("^{matching_version}");
    }

    if let Some(prefix) = trimmed
        .chars()
        .next()
        .filter(|c| matches!(c, '~' | '^' | '='))
    {
        return format!("{prefix}{matching_version}");
    }

    matching_version.to_string()
}
