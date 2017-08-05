
use std::path::{Path, PathBuf};

use Package;

/// This is the description sent to dependants of this package,
/// before the module is actually loaded.
#[derive(Debug)]
pub struct LoadingPackage<'a> {
    pub package: &'a Package,
    pub path: PathBuf,
}

impl<'a> LoadingPackage<'a> {
    pub fn new(package: &'a Package, path: PathBuf) -> LoadingPackage<'a> {
        LoadingPackage {
            package: package,
            path: path,
        }
    }
}
