//! GitHub Releases updater: checks for newer versions, downloads and verifies packages.
//!
//! Network work runs on background threads; the UI polls [`UpdateManager::status`].
//! Only assets from the official repository (see [`crate::release`]) are downloaded, and
//! a package is installable only after its minisign signature verifies against the
//! public key compiled into Kagero. See `docs/UPDATER.md`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::release;

/// Largest release list response accepted from the API.
const MAX_API_BYTES: u64 = 2 * 1024 * 1024;
/// Largest package download.
const MAX_PACKAGE_BYTES: u64 = 512 * 1024 * 1024;
/// Largest signature or checksum file.
const MAX_SMALL_FILE_BYTES: u64 = 64 * 1024;
/// Release notes are shortened to this many characters for display.
const MAX_NOTES_CHARS: usize = 1500;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UpdateError {
    #[error("couldn't reach GitHub ({0}). Kagero works normally offline; try again later")]
    Network(String),
    #[error("GitHub's request limit was reached. Try again in an hour")]
    RateLimited,
    #[error("GitHub answered with HTTP {0}")]
    Http(u16),
    #[error("the release information from GitHub is invalid: {0}")]
    InvalidResponse(String),
    #[error("no update package for this platform was published with this release")]
    NoAsset,
    #[error("the download was canceled")]
    Canceled,
    #[error("the download is incomplete ({got} of {expected} bytes)")]
    SizeMismatch { expected: u64, got: u64 },
    #[error("the download is larger than allowed")]
    TooLarge,
    #[error("the package signature is missing or invalid, so it was not installed")]
    BadSignature,
    #[error("the package checksum does not match the published SHA256SUMS")]
    ChecksumMismatch,
    #[error("the download URL is not an official Kagero release URL")]
    UntrustedUrl,
    #[error("file error: {0}")]
    Io(String),
}

impl From<std::io::Error> for UpdateError {
    fn from(e: std::io::Error) -> Self {
        UpdateError::Io(e.to_string())
    }
}

// ---------------------------------------------------------------- HTTP

/// Minimal HTTP interface so the logic can be tested with canned responses.
pub trait HttpClient: Send + Sync {
    /// GET a small resource into memory, refusing bodies over `limit` bytes.
    fn get_bytes(&self, url: &str, limit: u64) -> Result<Vec<u8>, UpdateError>;

    /// Streams a GET into `out`. `progress(received, total)` returns `false` to cancel.
    fn download(
        &self,
        url: &str,
        out: &mut dyn Write,
        limit: u64,
        progress: &mut dyn FnMut(u64, Option<u64>) -> bool,
    ) -> Result<u64, UpdateError>;
}

/// Real HTTPS client using the operating system's TLS and certificate store.
pub struct UreqClient {
    api: ureq::Agent,
    downloads: ureq::Agent,
}

impl UreqClient {
    pub fn new() -> Self {
        let make = |body_timeout: Duration| -> ureq::Agent {
            let tls = ureq::tls::TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::NativeTls)
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build();
            ureq::Agent::config_builder()
                .tls_config(tls)
                .http_status_as_error(false)
                .user_agent(format!(
                    "Kagero/{} (+{})",
                    crate::APP_VERSION,
                    release::releases_url()
                ))
                .timeout_connect(Some(Duration::from_secs(10)))
                .timeout_recv_response(Some(Duration::from_secs(20)))
                .timeout_recv_body(Some(body_timeout))
                .max_redirects(5)
                .build()
                .into()
        };
        Self {
            api: make(Duration::from_secs(20)),
            downloads: make(Duration::from_secs(15 * 60)),
        }
    }

    fn call(
        agent: &ureq::Agent,
        url: &str,
    ) -> Result<ureq::http::Response<ureq::Body>, UpdateError> {
        let response = agent
            .get(url)
            .header(
                "Accept",
                "application/vnd.github+json, application/octet-stream",
            )
            .header("X-GitHub-Api-Version", "2022-11-28")
            .call()
            .map_err(|e| UpdateError::Network(e.to_string()))?;
        let status = response.status().as_u16();
        if status == 403 || status == 429 {
            let remaining = response
                .headers()
                .get("x-ratelimit-remaining")
                .and_then(|v| v.to_str().ok());
            if status == 429 || remaining == Some("0") {
                return Err(UpdateError::RateLimited);
            }
        }
        if !(200..300).contains(&status) {
            return Err(UpdateError::Http(status));
        }
        Ok(response)
    }
}

