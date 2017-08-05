
use std::process::Command;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    // Gets the version of rustc and writes it to a file.

    let rustc = env::var("RUSTC").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("rustc_version.rs");
    let mut f = File::create(&dest_path).unwrap();

    let output = Command::new(rustc)
        .arg("--version")
        .output()
        .expect("failed to execute rustc");

    assert!(output.status.success(),
            "rustc quit with status code {}",
            output.status);

    let version = String::from_utf8(output.stdout).expect("stdout of rustc is not vaild utf-8");
    let version = format!(r#"b"{}\0""#, version.trim());

    f.write_all(version.as_bytes()).unwrap();
}
