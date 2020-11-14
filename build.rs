fn main() {
    println!("cargo:rerun-if-changed=native/bridge.c");
    println!("cargo:rerun-if-env-changed=LUCENT_MPFR_PREFIX");
    let mut bridge = cc::Build::new();
    bridge.file("native/bridge.c").flag_if_supported("-std=c11");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("emscripten") {
        let prefix = std::path::PathBuf::from(
            std::env::var_os("LUCENT_MPFR_PREFIX")
                .expect("set LUCENT_MPFR_PREFIX using scripts/build-wasm.sh"),
        );
        bridge
            .include(prefix.join("include"))
            .compile("lucent_mpfr");
    } else {
        let mpfr = pkg_config::Config::new()
            .cargo_metadata(false)
            .probe("mpfr")
            .expect("install MPFR and pkg-config");
        let gmp = pkg_config::Config::new()
            .cargo_metadata(false)
            .probe("gmp")
            .expect("install GMP");
        for path in mpfr.include_paths.iter().chain(&gmp.include_paths) {
            bridge.include(path);
        }
        bridge.compile("lucent_mpfr");
        for path in mpfr.link_paths.iter().chain(&gmp.link_paths) {
            println!("cargo:rustc-link-search=native={}", path.display());
        }
        println!("cargo:rustc-link-lib=mpfr");
        println!("cargo:rustc-link-lib=gmp");
    }
}