impl Default for UreqClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient for UreqClient {
    fn get_bytes(&self, url: &str, limit: u64) -> Result<Vec<u8>, UpdateError> {
        let mut response = Self::call(&self.api, url)?;
        response
            .body_mut()
            .with_config()
            .limit(limit)
            .read_to_vec()
            .map_err(|e| match e {
                ureq::Error::BodyExceedsLimit(_) => UpdateError::TooLarge,
                other => UpdateError::Network(other.to_string()),
            })
    }

    fn download(
        &self,
        url: &str,
        out: &mut dyn Write,
        limit: u64,
        progress: &mut dyn FnMut(u64, Option<u64>) -> bool,
    ) -> Result<u64, UpdateError> {
        let mut response = Self::call(&self.downloads, url)?;
        let total = response.body().content_length();
        if total.is_some_and(|t| t > limit) {
            return Err(UpdateError::TooLarge);
        }
        let mut reader = response.body_mut().with_config().limit(limit).reader();
        let mut buf = vec![0u8; 64 * 1024];
        let mut received = 0u64;
        loop {
            let n = std::io::Read::read(&mut reader, &mut buf)
                .map_err(|e| UpdateError::Network(e.to_string()))?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])?;
            received += n as u64;
            if !progress(received, total) {
                return Err(UpdateError::Canceled);
            }
        }
        Ok(received)
    }
}

// ---------------------------------------------------------------- releases

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    published_at: Option<String>,
    html_url: String,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

#[derive(Debug, Deserialize)]
struct GhAsset {
    name: String,
    size: u64,
    browser_download_url: String,
}

/// A downloadable file attached to a release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

/// A newer release, as shown in Settings → Updates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub version: Version,
    pub title: String,
    /// Plain-text release notes, shortened. Displayed only, never interpreted.
    pub notes: String,
    /// `YYYY-MM-DD`, if GitHub provided a date.
    pub published: String,
    pub page_url: String,
    pub prerelease: bool,
    pub package: Option<Asset>,
    pub signature: Option<Asset>,
    pub checksums: Option<Asset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckOutcome {
    UpToDate,
    Available(Box<ReleaseInfo>),
}

/// Parses a tag like `v0.2.0` or `0.3.0-beta.1` into a semantic version.
pub fn parse_tag(tag: &str) -> Option<Version> {
    let trimmed = tag.trim();
    let bare = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);
    Version::parse(bare).ok()
}

/// The version Kagero was built as.
pub fn current_version() -> Version {
    Version::parse(crate::APP_VERSION).unwrap_or_else(|_| Version::new(0, 0, 0))
}

fn clean_notes(notes: &str) -> String {
    let cleaned: String = notes
        .chars()
        .filter(|c| !c.is_control() || *c == '\n')
        .take(MAX_NOTES_CHARS)
        .collect();
    if notes.chars().count() > MAX_NOTES_CHARS {
        format!("{cleaned}…")
    } else {
        cleaned
    }
}

fn trusted_asset(asset: &GhAsset) -> Option<Asset> {
    asset
        .browser_download_url
        .starts_with(&release::trusted_download_prefix())
        .then(|| Asset {
            name: asset.name.clone(),
            url: asset.browser_download_url.clone(),
            size: asset.size,
        })
}

