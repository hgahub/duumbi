fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        panic!(
            "Native Windows is no longer supported by DUUMBI. Use Linux or macOS. \
             The final Windows source snapshot is tagged windows-support-final-2026-09-10."
        );
    }
}
