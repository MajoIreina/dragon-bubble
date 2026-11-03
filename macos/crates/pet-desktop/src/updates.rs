//! Explicit, read-only release checks. Network work never blocks the UI thread.
use semver::Version;
use std::time::Duration;

pub const PROJECT_URL: &str = "https://github.com/MajoIreina/dragon-bubble";
pub const RELEASES_URL: &str = "https://github.com/MajoIreina/dragon-bubble/releases";
/// Public release label. Cargo uses a SemVer prerelease internally; the
/// project publishes the compact patch suffix used by its GitHub tag.
pub const VERSION: &str = "0.1.2a";
#[derive(Debug, PartialEq)]
pub struct Release {
    pub version: String,
    pub url: String,
    pub notes: String,
    pub download: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Current,
    Available(Release),
    Unpublished,
    RateLimited,
    Failed,
}

#[derive(Debug, PartialEq, Eq)]
struct ComparableVersion {
    base: Version,
    suffix: Option<String>,
}

impl ComparableVersion {
    fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim().strip_prefix(['v', 'V']).unwrap_or(raw.trim());
        if let Ok(base) = Version::parse(raw) {
            return Some(Self { base, suffix: None });
        }
        let split = raw.find(|ch: char| !ch.is_ascii_digit() && ch != '.')?;
        let (numeric, suffix) = raw.split_at(split);
        if suffix.is_empty() || !suffix.chars().all(|ch| ch.is_ascii_alphanumeric()) {
            return None;
        }
        let base = Version::parse(numeric).ok()?;
        Some(Self {
            base,
            suffix: Some(suffix.to_ascii_lowercase()),
        })
    }

    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.base
            .cmp(&other.base)
            .then_with(|| match (&self.suffix, &other.suffix) {
                (None, None) => std::cmp::Ordering::Equal,
                (None, Some(_)) => std::cmp::Ordering::Less,
                (Some(_), None) => std::cmp::Ordering::Greater,
                (Some(a), Some(b)) => a.cmp(b),
            })
    }
}

fn compatible_asset(name: &str, arch: &str) -> bool {
    let mac = name.contains("macos") || name.contains("mac-os") || name.contains("darwin");
    let package = name.ends_with(".zip") || name.ends_with(".dmg");
    let architecture = name.contains("universal")
        || match arch {
            "aarch64" => {
                name.contains("aarch64") || name.contains("arm64") || name.contains("apple-silicon")
            }
            "x86_64" => name.contains("x86_64") || name.contains("intel") || name.contains("x64"),
            _ => false,
        };
    mac && package && architecture
}

fn classify_release(body: &str, current: &str) -> Option<Outcome> {
    let release: serde_json::Value = serde_json::from_str(body).ok()?;
    if release.get("draft")?.as_bool()? || release.get("prerelease")?.as_bool()? {
        return None;
    }
    let tag = release.get("tag_name")?.as_str()?.trim();
    let latest = ComparableVersion::parse(tag)?;
    let current = ComparableVersion::parse(current)?;
    Some(if latest.cmp(&current).is_gt() {
        Outcome::Available(Release {
            version: tag.strip_prefix(['v', 'V']).unwrap_or(tag).to_string(),
            url: release["html_url"]
                .as_str()
                .filter(|u| u.starts_with(&format!("{RELEASES_URL}/tag/")))
                .unwrap_or(RELEASES_URL)
                .to_string(),
            notes: release["body"]
                .as_str()
                .unwrap_or("")
                .chars()
                .take(6000)
                .collect(),
            download: release["assets"].as_array().and_then(|assets| {
                assets.iter().find_map(|a| {
                    let name = a["name"].as_str()?.to_lowercase();
                    let url = a["browser_download_url"].as_str()?;
                    (compatible_asset(&name, std::env::consts::ARCH)
                        && url.starts_with(&format!("{RELEASES_URL}/download/")))
                    .then(|| url.to_string())
                })
            }),
        })
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
        .header("X-GitHub-Api-Version", "2022-11-28")
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
    fn mac_updates_match_chip_and_never_offer_windows_packages() {
        assert!(compatible_asset("dragon-bubble-macos-arm64.zip", "aarch64"));
        assert!(compatible_asset(
            "dragon-bubble-aarch64-apple-darwin.zip",
            "aarch64"
        ));
        assert!(compatible_asset(
            "dragon-bubble-macos-universal.dmg",
            "aarch64"
        ));
        assert!(!compatible_asset(
            "dragon-bubble-macos-intel.zip",
            "aarch64"
        ));
        assert!(!compatible_asset(
            "dragon-bubble-windows-arm64.zip",
            "aarch64"
        ));
        assert!(!compatible_asset("dragon-bubble-windows-x64.exe", "x86_64"));
        assert!(compatible_asset("dragon-bubble-macos-intel.zip", "x86_64"));
        let body = serde_json::json!({"tag_name":"v9.0.0", "draft":false, "prerelease":false,
            "assets":[{"name":"windows-x64.zip", "browser_download_url":format!("{RELEASES_URL}/download/v9.0.0/windows-x64.zip")}]}).to_string();
        let Some(Outcome::Available(release)) = classify_release(&body, VERSION) else {
            panic!("new release");
        };
        assert!(release.download.is_none());
    }
    #[test]
    fn compares_release_versions_numerically_and_handles_invalid_metadata() {
        let release = |tag: &str| {
            serde_json::json!({"tag_name": tag, "draft": false, "prerelease": false}).to_string()
        };
        assert_eq!(
            classify_release(&release("v0.10.0"), "0.9.0"),
            Some(Outcome::Available(Release {
                version: "0.10.0".into(),
                url: RELEASES_URL.into(),
                notes: String::new(),
                download: None
            }))
        );
        assert_eq!(
            classify_release(&release("v0.1.0"), VERSION),
            Some(Outcome::Current)
        );
        assert_eq!(
            classify_release(&release("0.0.9"), VERSION),
            Some(Outcome::Current)
        );
        assert_eq!(
            classify_release(&release("v0.1.2a"), "0.1.2"),
            Some(Outcome::Available(Release {
                version: "0.1.2a".into(),
                url: RELEASES_URL.into(),
                notes: String::new(),
                download: None
            }))
        );
        assert_eq!(
            classify_release(&release("v0.1.2a"), VERSION),
            Some(Outcome::Current)
        );
        assert_eq!(
            classify_release(&release("v0.1.2b"), VERSION),
            Some(Outcome::Available(Release {
                version: "0.1.2b".into(),
                url: RELEASES_URL.into(),
                notes: String::new(),
                download: None
            }))
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
