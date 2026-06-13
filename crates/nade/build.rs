fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("src/pinch_monitor.m")
            .flag("-fobjc-arc")
            .compile("nade_pinch");
        println!("cargo:rustc-link-lib=framework=AppKit");
    }
}
