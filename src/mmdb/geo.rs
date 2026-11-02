//! Where the geolocation databases come from. openpacketmon ships none; in this order it uses
//! - the file chosen in the settings,
//! - the free DB-IP Lite databases, downloaded from the settings page on request
//!   (<https://db-ip.com/db/lite.php>, CC BY 4.0: pages using them link to DB-IP),
//! - the `GeoLite2` files of `MaxMind` kept up to date by `geoipupdate` (needs an account there).
//!
//! Without any of them countries and AS stay unknown.

use std::io::Read;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::countries::types::country::Country;
use crate::mmdb::asn::get_asn;
use crate::mmdb::country::get_country;
use crate::mmdb::types::mmdb_reader::MmdbReader;
use crate::utils::formatted_strings::APP_VERSION;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GeoKind {
    Country,
    Asn,
}

impl GeoKind {
    pub const ALL: [GeoKind; 2] = [GeoKind::Country, GeoKind::Asn];

    pub fn code(self) -> &'static str {
        match self {
            GeoKind::Country => "country",
            GeoKind::Asn => "asn",
        }
    }

    /// Name of the downloaded file in [`geo_dir`]
    pub fn file_name(self) -> String {
        format!("dbip-{}-lite.mmdb", self.code())
    }

    /// Where `geoipupdate` keeps the database of `MaxMind`
    fn geoipupdate_file(self) -> &'static str {
        match self {
            GeoKind::Country => "/var/lib/GeoIP/GeoLite2-Country.mmdb",
            GeoKind::Asn => "/var/lib/GeoIP/GeoLite2-ASN.mmdb",
        }
    }
}

/// Folder of the downloaded databases: `$XDG_DATA_HOME/openpacketmon/geo`, by default
/// `~/.local/share/openpacketmon/geo`
#[cfg(not(windows))]
pub fn geo_dir() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_DATA_HOME").map(PathBuf::from) {
        Some(dir) if dir.is_absolute() => dir,
        _ => PathBuf::from(std::env::var_os("HOME")?).join(".local/share"),
    };
    Some(base.join("openpacketmon/geo"))
}

/// Folder of the downloaded databases: `%LOCALAPPDATA%\openpacketmon\geo` (there is no `HOME`
/// under Windows)
#[cfg(windows)]
pub fn geo_dir() -> Option<PathBuf> {
    let base = PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
    base.is_absolute()
        .then(|| base.join("openpacketmon").join("geo"))
}

/// The files tried, in this order
fn candidates(kind: GeoKind, custom: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !custom.trim().is_empty() {
        files.push(PathBuf::from(custom.trim()));
    }
    if let Some(dir) = geo_dir() {
        files.push(dir.join(kind.file_name()));
    }
    files.push(PathBuf::from(kind.geoipupdate_file()));
    files
}

/// The first database found (`custom` is the path from the settings, may be empty)
pub fn open(kind: GeoKind, custom: &str) -> MmdbReader {
    candidates(kind, custom)
        .iter()
        .find_map(|path| MmdbReader::open(path))
        .unwrap_or_default()
}

/// The database in use comes from DB-IP (its license asks for a link to db-ip.com)
pub fn is_dbip(reader: &MmdbReader) -> bool {
    reader
        .source()
        .is_some_and(|(_, metadata)| metadata.database_type.starts_with("DBIP"))
}

/// Days since the database was built
pub fn age_days(build_epoch: u64) -> u64 {
    let now = jiff::Timestamp::now().as_second().max(0).cast_unsigned();
    now.saturating_sub(build_epoch) / 86_400
}

/// A database older than this is shown with a hint to download it again
pub const OLD_AFTER_DAYS: u64 = 60;

const DOWNLOAD_URL: &str = "https://download.db-ip.com/free";
/// Limits for a download (the files have about 5 MB packed and 10 MB unpacked)
const MAX_PACKED: usize = 64 << 20;
const MAX_UNPACKED: u64 = 256 << 20;

static DOWNLOADING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, PartialEq, Eq)]
pub enum DownloadError {
    /// Another download is running
    Busy,
    Failed(String),
}

/// Downloads the current DB-IP Lite databases (country and ASN) into [`geo_dir`]
pub async fn download_dbip() -> Result<(), DownloadError> {
    if DOWNLOADING.swap(true, Ordering::SeqCst) {
        return Err(DownloadError::Busy);
    }
    let result = download_all().await.map_err(DownloadError::Failed);
    DOWNLOADING.store(false, Ordering::SeqCst);
    result
}

