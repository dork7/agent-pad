fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rerun-if-changed=src/macos_hotkey.c");
        println!("cargo:rustc-link-lib=framework=Carbon");
        cc::Build::new()
            .file("src/macos_hotkey.c")
            .compile("skills_pad_macos_hotkey");
    }
    tauri_build::build();
}
