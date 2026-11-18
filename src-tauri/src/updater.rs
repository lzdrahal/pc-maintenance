use crate::models::UpdateInfo;
use serde::Deserialize;
use std::time::Duration;

const GITHUB_LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/lzdrahal/pc-maintenance/releases/latest";

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
}

fn parse_semver_tuple(version: &str) -> Option<(u64, u64, u64)> {
    let trimmed = version.trim().trim_start_matches(['v', 'V']);
    let core = trimmed.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

pub fn is_newer_version(current: &str, latest: &str) -> bool {
    match (parse_semver_tuple(current), parse_semver_tuple(latest)) {
        (Some(cur), Some(lat)) => lat > cur,
        _ => false,
    }
}

pub async fn check_latest_release(current_version: &str) -> Option<UpdateInfo> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;

    let response = client
        .get(GITHUB_LATEST_RELEASE_URL)
        .header(
            "User-Agent",
            format!("SystemMaintenanceToolkit/{current_version}"),
        )
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    let release: GitHubRelease = response.json().await.ok()?;
    if !is_newer_version(current_version, &release.tag_name) {
        return None;
    }

    let clean_latest = release.tag_name.trim().trim_start_matches(['v', 'V']).to_string();
    Some(UpdateInfo {
        current_version: current_version.to_string(),
        latest_version: clean_latest,
        release_url: release.html_url,
        release_notes: release.body.filter(|b| !b.trim().is_empty()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_semver_versions_correctly() {
        assert!(is_newer_version("2.0.0", "v2.0.1"));
        assert!(is_newer_version("2.0.0", "2.1.0"));
        assert!(is_newer_version("2.0.0", "V3.0.0"));
        assert!(!is_newer_version("2.0.0", "v2.0.0"));
        assert!(!is_newer_version("2.1.0", "v2.0.9"));
        assert!(!is_newer_version("2.0.0", "1.9.9"));
        assert!(!is_newer_version("2.0.0", "invalid-tag"));
    }
}
