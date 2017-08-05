
use std::collections::{HashSet, HashMap};
use std::error::Error;
use std::io;
use std::fs;
use std::path::{Path, PathBuf};

use semver::VersionReq;

use modules::{ModuleDef};
use packages::{Package, Ident, UniqueIdent};

#[derive(Default)]
pub struct PackageManager {
    external: HashMap<Package, PathBuf>,
    replace: HashMap<Package, PathBuf>,
    builtin: HashMap<Package, ModuleDef>,

    paths: HashMap<PathBuf, PackageType>,
}

enum PackageType {
    External,
    Replace,
}

impl PackageManager {
    pub fn new() -> PackageManager {
        PackageManager::default()
    }

    pub fn add_external_packages<P>(&mut self,
                                    path: P)
                                    -> Result<Vec<(PathBuf, Box<Error>)>, Box<Error>>
        where P: AsRef<Path>
    {
        let mut errors = Vec::new();

        for dir in fs::read_dir(path)? {
            let dir = dir?;
            let path = dir.path();
            let owner = dir.file_name()
                .into_string()
                .map_err(|_| format!("Directory name is not UTF-8, {:?}", path))?;

            if let Ok(errs) = self.add_packages(path, owner, PackageType::External)? {
                errors.extend(errs);
            }
        }

        Ok(errors)
    }

    pub fn add_replace_packages<P>(&mut self,
                                   path: P)
                                   -> Result<Vec<(PathBuf, Box<Error>)>, Box<Error>>
        where P: AsRef<Path>
    {
        self.add_packages(path, "citybound", PackageType::Replace)
    }

    pub fn add_builtin_package(&mut self,
                               manifest: Package,
                               module: ModuleDef)
                               -> Result<(), Box<Error>> {
        self.builtin.insert(manifest, module);
        Ok(())
    }

    fn add_package<P>(&mut self, path: P, owner: &str, pt: PackageType) -> Result<(), Box<Error>>
        where P: AsRef<Path>
    {
        let package = Package::read(path)?;
        let old = match pt {
            PackageType::External => self.external.insert(package, path.to_path_buf()),
            PackageType::Replace => self.replace.insert(package, path.to_path_buf()),
        };

        if let Some(old) = old {
            println!("Duplicate package at {:?} and {:?}", path, old);
        }

        Ok(())
    }

    fn add_packages<P>(&mut self,
                       path: P,
                       owner: &str,
                       pt: PackageType)
                       -> Result<Vec<(PathBuf, Box<Error>)>, Box<Error>>
        where P: AsRef<Path>
    {
        // Keep track of any errors from specific packages.
        let mut errors = Vec::new();

        for dir in fs::read_dir(path)? {
            let path = dir?.path();

            if let Err(err) = self.add_package(path, owner, pt) {
                errors.push((path.to_path_buf(), err));
            }
        }

        Ok(errors)
    }

    pub fn resolve<I>(&self, deps: I) -> HashSet<ResolvedPackage>
        where I: IntoIterator<Item = UniqueIdent>
    {
        let mut resolved = HashSet::new();

        for ident in deps.into_iter() {}

        resolved
    }

    fn query_package(&self, ident: UniqueIdent) -> Option<ResolvedPackage> {
        unimplemented!();
    }

    fn query_packages(&self, ident: Ident, version_req: VersionReq) -> HashSet<ResolvedPackage> {
        fn query_package_map<'a, I, T>(map: I,
                                       ident: Ident,
                                       version_req: VersionReq)
                                       -> impl Iterator<Item = ResolvedPackage<'a>>
            where I: IntoIterator<Item = &'a T>,
                  T: Into<ResolvedPackage<'a>> + 'a
        {
            map.into_iter()
                .map(|a| a.into())
                .filter(|resolved| {
                            resolved.package().info.name == ident &&
                            version_req.matches(resolved.package().info.version)
                        })
        }

        let mut set = HashSet::new();
        set.extend(query_package_map(self.replace.iter(), ident, version_req));
        set
    }
}

pub enum ResolvedPackage<'r> {
    External(&'r Package, &'r Path),
    Builtin(&'r Package, ModuleDef),
}

impl<'r> ResolvedPackage<'r> {
    fn package(&self) -> &Package {
        match *self {
            ResolvedPackage::External(package, _) => package,
            ResolvedPackage::Builtin(package, _) => package,
        }
    }
}

impl<'r> From<&'r (Package, PathBuf)> for ResolvedPackage<'r> {
    fn from(&(ref package, ref path): &'r (Package, PathBuf)) -> ResolvedPackage<'r> {
        ResolvedPackage::External(package, path.as_str())
    }
}

impl<'r> From<&'r (Package, ModuleDef)> for ResolvedPackage<'r> {
    fn from(&(ref package, ref module): &'r (Package, ModuleDef)) -> ResolvedPackage<'r> {
        ResolvedPackage::Builtin(package, module)
    }
}

impl<'r> ::std::cmp::PartialEq for ResolvedPackage<'r> {
    #[inline]
    fn eq(&self, other: &ResolvedPackage) -> bool {
        self.package() == other.package()
    }
}

impl<'r> ::std::cmp::Eq for ResolvedPackage<'r> {}

impl<'r> ::std::hash::Hash for ResolvedPackage<'r> {
    #[inline]
    fn hash<H>(&self, state: &mut H)
        where H: ::std::hash::Hasher
    {
        self.package().hash(state);
    }
}
