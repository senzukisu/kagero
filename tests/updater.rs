//! GitHub updater and installer, with canned HTTP responses (never the real GitHub).

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use kagero::release;
use kagero::services::update_installer::{
    extract_package, has_backup, install_files, restore_backup, HelperArgs, InstallError, APP_EXE,
    BACKUP_DIR, UPDATER_EXE,
};
use kagero::services::updater::{
    checksum_for, download_package, parse_tag, select_update, sha256_hex, verify_signature,
    CheckOutcome, HttpClient, ReleaseInfo, UpdateError,
};
use semver::Version;

// Test vector from the minisign-verify crate: a prehashed signature of b"test".
const TEST_PUBLIC_KEY: &str = "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
const TEST_SIGNATURE: &str = "untrusted comment: signature from minisign secret key
RUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=
trusted comment: timestamp:1556193335\tfile:test
y/rUw2y8/hOUYjZU71eHp/Wo1KZ40fGy2VJEDl34XMJM+TX48Ss/17u3IvIfbVR1FkZZSNCisQbuQY+bHwhEBg==";

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("kagero-tests")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn dl(file: &str) -> String {
    format!("{}v9.0.0/{file}", release::trusted_download_prefix())
}

/// Canned responses keyed by URL.
#[derive(Default)]
struct MockHttp {
    responses: HashMap<String, Result<Vec<u8>, UpdateError>>,
}

impl MockHttp {
    fn with(mut self, url: &str, body: &[u8]) -> Self {
        self.responses.insert(url.to_owned(), Ok(body.to_vec()));
        self
    }

    fn failing(mut self, url: &str, error: UpdateError) -> Self {
        self.responses.insert(url.to_owned(), Err(error));
        self
    }

    fn lookup(&self, url: &str) -> Result<Vec<u8>, UpdateError> {
        self.responses
            .get(url)
            .cloned()
            .unwrap_or(Err(UpdateError::Http(404)))
    }
}

impl HttpClient for MockHttp {
    fn get_bytes(&self, url: &str, limit: u64) -> Result<Vec<u8>, UpdateError> {
        let body = self.lookup(url)?;
        if body.len() as u64 > limit {
            return Err(UpdateError::TooLarge);
        }
        Ok(body)
    }

    fn download(
        &self,
        url: &str,
        out: &mut dyn Write,
        limit: u64,
        progress: &mut dyn FnMut(u64, Option<u64>) -> bool,
    ) -> Result<u64, UpdateError> {
        let body = self.lookup(url)?;
        if body.len() as u64 > limit {
            return Err(UpdateError::TooLarge);
        }
        let mut sent = 0u64;
        for chunk in body.chunks(2) {
            out.write_all(chunk)
                .map_err(|e| UpdateError::Io(e.to_string()))?;
            sent += chunk.len() as u64;
            if !progress(sent, Some(body.len() as u64)) {
                return Err(UpdateError::Canceled);
            }
        }
        Ok(sent)
    }
}

fn release_json(tag: &str, prerelease: bool, draft: bool, assets: &[(&str, u64, &str)]) -> String {
    let assets: Vec<String> = assets
        .iter()
        .map(|(name, size, url)| {
            format!(r#"{{"name":"{name}","size":{size},"browser_download_url":"{url}"}}"#)
        })
        .collect();
    format!(
        r#"{{"tag_name":"{tag}","name":"Kagero {tag}","body":"Notes for {tag}","draft":{draft},
            "prerelease":{prerelease},"published_at":"2026-11-02T10:00:00Z",
            "html_url":"https://github.com/senzukisu/kagero/releases/tag/{tag}","assets":[{}]}}"#,
        assets.join(",")
    )
}

fn list(releases: &[String]) -> Vec<u8> {
    format!("[{}]", releases.join(",")).into_bytes()
}

fn available(outcome: CheckOutcome) -> ReleaseInfo {
    match outcome {
        CheckOutcome::Available(info) => *info,
        CheckOutcome::UpToDate => panic!("expected an update"),
    }
}

const WIN: Option<&str> = Some("windows-x64");

