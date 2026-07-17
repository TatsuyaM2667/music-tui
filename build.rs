use std::process::Command;
use std::env;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let src_dir = env::current_dir().unwrap().join("src");

    // Compile Odin with PIC (required for Rust's PIE executables) and no entry point (library mode)
    let status = Command::new("odin")
        .arg("build")
        .arg(src_dir.join("braille_renderer.odin"))
        .arg("-file")
        .arg("-build-mode:obj")
        .arg("-reloc-mode:pic")
        .arg("-no-entry-point")
        .arg(format!("-out:{}/braille.o", out_dir))
        .status()
        .expect("Failed to execute odin compiler. Is odin installed?");

    if !status.success() {
        panic!("Odin compilation failed");
    }

    // Archive all Odin .o files (runtime + user code) into libbraille.a
    let ar_status = Command::new("sh")
        .arg("-c")
        .arg(format!("cd {} && ar rcs libbraille.a braille*.o", out_dir))
        .status()
        .expect("Failed to run ar");

    if !ar_status.success() {
        panic!("Failed to archive Odin object files");
    }

    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=braille");
    println!("cargo:rerun-if-changed=src/braille_renderer.odin");
}
