
use std::env;
use std::fs;
use std::process::Command;
use std::path::PathBuf;

fn main() {
    build("mymod");
}

fn build(name: &str) {
    println!("cargo:rerun-if-changed=./lib/{}", name);

    let manifest_path = format!("./lib/{}/Cargo.toml", name);
    let target_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/lib/.target");
    let dbg_or_rel = debug_or_release();

    let mut args = vec!["build", "--manifest-path", &manifest_path];
    if dbg_or_rel == "release" {
        args.push("--release");
    }

    let output = Command::new("cargo")
        .env("CARGO_HOME", concat!(env!("CARGO_MANIFEST_DIR"), "/.cargo"))
        .env("CARGO_TARGET_DIR", target_dir)
        .args(&args)
        .status().unwrap();
    if !output.success() {
        panic!("compilation failed");
    }

    fs::create_dir_all(&format!("./target/{}/mods/{}", dbg_or_rel, name)).unwrap();

    fs::copy(&format!("{}/{}/{}", target_dir, dbg_or_rel, lib_path(name)),
             &format!("./target/{}/mods/{}/{}.module", dbg_or_rel, name, name)).unwrap();
    fs::copy(&format!("./lib/{}/Mod.toml", name),
             &format!("./target/{}/mods/{}/Mod.toml", dbg_or_rel, name)).unwrap();
}

fn debug_or_release() -> &'static str {
    // Apparently cargo does not give us access to ./target/(debug|release)/
    // So lets try to get to the base dir.
    let mut out_dir: PathBuf = env::var("OUT_DIR").unwrap().into();
    for _ in 0..3 {
        out_dir.pop();
    }

    if out_dir.ends_with("debug") {
        "debug"
    } else if out_dir.ends_with("release") {
        "release"
    } else {
        panic!("out dir does not seem to be in debug or release, {:?}", out_dir);
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn lib_path(name: &str) -> String {
    format!("deps/lib{}.so", name)
}

#[cfg(target_os = "macos")]
fn lib_path(name: &str) -> String {
    format!("deps/lib{}.dylib", name)
}

#[cfg(windows)]
fn lib_path(name: &str) -> String {
    format!("{}.dll", name)
}
