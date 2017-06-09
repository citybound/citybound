use std::fs::File;
use std::io::Read;
use std::io::Write;

extern crate kay_codegen;
use kay_codegen::generate;

fn main() {
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
}