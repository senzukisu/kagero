//! Release identity: where official updates come from and which integrations are wired.
//!
//! Maintainers: these values are public (no secrets). See `docs/developers/services.md`
//! before changing them.

/// GitHub account that publishes official Kagero releases.
pub const GITHUB_OWNER: &str = "senzukisu";
/// GitHub repository with the official releases.
pub const GITHUB_REPO: &str = "kagero";

/// Human-facing releases page.
pub fn releases_url() -> String {
    format!("https://github.com/{GITHUB_OWNER}/{GITHUB_REPO}/releases")
}

/// GitHub REST endpoint listing releases (newest first).
pub fn releases_api_url() -> String {
    format!("https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/releases?per_page=20")
}

/// Only assets under this prefix are ever downloaded.
pub fn trusted_download_prefix() -> String {
    format!("https://github.com/{GITHUB_OWNER}/{GITHUB_REPO}/releases/download/")
}

/// minisign public key (the base64 line of `kagero.pub`) used to verify release
/// packages. Empty means "not configured": updates can be checked and downloaded but
/// never installed.
pub const UPDATE_PUBLIC_KEY: &str = "RWSXS+R3BCdmSx47V91P3O9sUYv3G3tdbivI9Dta7p0hSgnJqkRV8wDe";

/// Discord application ("client") ID for Rich Presence. Public, not a secret.
pub const DISCORD_APPLICATION_ID: &str = "1564753232763424869";

/// Platform tag used in release asset names, or `None` where no build is published.
pub fn platform_tag() -> Option<&'static str> {
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some("windows-x64")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("linux-x64")
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("macos-arm64")
    } else {
        None
    }
}

/// Name of the release package for a version, e.g. `kagero-0.2.0-windows-x64.zip`.
pub fn package_name(version: &str, platform: &str) -> String {
    format!("kagero-{version}-{platform}.zip")
}

/// True for builds produced by the release process (`KAGERO_OFFICIAL_BUILD=1` at build
/// time). Development builds can check for updates but never install them.
pub fn is_official_build() -> bool {
    matches!(option_env!("KAGERO_OFFICIAL_BUILD"), Some("1"))
}
