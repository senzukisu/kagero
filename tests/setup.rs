//! The installer and uninstaller logic, with test folders instead of the real Start
//! menu and desktop, and without touching the registry.

use std::io::Write;
use std::path::{Path, PathBuf};

use kagero::services::setup::{
    self, install, registry_values, uninstall, InstallOptions, ShellPaths, UNINSTALL_EXE,
};
use sha2::{Digest, Sha256};

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kagero-setup-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp");
    dir
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A release-like zip: the two programs and SHA256SUMS.txt.
fn package(dir: &Path, version: &str, damage: bool) -> PathBuf {
    let app = format!("kagero {version}").into_bytes();
    let updater = format!("updater {version}").into_bytes();
    let mut sums = format!(
        "{}  kagero.exe\n{}  kagero-updater.exe\n",
        hex(&app),
        hex(&updater)
    );
    if damage {
        sums = sums.replacen('a', "b", 1).replacen('0', "1", 1);
    }
    let path = dir.join(format!("kagero-{version}.zip"));
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).expect("zip"));
    for (name, data) in [
        ("kagero.exe", app),
        ("kagero-updater.exe", updater),
        ("SHA256SUMS.txt", sums.into_bytes()),
    ] {
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .expect("entry");
        zip.write_all(&data).expect("write");
    }
    zip.finish().expect("finish");
    path
}

fn options(dir: &Path, version: &str, desktop: bool) -> InstallOptions {
    InstallOptions {
        dir: dir.to_path_buf(),
        start_menu_shortcut: true,
        desktop_shortcut: desktop,
        register: false,
        version: version.to_owned(),
    }
}

