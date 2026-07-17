use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let current_dir = env::current_dir().unwrap();
    let src_path = current_dir.join("src/video_renderer.zig");

    // zig build-lib コマンドの実行 (OUT_DIRで実行)
    let status = Command::new("zig")
        .current_dir(&out_dir)
        .args(&[
            "build-lib",
            src_path.to_str().unwrap(),
            "-O", "ReleaseFast",
        ])
        .status()
        .expect("Failed to execute zig build-lib. Make sure Zig is installed.");

    if !status.success() {
        panic!("Zig compilation failed");
    }

    // Cargoにリンク設定を通知
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=video_renderer");
    println!("cargo:rerun-if-changed=src/video_renderer.zig");
}