async fn download_all() -> Result<(), String> {
    let dir = geo_dir().ok_or("no home folder")?;
    let client = reqwest::Client::builder()
        .user_agent(format!("openpacketmon-{APP_VERSION}"))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    for kind in GeoKind::ALL {
        let packed = fetch(&client, kind).await?;
        let dir = dir.clone();
        tokio::task::spawn_blocking(move || install(kind, &packed, &dir))
            .await
            .map_err(|e| e.to_string())??;
    }
    Ok(())
}

/// The packed database of this month, or of the last month while the new one is not out yet
async fn fetch(client: &reqwest::Client, kind: GeoKind) -> Result<Vec<u8>, String> {
    let today = jiff::Zoned::now().date();
    let last_month = today
        .first_of_month()
        .yesterday()
        .map_err(|e| e.to_string())?;
    for (i, month) in [today, last_month].into_iter().enumerate() {
        let url = format!(
            "{DOWNLOAD_URL}/dbip-{}-lite-{}.mmdb.gz",
            kind.code(),
            month.strftime("%Y-%m")
        );
        let mut response = client.get(&url).send().await.map_err(|e| e.to_string())?;
        if i == 0 && response.status() == reqwest::StatusCode::NOT_FOUND {
            continue;
        }
        if !response.status().is_success() {
            return Err(format!("{url}: {}", response.status()));
        }
        let mut packed = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
            packed.extend_from_slice(&chunk);
            if packed.len() > MAX_PACKED {
                return Err(format!("{url}: file too large"));
            }
        }
        return Ok(packed);
    }
    Err(format!("no DB-IP {} database found", kind.code()))
}

/// Unpacks, checks and stores a downloaded database; an existing file is only replaced by a
/// working one
fn install(kind: GeoKind, packed: &[u8], dir: &Path) -> Result<(), String> {
    let mut mmdb = Vec::new();
    flate2::read::GzDecoder::new(packed)
        .take(MAX_UNPACKED)
        .read_to_end(&mut mmdb)
        .map_err(|e| format!("unpacking: {e}"))?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(kind.file_name());
    let part = dir.join(format!("{}.part", kind.file_name()));
    std::fs::write(&part, &mmdb).map_err(|e| format!("{}: {e}", part.display()))?;
    if let Err(e) = check(kind, &part) {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    std::fs::rename(&part, &path).map_err(|e| format!("{}: {e}", path.display()))
}

/// A database of the expected kind that knows a well-known address
fn check(kind: GeoKind, path: &Path) -> Result<(), String> {
    let reader = MmdbReader::open(path).ok_or("no MMDB database")?;
    let known = IpAddr::from([8, 8, 8, 8]);
    let ok = match kind {
        GeoKind::Country => get_country(&known, &reader) != Country::ZZ,
        GeoKind::Asn => !get_asn(&known, &reader).code.is_empty(),
    };
    if ok {
        Ok(())
    } else {
        Err(format!("not a {} database", kind.code()))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn gzip(data: &[u8]) -> Vec<u8> {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("openpacketmon-geo-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_file_of_the_settings_comes_first() {
        let files = candidates(GeoKind::Asn, " /data/my-asn.mmdb ");
        assert_eq!(files[0], PathBuf::from("/data/my-asn.mmdb"));
        assert!(files[1].ends_with("openpacketmon/geo/dbip-asn-lite.mmdb"));
        assert_eq!(files[2], PathBuf::from("/var/lib/GeoIP/GeoLite2-ASN.mmdb"));
        assert_eq!(candidates(GeoKind::Country, "").len(), 2);
    }

    #[test]
    fn a_missing_database_falls_back() {
        let reader = open(
            GeoKind::Country,
            "resources/test/ipinfo_location_sample.mmdb",
        );
        assert!(reader.source().is_some());
        assert!(!is_dbip(&reader));
        assert!(age_days(0) > 20_000);
    }

    #[test]
    fn broken_downloads_are_not_installed() {
        let dir = temp_dir("broken");
        // not packed
        assert!(install(GeoKind::Country, b"<html>error</html>", &dir).is_err());
        // packed, but no database
        assert!(install(GeoKind::Country, &gzip(b"no database"), &dir).is_err());
        // a database without 8.8.8.8 (the sample of IPinfo)
        let sample = std::fs::read("resources/test/ipinfo_location_sample.mmdb").unwrap();
        let error = install(GeoKind::Country, &gzip(&sample), &dir).unwrap_err();
        assert_eq!(error, "not a country database");
        assert!(!dir.join(GeoKind::Country.file_name()).exists());
        assert!(!dir.join("dbip-country-lite.mmdb.part").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