/// Picks the newest release that is newer than `current`, from a GitHub releases list.
///
/// Drafts, unparsable tags and (unless `include_prereleases`) prereleases are ignored.
/// Assets are only taken from the official repository's download URLs.
pub fn select_update(
    json: &[u8],
    current: &Version,
    include_prereleases: bool,
    platform: Option<&str>,
) -> Result<CheckOutcome, UpdateError> {
    let releases: Vec<GhRelease> =
        serde_json::from_slice(json).map_err(|e| UpdateError::InvalidResponse(e.to_string()))?;
    let best = releases
        .iter()
        .filter(|r| !r.draft)
        .filter_map(|r| parse_tag(&r.tag_name).map(|v| (v, r)))
        .filter(|(v, r)| include_prereleases || (v.pre.is_empty() && !r.prerelease))
        .filter(|(v, _)| v > current)
        .max_by(|a, b| a.0.cmp(&b.0));
    let Some((version, gh)) = best else {
        return Ok(CheckOutcome::UpToDate);
    };
    let page_prefix = format!(
        "https://github.com/{}/{}/",
        release::GITHUB_OWNER,
        release::GITHUB_REPO
    );
    let page_url = if gh.html_url.starts_with(&page_prefix) {
        gh.html_url.clone()
    } else {
        release::releases_url()
    };
    let package_name = platform.map(|p| release::package_name(&version.to_string(), p));
    let find = |name: &str| {
        gh.assets
            .iter()
            .find(|a| a.name == name)
            .and_then(trusted_asset)
    };
    let package = package_name.as_deref().and_then(find);
    let signature = package_name
        .as_deref()
        .and_then(|n| find(&format!("{n}.minisig")));
    Ok(CheckOutcome::Available(Box::new(ReleaseInfo {
        title: gh
            .name
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| format!("Kagero {version}")),
        notes: clean_notes(gh.body.as_deref().unwrap_or("")),
        published: gh
            .published_at
            .as_deref()
            .map(|d| d.chars().take(10).collect())
            .unwrap_or_default(),
        page_url,
        prerelease: gh.prerelease || !version.pre.is_empty(),
        package,
        signature,
        checksums: find("SHA256SUMS.txt"),
        version,
    })))
}

/// Asks GitHub for releases and picks the best update.
pub fn check_for_update(
    http: &dyn HttpClient,
    include_prereleases: bool,
) -> Result<CheckOutcome, UpdateError> {
    let body = http.get_bytes(&release::releases_api_url(), MAX_API_BYTES)?;
    select_update(
        &body,
        &current_version(),
        include_prereleases,
        release::platform_tag(),
    )
}

// ---------------------------------------------------------------- verification

/// Verifies a minisign signature (`.minisig` text) of `data` with a base64 public key.
pub fn verify_signature(public_key: &str, data: &[u8], signature: &str) -> Result<(), UpdateError> {
    let key = minisign_verify::PublicKey::from_base64(public_key.trim())
        .map_err(|_| UpdateError::BadSignature)?;
    let sig =
        minisign_verify::Signature::decode(signature).map_err(|_| UpdateError::BadSignature)?;
    key.verify(data, &sig, false)
        .map_err(|_| UpdateError::BadSignature)
}

/// Looks up `file`'s hash in a `SHA256SUMS` text (`<hex>  <name>` per line).
pub fn checksum_for(sums: &str, file: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?.trim_start_matches('*');
        (name == file && hash.len() == 64).then(|| hash.to_ascii_lowercase())
    })
}

pub fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// A downloaded package and how far it could be trusted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadedPackage {
    pub path: PathBuf,
    pub sha256: String,
    /// The signature verified against the compiled-in public key.
    pub signature_verified: bool,
}

