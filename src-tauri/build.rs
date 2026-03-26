fn main() {
    tauri_build::build();

    #[cfg(target_os = "macos")]
    {
        swift_rs::SwiftLinker::new("14")
            .with_package("AppleIntelligence", "./apple-intelligence/")
            .link();

        // Swift Concurrency ランタイム dylib を実行時に解決できるよう rpath を追加
        println!("cargo:rustc-link-arg=-Wl,-rpath,/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx");
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    }
}
