//! Windows resources of the executables: the program icon (shown by Explorer, the
//! taskbar, shortcuts and "Apps & features") and the version information.
//!
//! Needs the Windows SDK resource compiler (rc.exe), which comes with the Visual
//! Studio Build Tools. Without it the build still succeeds, with a warning, and the
//! executables simply have no icon. Regenerate the icon with
//! `cargo run --example make_icon`.

fn main() {
    println!("cargo:rerun-if-changed=assets/kagero.ico");
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/kagero.ico")
        .set("ProductName", "Kagero")
        .set("FileDescription", "Kagero")
        .set("CompanyName", "Onibi Lab")
        .set("LegalCopyright", "MIT License · Onibi Lab");
    if let Err(e) = res.compile() {
        println!("cargo:warning=no Windows icon/version resource (rc.exe not found?): {e}");
    }
}