#[test]
fn install_update_and_uninstall_keep_user_data() {
    let root = temp("full");
    let dir = root.join("Programs").join("Kagero");
    let shell = ShellPaths {
        start_menu: Some(root.join("StartMenu")),
        desktop: Some(root.join("Desktop")),
    };
    // The user's data lives elsewhere and must never be touched.
    let user_data = root.join("AppData").join("OnibiLab").join("Kagero");
    std::fs::create_dir_all(user_data.join("themes")).expect("data");
    std::fs::write(user_data.join("config.json"), "{}").expect("data");
    let uninstaller = root.join("stub.exe");
    std::fs::write(&uninstaller, b"MZ uninstaller").expect("stub");

    let v1 = package(&root, "0.3.8", false);
    let report =
        install(&v1, &uninstaller, &options(&dir, "0.3.8", true), &shell).expect("install");
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(
        std::fs::read(dir.join("kagero.exe")).expect("exe"),
        b"kagero 0.3.8"
    );
    assert!(dir.join("kagero-updater.exe").exists());
    assert_eq!(
        std::fs::read(dir.join(UNINSTALL_EXE)).expect("u"),
        b"MZ uninstaller"
    );
    // A first install leaves no "previous version" to restore.
    assert!(!kagero::services::update_installer::has_backup(&dir));
    if cfg!(windows) {
        for lnk in [
            root.join("StartMenu/Kagero.lnk"),
            root.join("Desktop/Kagero.lnk"),
        ] {
            assert!(lnk.exists(), "{}", lnk.display());
            assert!(setup::shortcut_points_into(&lnk, &dir), "{}", lnk.display());
        }
    }

    // Installing a newer version over it keeps the old one as the previous version,
    // exactly like the built-in updater.
    let v2 = package(&root, "0.4.0", false);
    install(&v2, &uninstaller, &options(&dir, "0.4.0", false), &shell).expect("upgrade");
    assert_eq!(
        std::fs::read(dir.join("kagero.exe")).expect("exe"),
        b"kagero 0.4.0"
    );
    assert!(kagero::services::update_installer::has_backup(&dir));

    // Something the user put into the program folder stays, and so does the folder.
    std::fs::write(dir.join("my notes.txt"), "keep me").expect("note");
    // A shortcut to another (portable) Kagero is not removed.
    let other = root.join("Other");
    std::fs::create_dir_all(&other).expect("other");
    let report = uninstall(&dir, &shell, false).expect("uninstall");
    assert!(!dir.join("kagero.exe").exists());
    assert!(!dir.join("kagero-updater.exe").exists());
    assert!(!dir.join(UNINSTALL_EXE).exists());
    assert!(!dir.join(".kagero-backup").exists());
    assert!(dir.join("my notes.txt").exists());
    assert!(
        report.warnings.iter().any(|w| w.contains("other files")),
        "{report:?}"
    );
    assert!(!root.join("StartMenu/Kagero.lnk").exists());
    assert!(!root.join("Desktop/Kagero.lnk").exists());
    // User data untouched.
    assert_eq!(
        std::fs::read_to_string(user_data.join("config.json")).expect("c"),
        "{}"
    );
    assert!(user_data.join("themes").is_dir());

    // With nothing else in it, the folder itself goes too.
    std::fs::remove_file(dir.join("my notes.txt")).expect("rm");
    install(&v2, &uninstaller, &options(&dir, "0.4.0", false), &shell).expect("again");
    uninstall(&dir, &shell, false).expect("uninstall");
    assert!(!dir.exists());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn shortcuts_of_other_copies_are_left_alone() {
    if !cfg!(windows) {
        return;
    }
    let root = temp("other-lnk");
    let mine = root.join("Mine");
    let portable = root.join("Portable");
    let shell = ShellPaths {
        start_menu: Some(root.join("StartMenu")),
        desktop: None,
    };
    let uninstaller = root.join("stub.exe");
    std::fs::write(&uninstaller, b"MZ").expect("stub");
    let pkg = package(&root, "0.4.0", false);
    // The Start menu shortcut belongs to a portable copy elsewhere.
    install(
        &pkg,
        &uninstaller,
        &options(&portable, "0.4.0", false),
        &shell,
    )
    .expect("portable");
    let mut opts = options(&mine, "0.4.0", false);
    opts.start_menu_shortcut = false;
    install(&pkg, &uninstaller, &opts, &shell).expect("mine");
    uninstall(&mine, &shell, false).expect("uninstall");
    assert!(
        root.join("StartMenu/Kagero.lnk").exists(),
        "the portable copy's shortcut stays"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn damaged_packages_and_wrong_folders_are_refused() {
    let root = temp("bad");
    let dir = root.join("Kagero");
    let shell = ShellPaths {
        start_menu: None,
        desktop: None,
    };
    let uninstaller = root.join("stub.exe");
    std::fs::write(&uninstaller, b"MZ").expect("stub");
    let bad = package(&root, "0.4.0", true);
    let err = install(&bad, &uninstaller, &options(&dir, "0.4.0", false), &shell).unwrap_err();
    assert!(err.contains("damaged"), "{err}");
    assert!(!dir.join("kagero.exe").exists(), "nothing installed");
    // A folder that isn't a Kagero install is never emptied.
    let random = root.join("Documents");
    std::fs::create_dir_all(&random).expect("dir");
    std::fs::write(random.join("thesis.docx"), "important").expect("file");
    assert!(uninstall(&random, &shell, false).is_err());
    assert!(random.join("thesis.docx").exists());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_installer_file_carries_its_package() {
    let root = temp("payload");
    let stub = b"MZ this is the setup program".to_vec();
    let zip = std::fs::read(package(&root, "0.4.0", false)).expect("zip");
    let installer = root.join("setup.exe");
    std::fs::write(&installer, setup::attach_payload(&stub, &zip)).expect("write");
    let (stub_out, zip_out) = (root.join("u.exe"), root.join("p.zip"));
    setup::unpack_installer(&installer, &stub_out, &zip_out).expect("unpack");
    assert_eq!(
        std::fs::read(&stub_out).expect("stub"),
        stub,
        "the uninstaller has no payload"
    );
    assert_eq!(std::fs::read(&zip_out).expect("zip"), zip);
    // The stub alone (uninstall.exe) has nothing to unpack.
    assert!(setup::unpack_installer(&stub_out, &root.join("a"), &root.join("b")).is_err());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn apps_and_features_entry() {
    let dir = Path::new(r"C:\Users\me\AppData\Local\Programs\Kagero");
    let values = registry_values(dir, "0.4.0");
    let get = |k: &str| values.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
    assert_eq!(get("DisplayName").as_deref(), Some("Kagero"));
    assert_eq!(get("DisplayVersion").as_deref(), Some("0.4.0"));
    assert_eq!(get("Publisher").as_deref(), Some("Onibi Lab"));
    assert_eq!(
        get("UninstallString").as_deref(),
        Some(r#""C:\Users\me\AppData\Local\Programs\Kagero\uninstall.exe""#)
    );
    assert!(setup::same_dir(
        dir,
        Path::new(r"c:\users\me\appdata\local\programs\kagero\")
    ));
    assert!(!setup::same_dir(dir, Path::new(r"C:\Kagero")));
}
