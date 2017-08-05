//! The builder takes care of compiling mods and their
//! libraries in a reliable manner.
//!
//! Builder does all its work in a special directory.
//! This directory must contain a `lib` folder with all
//! available libraries. A `Cargo.lock` for specifying
//! the versions of all crates used when building the game.
//! This is very important so that we can guarantee
//! compatability when mods send data in between each other.

#![feature(plugin)]
#![plugin(clippy)]

#[macro_use]
extern crate weaver;
extern crate cargo;
extern crate uuid;
extern crate toml;

use std::io::{self, Error, ErrorKind, Read, Write};
use std::fs;
use std::path::{PathBuf, Path};

use cargo::core::{Workspace, shell, MultiShell, Shell};
use cargo::ops::{self, CompileOptions};
use cargo::util::Config;

use uuid::Uuid;

use weaver::CityboundMod;
use weaver::modules::LoadingPackage;
use weaver::kay::ActorSystem;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BuildMode {
    Debug,
    Release,
}

#[derive(Clone, Debug)]
pub struct BuildOptions {
    /// The working directory for the build.
    pub home: PathBuf,

    /// Temporary name for package to build.
    pub name: String,

    /// Path of the package to build.
    pub path: PathBuf,

    pub mode: BuildMode,
}

pub struct BuildResult {
    pub module: PathBuf,
}

/// Compiles a mod according to build options.
/// All files will be placed in the home path of build options.
///
/// The caller needs to make sure that the home path contains
/// a lib folder with libraries.
pub fn build(options: &BuildOptions) -> io::Result<BuildResult> {
    create_workspace(options)?;
    clean_mod_dir(&options.home)?;
    link_mod(options, &options.path)?;
    let _guard = rename_mod(options)?;

    let shell_config = shell::ShellConfig {
        color_config: shell::ColorConfig::Auto,
        tty: false,
    };
    let multi_shell = MultiShell::new(Shell::create(|| Box::new(io::stdout()), shell_config),
                                      Shell::create(|| Box::new(io::stdout()), shell_config),
                                      shell::Verbosity::Normal);
    let config = Config::new(multi_shell, options.home.clone(), options.cargo_home()?);
    let workspace = Workspace::new(&options.cargo_manifest_path()?, &config)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("{:?}", err)))?;
    let compile_options = CompileOptions {
        config: &config,
        jobs: None,
        target: None,
        features: &[],
        all_features: false,
        no_default_features: false,
        spec: &[],
        filter: ops::CompileFilter::Only {
            lib: true,
            bins: &[],
            examples: &[],
            tests: &[],
            benches: &[],
        },
        release: options.mode == BuildMode::Release,
        mode: ops::CompileMode::Build,
        message_format: ops::MessageFormat::Human,
        target_rustdoc_args: None,
        target_rustc_args: None,
    };

    match ops::compile(&workspace, &compile_options) {
        Ok(result) => {
            let module = result.libraries
                .iter()
                .filter(|&(id, _)| id.name() == options.name)
                .map(|(_, lib)| lib[0].1.clone())
                .next()
                .unwrap();
            Ok(BuildResult { module: module })
        }
        Err(err) => Err(io::Error::new(io::ErrorKind::Other, format!("{:?}", err))),
    }
}

fn create_workspace(options: &BuildOptions) -> io::Result<()> {
    let path = {
        let mut path = options.home.clone();
        path.push("Cargo.toml");
        path
    };

    let mut f = fs::File::create(path)?;
    write!(&mut f,
           r#"[workspace]
              members = [
                  "./mods/{}",

                  "./lib/allocators",
                  "./lib/chunked",
                  "./lib/compact",
                  "./lib/compact_macros",
                  "./lib/descartes",
                  "./lib/kay",
                  "./lib/kay_macros",
                  "./lib/monet",
                  "./lib/weaver",
              ]
           "#,
           &options.name)?;
    Ok(())
}

