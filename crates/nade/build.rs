fn main() {
	println!("cargo:rerun-if-changed=src/platform/macos_scroll.m");
	if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
		cc::Build::new()
			.file("src/platform/macos_scroll.m")
			.flag("-fobjc-arc")
			.compile("nade_macos_scroll");
		println!("cargo:rustc-link-lib=framework=AppKit");
	}
}
