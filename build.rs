extern crate builder;

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use builder::{BuildMode, BuildOptions};

const SYSTEM_MODS: &'static [&'static str] = &["builder", "mymod"];

fn main() {
    let home_dir = env::var("OUT_DIR").unwrap();
    let (target_dir, mode) = target_dir();
    link_libs(&home_dir).expect("couldn't link lib");

    for &mod_ in SYSTEM_MODS {
        let opts = BuildOptions {
            home: home_dir.to_owned().into(),
            name: mod_.into(),
            path: format!("./lib/{}", mod_).into(),
            mode: mode,
        };
        let result = builder::build(&opts).expect(&format!("mod failed to build {:?}", mod_));

        let mut dir = target_dir.clone();
        dir.push("mods");
        dir.push(mod_);

        fs::create_dir_all(&dir).expect("couldn't create dir");

        {
            let mut module = dir.clone();
            module.push(format!("{}.module", mod_));
            fs::copy(result.module, module).expect("couldn't copy module");
        }

        {
            let mut manifest = dir.clone();
            manifest.push("Mod.toml");
            fs::copy(opts.manifest_path().unwrap(), manifest).expect("couldn't copy manifest");
        }
    }

    depend_src_dir("lib");
}

fn link_libs<P: AsRef<Path>>(home: P) -> io::Result<()> {
    let dst = {
        let mut dst = home.as_ref().to_owned();
        dst.push("lib");
        dst
    };

    let src = {
        let mut src = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        src.push("lib");
        src
    };

    if dst.exists() {
        fs::remove_file(&dst)?;
    }

    #[cfg(unix)]
    let res = ::std::os::unix::fs::symlink(src, dst);

    #[cfg(windows)]
    let res = ::std::os::windows::fs::symlink_dir(src, dst);

    res
}

fn depend_src_dir(name: &str) {
    fn depend_rec<P: AsRef<Path>>(path: P) -> io::Result<()> {
        for dir in fs::read_dir(path)? {
            let path = dir?.path();
            if path.is_file() {
                println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
            } else if path.is_dir() {
                depend_rec(&path)?;
            }
        }

        Ok(())
    }

    depend_rec(&format!("./{}", name)).unwrap();
}

fn target_dir() -> (PathBuf, BuildMode) {
    // Apparently cargo does not give us access to ./target/(debug|release)/
    // So lets try to get there our selves.
    let mut out_dir: PathBuf = env::var("OUT_DIR").unwrap().into();
    for _ in 0..3 {
        out_dir.pop();
    }

    let mode;
    if out_dir.ends_with("debug") {
        mode = BuildMode::Debug;
    } else if out_dir.ends_with("release") {
        mode = BuildMode::Release;
    } else {
        panic!("out dir does not seem to be in debug or release, {:?}",
               out_dir);
    }

    (out_dir, mode)
}