/// Downloads the release package into `dir`, checks size and checksum, and verifies
/// the signature when a public key is configured.
pub fn download_package(
    http: &dyn HttpClient,
    info: &ReleaseInfo,
    dir: &Path,
    public_key: &str,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<DownloadedPackage, UpdateError> {
    let package = info.package.as_ref().ok_or(UpdateError::NoAsset)?;
    if !package.url.starts_with(&release::trusted_download_prefix())
        || package.name.contains(['/', '\\'])
    {
        return Err(UpdateError::UntrustedUrl);
    }
    if package.size > MAX_PACKAGE_BYTES {
        return Err(UpdateError::TooLarge);
    }
    std::fs::create_dir_all(dir)?;
    let final_path = dir.join(&package.name);
    let part_path = dir.join(format!("{}.part", package.name));

    let result = (|| {
        let mut file = std::fs::File::create(&part_path)?;
        let mut data = Vec::with_capacity(package.size.min(MAX_PACKAGE_BYTES) as usize);
        let mut sink = TeeWriter {
            file: &mut file,
            copy: &mut data,
        };
        let received = http.download(
            &package.url,
            &mut sink,
            MAX_PACKAGE_BYTES,
            &mut |got, total| {
                progress(got, total.or(Some(package.size)));
                !cancel.load(Ordering::Relaxed)
            },
        )?;
        file.sync_all()?;
        drop(file);
        if received != package.size {
            return Err(UpdateError::SizeMismatch {
                expected: package.size,
                got: received,
            });
        }
        let sha256 = sha256_hex(&data);
        if let Some(sums) = &info.checksums {
            let text = small_text(http, sums)?;
            match checksum_for(&text, &package.name) {
                Some(expected) if expected == sha256 => {}
                _ => return Err(UpdateError::ChecksumMismatch),
            }
        }
        let signature_verified = if public_key.trim().is_empty() {
            false
        } else {
            let sig = info.signature.as_ref().ok_or(UpdateError::BadSignature)?;
            verify_signature(public_key, &data, &small_text(http, sig)?)?;
            true
        };
        std::fs::rename(&part_path, &final_path)?;
        Ok(DownloadedPackage {
            path: final_path.clone(),
            sha256,
            signature_verified,
        })
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&part_path);
    }
    result
}

fn small_text(http: &dyn HttpClient, asset: &Asset) -> Result<String, UpdateError> {
    if !asset.url.starts_with(&release::trusted_download_prefix()) {
        return Err(UpdateError::UntrustedUrl);
    }
    let bytes = http.get_bytes(&asset.url, MAX_SMALL_FILE_BYTES)?;
    String::from_utf8(bytes).map_err(|_| UpdateError::InvalidResponse("not text".into()))
}

/// Writes to a file and keeps an in-memory copy for hashing and signature checks.
struct TeeWriter<'a> {
    file: &'a mut std::fs::File,
    copy: &'a mut Vec<u8>,
}

impl Write for TeeWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.file.write(buf)?;
        self.copy.extend_from_slice(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

// ---------------------------------------------------------------- manager

/// What the updater is doing, for the settings page and notifications.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    Available(Box<ReleaseInfo>),
    Downloading {
        release: Box<ReleaseInfo>,
        received: u64,
        total: Option<u64>,
    },
    Downloaded {
        release: Box<ReleaseInfo>,
        package: DownloadedPackage,
    },
    Failed(String),
}

/// Something the app should react to (persist the check time, notify the user).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateEvent {
    CheckFinished { found: Option<Version> },
    CheckFailed(String),
    DownloadFinished,
}

struct Shared {
    status: UpdateStatus,
    events: Vec<UpdateEvent>,
}

type Repaint = Arc<dyn Fn() + Send + Sync>;

/// Runs checks and downloads on background threads.
pub struct UpdateManager {
    http: Arc<dyn HttpClient>,
    shared: Arc<Mutex<Shared>>,
    cancel: Arc<AtomicBool>,
    repaint: Repaint,
    download_dir: PathBuf,
}

impl UpdateManager {
    pub fn new(http: Arc<dyn HttpClient>, repaint: Repaint) -> Self {
        Self {
            http,
            shared: Arc::new(Mutex::new(Shared {
                status: UpdateStatus::Idle,
                events: Vec::new(),
            })),
            cancel: Arc::new(AtomicBool::new(false)),
            repaint,
            download_dir: std::env::temp_dir().join("kagero-update"),
        }
    }

    /// Where packages are downloaded (overridable for tests).
    pub fn with_download_dir(mut self, dir: PathBuf) -> Self {
        self.download_dir = dir;
        self
    }

