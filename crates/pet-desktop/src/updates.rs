//! Explicit, read-only release checks. Network work never blocks the UI thread.
use semver::Version;
use std::time::Duration;

pub const PROJECT_URL: &str = "https://github.com/MajoIreina/dragon-bubble";
pub const RELEASES_URL: &str = "https://github.com/MajoIreina/dragon-bubble/releases";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Current,
    Available(String),
    Unpublished,
    RateLimited,
    Failed,
}

fn classify_release(body: &str, current: &str) -> Option<Outcome> {
    let release: serde_json::Value = serde_json::from_str(body).ok()?;
    if release.get("draft")?.as_bool()? || release.get("prerelease")?.as_bool()? {
        return None;
    }
    let tag = release.get("tag_name")?.as_str()?.trim();
    let latest = Version::parse(tag.strip_prefix('v').unwrap_or(tag)).ok()?;
    let current = Version::parse(current).ok()?;
    Some(if latest > current {
        Outcome::Available(latest.to_string())
    } else {
        Outcome::Current
    })
}

pub fn check() -> Outcome {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into();
    match agent
        .get("https://api.github.com/repos/MajoIreina/dragon-bubble/releases/latest")
        .header(
            "User-Agent",
            concat!("DragonBubble/", env!("CARGO_PKG_VERSION")),
        )
        .header("Accept", "application/vnd.github+json")
        .call()
    {
        Ok(mut response) => response
            .body_mut()
            .with_config()
            .limit(256 * 1024)
            .read_to_string()
            .ok()
            .and_then(|body| classify_release(&body, VERSION))
            .unwrap_or(Outcome::Failed),
        Err(ureq::Error::StatusCode(404)) => Outcome::Unpublished,
        Err(ureq::Error::StatusCode(403 | 429)) => Outcome::RateLimited,
        Err(_) => Outcome::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compares_release_versions_numerically_and_handles_invalid_metadata() {
        let release = |tag: &str| {
            serde_json::json!({"tag_name": tag, "draft": false, "prerelease": false}).to_string()
        };
        assert_eq!(
            classify_release(&release("v0.10.0"), "0.9.0"),
            Some(Outcome::Available("0.10.0".into()))
        );
        assert_eq!(
            classify_release(&release("v0.1.0"), VERSION),
            Some(Outcome::Current)
        );
        assert_eq!(
            classify_release(&release("0.0.9"), VERSION),
            Some(Outcome::Current)
        );
        assert_eq!(classify_release(&release("bad tag"), VERSION), None);
        assert_eq!(classify_release("not json", VERSION), None);
        assert_eq!(
            classify_release(
                r#"{"tag_name":"v1.0.0","draft":true,"prerelease":false}"#,
                VERSION
            ),
            None
        );
        assert_eq!(
            classify_release(
                r#"{"tag_name":"v1.0.0-beta.1","draft":false,"prerelease":true}"#,
                VERSION
            ),
            None
        );
    }
}
