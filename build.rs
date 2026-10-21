fn main() {
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