    fn lock(shared: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
        // A panic while holding the lock leaves plain data behind; keep using it.
        shared.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn status(&self) -> UpdateStatus {
        Self::lock(&self.shared).status.clone()
    }

    pub fn take_events(&self) -> Vec<UpdateEvent> {
        std::mem::take(&mut Self::lock(&self.shared).events)
    }

    pub fn is_busy(&self) -> bool {
        matches!(
            self.status(),
            UpdateStatus::Checking | UpdateStatus::Downloading { .. }
        )
    }

    /// Starts a background check. Ignored while another operation runs.
    pub fn check(&self, include_prereleases: bool) {
        if self.is_busy() {
            return;
        }
        Self::lock(&self.shared).status = UpdateStatus::Checking;
        let (http, shared, repaint) =
            (self.http.clone(), self.shared.clone(), self.repaint.clone());
        let spawned = std::thread::Builder::new()
            .name("kagero-update-check".into())
            .spawn(move || {
                let result = check_for_update(http.as_ref(), include_prereleases);
                let mut s = Self::lock(&shared);
                match result {
                    Ok(CheckOutcome::UpToDate) => {
                        s.status = UpdateStatus::UpToDate;
                        s.events.push(UpdateEvent::CheckFinished { found: None });
                    }
                    Ok(CheckOutcome::Available(info)) => {
                        s.events.push(UpdateEvent::CheckFinished {
                            found: Some(info.version.clone()),
                        });
                        s.status = UpdateStatus::Available(info);
                    }
                    Err(e) => {
                        tracing::warn!("update check failed: {e}");
                        s.status = UpdateStatus::Failed(e.to_string());
                        s.events.push(UpdateEvent::CheckFailed(e.to_string()));
                    }
                }
                drop(s);
                repaint();
            });
        if let Err(e) = spawned {
            Self::lock(&self.shared).status = UpdateStatus::Failed(e.to_string());
        }
    }

    /// Downloads and verifies the available release in the background.
    pub fn download(&self) {
        let release = match self.status() {
            UpdateStatus::Available(r) => r,
            UpdateStatus::Failed(_) | UpdateStatus::Downloaded { .. } => return,
            _ => return,
        };
        if release.package.is_none() {
            Self::lock(&self.shared).status =
                UpdateStatus::Failed(UpdateError::NoAsset.to_string());
            return;
        }
        self.cancel.store(false, Ordering::Relaxed);
        Self::lock(&self.shared).status = UpdateStatus::Downloading {
            release: release.clone(),
            received: 0,
            total: release.package.as_ref().map(|p| p.size),
        };
        let (http, shared, repaint, cancel) = (
            self.http.clone(),
            self.shared.clone(),
            self.repaint.clone(),
            self.cancel.clone(),
        );
        let dir = self.download_dir.join(release.version.to_string());
        let spawned = std::thread::Builder::new()
            .name("kagero-update-download".into())
            .spawn(move || {
                let mut last_report = 0u64;
                let result = download_package(
                    http.as_ref(),
                    &release,
                    &dir,
                    release::UPDATE_PUBLIC_KEY,
                    &cancel,
                    &mut |received, total| {
                        // Report about every 256 KiB to keep the lock and repaints cheap.
                        if received - last_report >= 256 * 1024 || Some(received) == total {
                            last_report = received;
                            if let UpdateStatus::Downloading {
                                received: r,
                                total: t,
                                ..
                            } = &mut Self::lock(&shared).status
                            {
                                *r = received;
                                *t = total;
                            }
                            repaint();
                        }
                    },
                );
                let mut s = Self::lock(&shared);
                match result {
                    Ok(package) => {
                        s.status = UpdateStatus::Downloaded { release, package };
                        s.events.push(UpdateEvent::DownloadFinished);
                    }
                    Err(UpdateError::Canceled) => s.status = UpdateStatus::Available(release),
                    Err(e) => {
                        tracing::warn!("update download failed: {e}");
                        s.status = UpdateStatus::Failed(e.to_string());
                    }
                }
                drop(s);
                repaint();
            });
        if let Err(e) = spawned {
            Self::lock(&self.shared).status = UpdateStatus::Failed(e.to_string());
        }
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Clears an error so the page shows the neutral state again.
    pub fn dismiss_error(&self) {
        let mut s = Self::lock(&self.shared);
        if matches!(s.status, UpdateStatus::Failed(_)) {
            s.status = UpdateStatus::Idle;
        }
    }
}

/// Current time in Unix seconds (0 if the clock is before 1970).
pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Whether an automatic check is due.
pub fn check_due(last_check_unix: Option<u64>, interval_hours: u32, now_unix: u64) -> bool {
    match last_check_unix {
        None => true,
        Some(last) => now_unix.saturating_sub(last) >= u64::from(interval_hours) * 3600,
    }
}
