use std::fs::{File, metadata};
use std::io::Read;
use std::io::Write;
use std::process::Command;

extern crate kay_codegen;
use kay_codegen::generate;

fn main() {
    if let Ok(src_meta) = metadata("src/renderer/mod.rs") {
        let regenerate = match metadata("src/renderer/kay_auto.rs") {
            Ok(auto_meta) => src_meta.modified().unwrap() > auto_meta.modified().unwrap(),
            _ => true,
        };

        if regenerate {
            let auto_file = if let Ok(ref mut file) = File::open("src/renderer/mod.rs") {
                let mut file_str = String::new();
                file.read_to_string(&mut file_str).unwrap();
                generate(&file_str)
            } else {
                panic!("couldn't load");
            };

            if let Ok(ref mut file) = File::create("src/renderer/kay_auto.rs") {
                file.write_all(auto_file.as_bytes()).unwrap();
            }

            let _ = Command::new("rustfmt")
                .arg("--write-mode")
                .arg("overwrite")
                .arg("src/renderer/kay_auto.rs")
                .spawn();
        }
    }
}