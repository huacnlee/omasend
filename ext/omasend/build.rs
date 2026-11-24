fn main() {
    // The TSX interface: `nsh build --release` writes `bundle.js` into the
    // directory OMASEND_BUNDLE names; the binaries embed it. Without it they
    // read the file OMASEND_BUNDLE names at run time (development, tests).
    println!("cargo:rustc-check-cfg=cfg(omasend_bundle)");
    println!("cargo:rerun-if-env-changed=OMASEND_BUNDLE");
    if let Some(dir) = std::env::var_os("OMASEND_BUNDLE") {
        let file = std::path::Path::new(&dir).join("bundle.js");
        if file.is_file() {
            println!("cargo:rerun-if-changed={}", file.display());
            println!("cargo:rustc-cfg=omasend_bundle");
            println!("cargo:rustc-env=OMASEND_BUNDLE_JS={}", file.display());
        }
    }
    println!("cargo:rerun-if-changed=native/ios/services.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("ios") {
        cc::Build::new()
            .file("native/ios/services.m")
            .flag("-fobjc-arc")
            .compile("omasend_ios_services");
        for framework in ["UIKit", "Foundation", "UniformTypeIdentifiers", "PhotosUI"] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
    }
}
