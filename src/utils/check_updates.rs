// Modified for openpacketmon (2026), see NOTICE. Based on `src/utils/check_updates.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use crate::utils::error_logger::{ErrorLogger, Location};
use crate::utils::formatted_strings::APP_VERSION;
use crate::{APP_NAME, location};
use semver::Version;
use serde::Deserialize;
use std::time::Duration;

/// Longest release notes kept; they are text from the network
const MAX_NOTES: usize = 20_000;

/// Result of the check for a newer release
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UpdateStatus {
    #[default]
    Unknown,
    InProgress,
    UpToDate(Release),
    UpdateAvailable(Release),
}

impl UpdateStatus {
    /// The latest release, once the check found it
    pub fn release(&self) -> Option<&Release> {
        match self {
            Self::UpToDate(release) | Self::UpdateAvailable(release) => Some(release),
            Self::Unknown | Self::InProgress => None,
        }
    }
}

/// The latest release on GitHub, as the check saw it
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// Version without the leading `v`
    pub version: String,
    /// When it was published
    pub published: Option<jiff::Timestamp>,
    /// The release notes (Markdown, as written on GitHub), cut to a sane length
    pub notes: String,
    /// When the check asked GitHub
    pub checked: jiff::Timestamp,
}

#[derive(Deserialize, Debug)]
struct AppVersion {
    name: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
}

/// The release notes, at most [`MAX_NOTES`] bytes and cut at a character
fn notes(body: Option<String>) -> String {
    let mut notes = body.unwrap_or_default().replace("\r\n", "\n");
    if notes.len() > MAX_NOTES {
        let mut end = MAX_NOTES;
        while !notes.is_char_boundary(end) {
            end -= 1;
        }
        notes.truncate(end);
    }
    notes
}

/// Checks whether a newer release of openpacketmon is available on GitHub
pub async fn is_newer_release_available() -> UpdateStatus {
    is_newer_release_available_inner(6, 30).await
}

async fn is_newer_release_available_inner(
    max_retries: u8,
    seconds_between_retries: u8,
) -> UpdateStatus {
    let Ok(client) = reqwest::Client::builder()
        .user_agent(format!("{APP_NAME}-{APP_VERSION}"))
        // a stalled connection would leave the check waiting for good
        .timeout(Duration::from_secs(30))
        .build()
        .log_err(location!())
    else {
        return UpdateStatus::Unknown;
    };
    let response = client
        .get("https://api.github.com/repos/tomCSLK/openpacketmon/releases/latest")
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await;

    if let Ok(result) = response {
        let result_json = result.json::<AppVersion>().await;

        #[cfg(test)]
        if result_json.is_err() {
            let response2 = client
                .get("https://api.github.com/repos/tomCSLK/openpacketmon/releases/latest")
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .send()
                .await;
            println!("\nResponse text: {:?}", response2.unwrap());
            println!("JSON result: {result_json:?}\n");
        }

        let latest = result_json.unwrap_or_else(|_| AppVersion {
            name: String::from(":-("),
            body: None,
            published_at: None,
        });
        // release name sample: v1.2.3
        let stripped = latest.name.trim().trim_start_matches('v').to_owned();

        if let (Ok(latest_semver), Ok(current_semver)) =
            (Version::parse(&stripped), Version::parse(APP_VERSION))
        {
            let release = Release {
                version: stripped,
                published: latest.published_at.and_then(|at| at.parse().ok()),
                notes: notes(latest.body),
                checked: jiff::Timestamp::now(),
            };
            return if latest_semver > current_semver {
                UpdateStatus::UpdateAvailable(release)
            } else {
                UpdateStatus::UpToDate(release)
            };
        }
    }
    let retries_left = max_retries.saturating_sub(1);
    if retries_left > 0 {
        // sleep seconds_between_retries and retries the request
        tokio::time::sleep(Duration::from_secs(u64::from(seconds_between_retries))).await;
        Box::pin(is_newer_release_available_inner(
            retries_left,
            seconds_between_retries,
        ))
        .await
    } else {
        UpdateStatus::Unknown
    }
}

#[cfg(all(test, not(target_os = "macos")))]
mod tests {
    use super::*;
    use std::assert_matches;

    #[tokio::test]
    #[ignore = "needs network access and a published release of openpacketmon"]
    async fn test_fetch_latest_release_from_github() {
        let result = is_newer_release_available_inner(6, 2).await;
        assert_matches!(
            result,
            UpdateStatus::UpToDate(_) | UpdateStatus::UpdateAvailable(_)
        );
    }

    #[test]
    fn long_notes_are_cut_at_a_character() {
        let long = "ä".repeat(MAX_NOTES);
        let cut = notes(Some(long));
        assert!(cut.len() <= MAX_NOTES);
        assert!(cut.chars().all(|c| c == 'ä'));
        assert_eq!(notes(Some("a\r\nb".to_owned())), "a\nb");
        assert_eq!(notes(None), "");
    }
}
