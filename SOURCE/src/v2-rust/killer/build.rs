fn main() {
    // Only link kore_ffi when the feature is requested.
    if std::env::var("CARGO_FEATURE_KORE_ENGINE").is_ok() {
        let kore_dir = std::env::var("KORE_FFI_DIR").unwrap_or_else(|_| {
            r"C:\Users\skathera\Downloads\KoreRepo\target\release".to_string()
        });
        println!("cargo:rustc-link-search=native={}", kore_dir);
        println!("cargo:rustc-link-lib=static=kore_ffi");
        if cfg!(target_os = "windows") {
            println!("cargo:rustc-link-lib=dylib=ws2_32");
            println!("cargo:rustc-link-lib=dylib=userenv");
            println!("cargo:rustc-link-lib=dylib=bcrypt");
            println!("cargo:rustc-link-lib=dylib=ntdll");
        }
    }
}