#[test]
fn versions_use_semver_not_strings() {
    assert_eq!(parse_tag("v0.10.0"), Some(Version::new(0, 10, 0)));
    assert_eq!(parse_tag("0.2.0"), Some(Version::new(0, 2, 0)));
    assert!(parse_tag("v0.3.0-beta.1").is_some_and(|v| !v.pre.is_empty()));
    assert_eq!(parse_tag("latest"), None);
    assert_eq!(parse_tag("v1"), None);
    let json = list(&[release_json("v0.10.0", false, false, &[])]);
    let info = available(select_update(&json, &Version::new(0, 9, 0), false, WIN).expect("ok"));
    assert_eq!(
        info.version,
        Version::new(0, 10, 0),
        "0.10 is newer than 0.9"
    );
}

#[test]
fn no_update_when_current_is_latest_or_newer() {
    let json = list(&[
        release_json("v0.2.0", false, false, &[]),
        release_json("v0.1.0", false, false, &[]),
    ]);
    let current = Version::new(0, 2, 0);
    assert_eq!(
        select_update(&json, &current, false, WIN),
        Ok(CheckOutcome::UpToDate)
    );
    assert_eq!(
        select_update(&json, &Version::new(0, 3, 0), true, WIN),
        Ok(CheckOutcome::UpToDate),
        "older releases are never offered"
    );
    assert_eq!(
        select_update(b"[]", &current, true, WIN),
        Ok(CheckOutcome::UpToDate)
    );
}

#[test]
fn picks_newest_stable_and_its_assets() {
    let pkg = release::package_name("0.4.0", "windows-x64");
    let json = list(&[
        release_json("v0.3.0", false, false, &[]),
        release_json(
            "v0.4.0",
            false,
            false,
            &[
                (&pkg, 1234, &dl(&pkg)),
                (
                    &format!("{pkg}.minisig"),
                    300,
                    &dl(&format!("{pkg}.minisig")),
                ),
                ("SHA256SUMS.txt", 100, &dl("SHA256SUMS.txt")),
                (
                    "kagero-0.4.0-linux-x64.zip",
                    999,
                    &dl("kagero-0.4.0-linux-x64.zip"),
                ),
            ],
        ),
        release_json("v9.9.9", false, true, &[]),
    ]);
    let info = available(select_update(&json, &Version::new(0, 2, 0), false, WIN).expect("ok"));
    assert_eq!(info.version, Version::new(0, 4, 0), "drafts are skipped");
    assert_eq!(info.published, "2026-11-02");
    assert_eq!(info.package.as_ref().map(|p| p.size), Some(1234));
    assert!(info.signature.is_some());
    assert!(info.checksums.is_some());
    assert!(info
        .page_url
        .starts_with("https://github.com/senzukisu/kagero/"));
}

#[test]
fn prereleases_only_when_enabled() {
    let json = list(&[
        release_json("v0.3.0-beta.1", true, false, &[]),
        release_json("v0.2.1", false, false, &[]),
    ]);
    let current = Version::new(0, 2, 0);
    let stable = available(select_update(&json, &current, false, WIN).expect("ok"));
    assert_eq!(stable.version, Version::new(0, 2, 1));
    let pre = available(select_update(&json, &current, true, WIN).expect("ok"));
    assert_eq!(pre.version.to_string(), "0.3.0-beta.1");
    assert!(pre.prerelease);
}