fn clean_mod_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = {
        let mut path = path.as_ref().to_owned();
        path.push("mods");
        path
    };

    fs::create_dir_all(&path)?;
    for dir in fs::read_dir(&path)? {
        let path = dir?.path();
        fs::remove_file(path)?;
    }

    Ok(())
}

fn link_mod<P: AsRef<Path>>(options: &BuildOptions, src: P) -> io::Result<()> {
    let dst = {
        let mut dst = options.home.clone();
        dst.push("mods");
        dst.push(&options.name);
        dst
    };

    let src = src.as_ref().canonicalize()?;

    #[cfg(unix)]
    let res = ::std::os::unix::fs::symlink(&src, &dst);

    #[cfg(windows)]
    let res = ::std::os::windows::fs::symlink_dir(&src, &dst);

    res
}

#[must_use]
#[derive(Debug)]
struct TemporaryEditGuard {
    temp: PathBuf,
    original: PathBuf,
}

impl Drop for TemporaryEditGuard {
    fn drop(&mut self) {
        match fs::copy(&self.original, &self.temp) {
            Ok(_) => (),
            Err(err) => println!("error when copying temporary file {:?}: {:?}", self, err),
        }

        match fs::remove_file(&self.original) {
            Ok(_) => (),
            Err(err) => println!("error when removing temporary file {:?}: {:?}", self, err),
        }
    }
}

fn rename_mod(options: &BuildOptions) -> io::Result<TemporaryEditGuard> {
    let temp = options.cargo_manifest_path()?;
    let original = {
        let mut manifest = options.cargo_manifest_path()?;
        manifest.set_extension("toml~");
        manifest
    };

    if original.exists() {
        panic!("an og exists plz send help! {:?}", original);
    }

    fs::copy(&temp, &original)?;

    let mut table = {
        let mut s = String::new();
        fs::File::open(&temp)?.read_to_string(&mut s)?;

        let mut parser = toml::Parser::new(&s);
        match parser.parse() {
            Some(table) => toml::Value::Table(table),
            None => panic!("invalid toml {:?}, {:?}", &temp, parser.errors),
        }
    };

    match table.lookup_mut("package.name") {
        Some(&mut toml::Value::String(ref mut name)) => {
            *name = options.name.clone();
        }
        _ => panic!("Cargo.toml must contain [package] and name, {:?}", &temp),
    }

    fs::File::create(&temp)?.write_all(table.to_string().as_bytes())?;

    Ok(TemporaryEditGuard {
        temp: temp,
        original: original,
    })
}

impl BuildOptions {
    pub fn target_dir(&self) -> io::Result<PathBuf> {
        let mut target = self.home.clone();
        target.push("target");
        Ok(target)
    }

    pub fn cargo_home(&self) -> io::Result<PathBuf> {
        let mut cargo = self.home.clone();
        cargo.push("cargo");
        Ok(cargo)
    }

    pub fn cargo_manifest_path(&self) -> io::Result<PathBuf> {
        let mut path = self.home.clone();
        path.push("mods");
        path.push(&self.name);
        path.push("Cargo.toml");

        Ok(path)
    }

    pub fn manifest_path(&self) -> io::Result<PathBuf> {
        let mut path = self.path.clone();
        path.push("Mod.toml");

        if path.is_file() {
            Ok(path)
        } else if path.exists() {
            Err(Error::new(ErrorKind::NotFound,
                           format!("the manifest 'Mod.toml' was not found in {:?}", &path)))
        } else {
            Err(Error::new(ErrorKind::NotFound,
                           format!("the manifest is not a file {:?}", &path)))
        }
    }
}

struct BuilderMod;

impl CityboundMod for BuilderMod {
    fn setup(_system: &mut ActorSystem) -> BuilderMod {
        println!("builder was loaded!");
        BuilderMod
    }

    fn dependant_loading(&mut self,
                         loading: &mut LoadingPackage,
                         _system: &mut ActorSystem)
                         -> Result<(), Box<::std::error::Error>> {
        println!("detected {:?}", loading);
        Ok(())
    }
}

register_module! {
    module: BuilderMod,
}
