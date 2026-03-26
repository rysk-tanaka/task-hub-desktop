fn main() {
    tauri_build::build();

    #[cfg(target_os = "macos")]
    {
        swift_rs::SwiftLinker::new("14")
            .with_package("AppleIntelligence", "./apple-intelligence/")
            .link();

        // Swift Concurrency ランタイム dylib を実行時に解決できるよう rpath を追加
        // xcode-select -p で動的にパスを解決し、標準外の Xcode インストールにも対応
        if let Ok(output) = std::process::Command::new("xcode-select")
            .arg("-p")
            .output()
        {
            if output.status.success() {
                if let Ok(developer_dir) = String::from_utf8(output.stdout) {
                    let developer_dir = developer_dir.trim();
                    if !developer_dir.is_empty() {
                        println!(
                            "cargo:rustc-link-arg=-Wl,-rpath,{developer_dir}/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx"
                        );
                    }
                }
            }
        }
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    }
}
