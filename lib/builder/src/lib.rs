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

use std::io::{self, Error, ErrorKind, Write};
use std::fs;
use std::path::{PathBuf, Path};

use cargo::core::{Workspace, shell, MultiShell, Shell};
use cargo::ops::{self, CompileOptions};
use cargo::util::Config;

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

    /// Name of the package to build.
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
           "[workspace]\nmembers=['./mods/{}']\n",
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

    pub fn output_path(&self) -> io::Result<PathBuf> {
        let mut path = self.target_dir()?;
        match self.mode {
            BuildMode::Debug => path.push("debug"),
            BuildMode::Release => path.push("release"),
        }
        path.push(&self.lib_path());
        let path = path.canonicalize()?;

        if !path.is_file() {
            Err(io::Error::new(io::ErrorKind::NotFound,
                               format!("could not find {:?}", path)))
        } else {
            Ok(path)
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    fn lib_path(&self) -> String {
        format!("deps/lib{}.so", &self.name)
    }

    #[cfg(target_os = "macos")]
    fn lib_path(&self) -> String {
        format!("deps/lib{}.dylib", &self.name)
    }

    #[cfg(windows)]
    fn lib_path(name: &str) -> String {
        format!("{}.dll", name)
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
                         -> Result<(), String> {
        println!("detected {:?}", loading);
        Ok(())
    }
}

register_mod! {
    cb_mod: BuilderMod,
}
