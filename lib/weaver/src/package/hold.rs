
use std::collections::HashSet;
use std::io;
use std::fs;
use std::path::Path;

use ::Package;
use ::package::{Ident, PackagePath};

#[derive(Debug, Default)]
pub struct PackageHold {
    packages: HashSet<Package>,
}

impl PackageHold {
    pub fn new() -> PackageHold {
        PackageHold::default()
    }

    fn add_package<P>(&mut self,
                      path: P,
                      namespace: &[&str],
                      replace_existing: bool)
                      -> io::Result<()>
        where P: AsRef<Path>
    {
        let package = Package::read_package_manifest(path, namespace)?;
        if self.packages.contains(&package) && !replace_existing {
            println!("INFO: package collision '{}'\n  {:?}\n  {:?}",
                     package.ident(),
                     package.path(),
                     self.packages.get(&package).unwrap().path());
        } else {
            assert!(self.packages.insert(package));
        }
        Ok(())
    }

    pub fn add_packages<P>(&mut self,
                           path: P,
                           namespace: &[&str],
                           replace_existing: bool)
                           -> io::Result<()>
        where P: AsRef<Path>
    {
        for dir in fs::read_dir(path)? {
            let dir = dir?;
            let path = dir.path();
            if PackageHold::is_mod_dir(&path) {
                self.add_package(path, namespace, replace_existing)?;
            } else {
                let f = dir.file_name();
                let s = f.to_str().unwrap();
                let mut ns = namespace.to_vec();
                ns.push(s);
                self.add_packages(&path, &ns, replace_existing)?;
            }
        }
        Ok(())
    }

    fn is_mod_dir<P: AsRef<Path>>(path: P) -> bool {
        let mut manifest = path.as_ref().to_owned();
        manifest.push("Mod.toml");
        manifest.is_file()
    }

    pub fn get(&self, ident: &Ident) -> Option<&Package> {
        self.packages.get(ident)
    }

    pub fn resolve<'a>(&'a self, path: &'a PackagePath) -> Resolve<'a> {
        Resolve {
            hold: self,
            path: path,
        }
    }
}

pub struct Resolve<'a> {
    hold: &'a PackageHold,
    path: &'a PackagePath,
}

impl<'a> Resolve<'a> {
    pub fn latest(self) -> Option<&'a Package> {
        self.iter().next()
    }

    pub fn iter(self) -> impl Iterator<Item = &'a Package> {
        let path = self.path.clone();
        self.hold
            .packages
            .iter()
            .filter(move |package| PackagePath::from(package.ident().clone()) == path)
    }
}
