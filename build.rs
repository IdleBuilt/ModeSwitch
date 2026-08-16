use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=res/app.rc");
    println!("cargo:rerun-if-changed=res/app.ico");
    println!("cargo:rerun-if-changed=res/dark.ico");
    println!("cargo:rerun-if-changed=res/light.ico");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_obj = Path::new(&out_dir).join("app_res.o");

    let status = Command::new("windres")
        .current_dir("res")
        .args(["app.rc", "-O", "coff", "-o"])
        .arg(&out_obj)
        .status()
        .expect("failed to launch windres (mingw-w64 binutils) - is it on PATH?");

    if !status.success() {
        panic!("windres failed to compile res/app.rc");
    }

    println!("cargo:rustc-link-arg-bins={}", out_obj.display());
}
