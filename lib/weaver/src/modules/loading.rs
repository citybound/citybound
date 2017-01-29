
use std::path::{Path, PathBuf};

use ::Package;

/// This is the description sent to dependants of this package,
/// before the module is actually loaded.
#[derive(Debug)]
pub struct LoadingPackage<'a> {
    package: &'a Package,
    module_path: PathBuf,
}

impl<'a> LoadingPackage<'a> {
    pub fn new(package: &'a Package, module_path: PathBuf) -> LoadingPackage<'a> {
        LoadingPackage {
            package: package,
            module_path: module_path,
        }
    }

    #[inline]
    pub fn package(&self) -> &Package {
        self.package
    }

    #[inline]
    pub fn module_path(&self) -> &Path {
        self.module_path.as_path()
    }

    #[inline]
    pub fn module_path_mut(&mut self) -> &mut PathBuf {
        &mut self.module_path
    }
}
