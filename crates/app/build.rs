fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rerun-if-changed=src/macos_accessibility.c");
        println!("cargo:rerun-if-changed=src/macos_hotkey.c");
        println!("cargo:rustc-link-lib=framework=ApplicationServices");
        println!("cargo:rustc-link-lib=framework=Carbon");
        cc::Build::new()
            .file("src/macos_accessibility.c")
            .file("src/macos_hotkey.c")
            .compile("agent_pad_macos");
    }
    tauri_build::build();
}