#[test]
fn invalid_api_data_is_an_error() {
    let current = Version::new(0, 2, 0);
    assert!(matches!(
        select_update(b"{not json", &current, false, WIN),
        Err(UpdateError::InvalidResponse(_))
    ));
    assert!(matches!(
        select_update(br#"{"message":"Not Found"}"#, &current, false, WIN),
        Err(UpdateError::InvalidResponse(_))
    ));
    // Unparsable tags are skipped rather than crashing.
    let json = list(&[release_json("nightly", false, false, &[])]);
    assert_eq!(
        select_update(&json, &current, true, WIN),
        Ok(CheckOutcome::UpToDate)
    );
}

#[test]
fn missing_or_untrusted_platform_assets() {
    let pkg = release::package_name("0.5.0", "windows-x64");
    let json = list(&[release_json(
        "v0.5.0",
        false,
        false,
        &[(&pkg, 10, "https://evil.example.com/kagero.zip")],
    )]);
    let info = available(select_update(&json, &Version::new(0, 2, 0), false, WIN).expect("ok"));
    assert!(
        info.package.is_none(),
        "assets outside the official repo are ignored"
    );
    let info = available(select_update(&json, &Version::new(0, 2, 0), false, None).expect("ok"));
    assert!(
        info.package.is_none(),
        "no package for an unsupported platform"
    );
}

#[test]
fn network_errors_reach_the_caller() {
    let http = MockHttp::default().failing(
        &release::releases_api_url(),
        UpdateError::Network("timed out".into()),
    );
    let err = kagero::services::updater::check_for_update(&http, false).expect_err("offline");
    assert!(matches!(err, UpdateError::Network(_)));
    assert!(
        err.to_string().contains("offline"),
        "message tells the user Kagero still works"
    );
}

fn info_for(data_len: u64, with_sums: bool) -> ReleaseInfo {
    let pkg = release::package_name("9.0.0", "windows-x64");
    let (extra, extra_size) = if with_sums {
        ("SHA256SUMS.txt", 100)
    } else {
        ("unrelated.txt", 1)
    };
    let extra_url = dl(extra);
    let json = list(&[release_json(
        "v9.0.0",
        false,
        false,
        &[
            (&pkg, data_len, &dl(&pkg)),
            (
                &format!("{pkg}.minisig"),
                300,
                &dl(&format!("{pkg}.minisig")),
            ),
            (extra, extra_size, &extra_url),
        ],
    )]);
    available(select_update(&json, &Version::new(0, 2, 0), false, WIN).expect("ok"))
}

#[test]
fn download_verifies_size_checksum_and_signature() {
    let dir = temp_dir("upd-ok");
    let info = info_for(4, true);
    let pkg = info.package.clone().expect("package");
    let sums = format!("{}  {}\n", sha256_hex(b"test"), pkg.name);
    let http = MockHttp::default()
        .with(&pkg.url, b"test")
        .with(
            &dl(&format!("{}.minisig", pkg.name)),
            TEST_SIGNATURE.as_bytes(),
        )
        .with(&dl("SHA256SUMS.txt"), sums.as_bytes());
    let mut last = 0;
    let done = download_package(
        &http,
        &info,
        &dir,
        TEST_PUBLIC_KEY,
        &AtomicBool::new(false),
        &mut |got, _| last = got,
    )
    .expect("verified download");
    assert!(done.signature_verified);
    assert_eq!(done.sha256, sha256_hex(b"test"));
    assert_eq!(last, 4, "progress reaches the full size");
    assert_eq!(std::fs::read(&done.path).expect("file"), b"test");
}

#[test]
fn download_without_key_is_kept_but_unverified() {
    let dir = temp_dir("upd-nokey");
    let info = info_for(4, false);
    let pkg = info.package.clone().expect("package");
    let http = MockHttp::default().with(&pkg.url, b"test");
    let done = download_package(
        &http,
        &info,
        &dir,
        "",
        &AtomicBool::new(false),
        &mut |_, _| {},
    )
    .expect("downloaded");
    assert!(
        !done.signature_verified,
        "never claims authenticity without a key"
    );
}

#[test]
fn bad_signature_size_or_checksum_is_rejected_and_cleaned_up() {
    let info = info_for(4, false);
    let pkg = info.package.clone().expect("package");
    let sig_url = dl(&format!("{}.minisig", pkg.name));

    let dir = temp_dir("upd-badsig");
    let tampered = MockHttp::default()
        .with(&pkg.url, b"Test")
        .with(&sig_url, TEST_SIGNATURE.as_bytes());
    let err = download_package(
        &tampered,
        &info,
        &dir,
        TEST_PUBLIC_KEY,
        &AtomicBool::new(false),
        &mut |_, _| {},
    )
    .expect_err("tampered file");
    assert_eq!(err, UpdateError::BadSignature);
    assert_eq!(
        std::fs::read_dir(&dir).expect("dir").count(),
        0,
        "partial file removed"
    );

    let no_sig = MockHttp::default().with(&pkg.url, b"test");
    let err = download_package(
        &no_sig,
        &info,
        &dir,
        TEST_PUBLIC_KEY,
        &AtomicBool::new(false),
        &mut |_, _| {},
    )
    .expect_err("missing signature");
    assert!(matches!(
        err,
        UpdateError::Http(404) | UpdateError::BadSignature
    ));

    let short = MockHttp::default().with(&pkg.url, b"tes");
    let err = download_package(
        &short,
        &info,
        &dir,
        "",
        &AtomicBool::new(false),
        &mut |_, _| {},
    )
    .expect_err("short download");
    assert_eq!(
        err,
        UpdateError::SizeMismatch {
            expected: 4,
            got: 3
        }
    );

    let info = info_for(4, true);
    let wrong_sums = MockHttp::default().with(&pkg.url, b"test").with(
        &dl("SHA256SUMS.txt"),
        format!("{}  {}", "0".repeat(64), pkg.name).as_bytes(),
    );
    let err = download_package(
        &wrong_sums,
        &info,
        &dir,
        "",
        &AtomicBool::new(false),
        &mut |_, _| {},
    )
    .expect_err("checksum mismatch");
    assert_eq!(err, UpdateError::ChecksumMismatch);
}

#[test]
fn download_can_be_canceled() {
    let dir = temp_dir("upd-cancel");
    let info = info_for(4, false);
    let pkg = info.package.clone().expect("package");
    let http = MockHttp::default().with(&pkg.url, b"test");
    let cancel = AtomicBool::new(true);
    let err =
        download_package(&http, &info, &dir, "", &cancel, &mut |_, _| {}).expect_err("canceled");
    assert_eq!(err, UpdateError::Canceled);
    assert_eq!(std::fs::read_dir(&dir).expect("dir").count(), 0);
}

#[test]
fn signature_and_checksum_helpers() {
    verify_signature(TEST_PUBLIC_KEY, b"test", TEST_SIGNATURE).expect("valid");
    assert_eq!(
        verify_signature(TEST_PUBLIC_KEY, b"test2", TEST_SIGNATURE),
        Err(UpdateError::BadSignature)
    );
    assert_eq!(
        verify_signature("not-a-key", b"test", TEST_SIGNATURE),
        Err(UpdateError::BadSignature)
    );
    let sums = format!("{}  a.zip\n{} *b.zip\n", "a".repeat(64), "B".repeat(64));
    assert_eq!(checksum_for(&sums, "a.zip"), Some("a".repeat(64)));
    assert_eq!(checksum_for(&sums, "b.zip"), Some("b".repeat(64)));
    assert_eq!(checksum_for(&sums, "c.zip"), None);
}

#[test]
fn update_checks_are_scheduled_by_interval() {
    use kagero::services::updater::check_due;
    assert!(check_due(None, 24, 1_000));
    assert!(!check_due(Some(1_000), 24, 1_000 + 3600));
    assert!(check_due(Some(1_000), 24, 1_000 + 24 * 3600));
}

// ---------------------------------------------------------------- installer

fn package_zip(dir: &std::path::Path, entries: &[(&str, &[u8])]) -> PathBuf {
    let path = dir.join("package.zip");
    let file = std::fs::File::create(&path).expect("create");
    let mut zip = zip::ZipWriter::new(file);
    for (name, data) in entries {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .expect("start");
        zip.write_all(data).expect("write");
    }
    zip.finish().expect("finish");
    path
}

#[test]
fn package_extraction_validates_entries() {
    let dir = temp_dir("inst-extract");
    let good = package_zip(
        &dir,
        &[
            (APP_EXE, b"new app"),
            (UPDATER_EXE, b"new helper"),
            ("README.md", b"r"),
        ],
    );
    let names = extract_package(&good, &dir.join("staging")).expect("valid package");
    assert_eq!(names.len(), 3);

    for (label, entries) in [
        (
            "traversal",
            vec![
                ("../evil.exe", b"x" as &[u8]),
                (APP_EXE, b"a"),
                (UPDATER_EXE, b"u"),
            ],
        ),
        (
            "nested",
            vec![
                ("bin/kagero.exe", b"x"),
                (APP_EXE, b"a"),
                (UPDATER_EXE, b"u"),
            ],
        ),
        (
            "hidden",
            vec![(".evil", b"x"), (APP_EXE, b"a"), (UPDATER_EXE, b"u")],
        ),
        ("no app", vec![(UPDATER_EXE, b"u")]),
        ("no helper", vec![(APP_EXE, b"a")]),
    ] {
        let zip = package_zip(&dir, &entries);
        assert!(
            extract_package(&zip, &dir.join("staging-bad")).is_err(),
            "accepted package with {label}"
        );
    }
    std::fs::write(dir.join("garbage.zip"), b"not a zip").expect("write");
    assert!(matches!(
        extract_package(&dir.join("garbage.zip"), &dir.join("s2")),
        Err(InstallError::BadPackage(_))
    ));
}

fn setup_install(name: &str) -> (PathBuf, PathBuf, PathBuf, Vec<String>) {
    let root = temp_dir(name);
    let source = root.join("source");
    let target = root.join("target");
    std::fs::create_dir_all(&source).expect("dir");
    std::fs::create_dir_all(&target).expect("dir");
    std::fs::write(source.join(APP_EXE), b"new app").expect("write");
    std::fs::write(source.join(UPDATER_EXE), b"new helper").expect("write");
    std::fs::write(source.join("NEW.txt"), b"added").expect("write");
    std::fs::write(target.join(APP_EXE), b"old app").expect("write");
    std::fs::write(target.join(UPDATER_EXE), b"old helper").expect("write");
    let names = vec![
        APP_EXE.to_owned(),
        UPDATER_EXE.to_owned(),
        "NEW.txt".to_owned(),
    ];
    let backup = target.join(BACKUP_DIR);
    (source, target, backup, names)
}

#[test]
fn install_replaces_files_and_keeps_a_backup_that_restores() {
    let (source, target, backup, names) = setup_install("inst-ok");
    install_files(
        &source,
        &target,
        &backup,
        &names,
        std::time::Duration::from_secs(1),
    )
    .expect("installed");
    assert_eq!(
        std::fs::read(target.join(APP_EXE)).expect("app"),
        b"new app"
    );
    assert_eq!(
        std::fs::read(target.join("NEW.txt")).expect("new"),
        b"added"
    );
    assert!(has_backup(&target));

    restore_backup(&target, &backup).expect("restored");
    assert_eq!(
        std::fs::read(target.join(APP_EXE)).expect("app"),
        b"old app"
    );
    assert_eq!(
        std::fs::read(target.join(UPDATER_EXE)).expect("helper"),
        b"old helper"
    );
    assert!(
        !target.join("NEW.txt").exists(),
        "files added by the update are removed"
    );
    assert!(!has_backup(&target));
}

#[test]
fn failed_install_rolls_back_to_the_previous_version() {
    let (source, target, backup, mut names) = setup_install("inst-fail");
    // A file listed but missing from the package makes the copy step fail.
    names.push("missing.dll".to_owned());
    let err = install_files(
        &source,
        &target,
        &backup,
        &names,
        std::time::Duration::from_secs(1),
    )
    .expect_err("must fail");
    assert!(matches!(err, InstallError::RolledBack(_)), "{err}");
    assert_eq!(
        std::fs::read(target.join(APP_EXE)).expect("app"),
        b"old app"
    );
    assert_eq!(
        std::fs::read(target.join(UPDATER_EXE)).expect("helper"),
        b"old helper"
    );
    assert!(!target.join("NEW.txt").exists());
}

#[test]
fn unwritable_or_unsafe_names_are_refused() {
    let (source, target, backup, _) = setup_install("inst-names");
    let names = vec!["..\\evil.exe".to_owned()];
    let err = install_files(
        &source,
        &target,
        &backup,
        &names,
        std::time::Duration::from_secs(1),
    )
    .expect_err("bad name");
    assert!(matches!(
        err,
        InstallError::RolledBack(_) | InstallError::BadPackage(_)
    ));
    let missing = target.join("does-not-exist");
    assert!(matches!(
        kagero::services::update_installer::check_writable(&missing),
        Err(InstallError::NotWritable(_))
    ));
}

#[test]
fn helper_arguments_round_trip_and_reject_unknown() {
    let args = HelperArgs {
        source: PathBuf::from("C:/tmp/files"),
        target: PathBuf::from("C:/Apps/Kagero"),
        restore: false,
    };
    assert_eq!(HelperArgs::parse(args.to_args()), Some(args));
    let restore = HelperArgs {
        source: PathBuf::new(),
        target: PathBuf::from("C:/Apps/Kagero"),
        restore: true,
    };
    assert_eq!(HelperArgs::parse(restore.to_args()), Some(restore));
    assert_eq!(
        HelperArgs::parse(vec!["--run".into(), "calc.exe".into()]),
        None
    );
    assert_eq!(HelperArgs::parse(Vec::<std::ffi::OsString>::new()), None);
}

#[test]
fn development_builds_never_install() {
    assert!(!release::is_official_build());
    let err =
        kagero::services::update_installer::launch_install(std::path::Path::new("x.zip"), "9.0.0")
            .expect_err("dev build");
    assert!(matches!(err, InstallError::Unsupported));
}
