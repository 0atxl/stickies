fn main() {
    println!("cargo:rerun-if-changed=assets");
    let target =
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("stickies.gresource");
    let status = std::process::Command::new("glib-compile-resources")
        .args(["--sourcedir=assets", "--target"])
        .arg(target)
        .arg("assets/stickies.gresource.xml")
        .status()
        .expect("glib-compile-resources is required");
    assert!(status.success(), "could not compile UI resources");
}
