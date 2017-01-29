
use semver::Version;

use std::str::FromStr;
use std::fmt;

/// Specifies the path of a package relative to the current package
/// or in absolute terms. Does not contain package version.
///
/// Paths with leading double forward slashes (`//`)
/// are considered absolute.
///
/// **Absolute:** `//system/mods/mymod`
///
/// **Relative:** `mods/mymod`, `mymod`
#[derive(Clone, Debug, PartialEq)]
pub struct PackagePath {
    is_absolute: bool,
    path: Vec<String>,
}

impl PackagePath {
    /// Resolves a package path into an identy.
    ///
    /// If self is absolute, then we simply use that path.
    /// Otherwise we try to resolve the path as follows:
    ///
    /// | self       | current                | result              |
    /// |------------|------------------------|---------------------|
    /// | mymod      | //system/mods/othermod | //system/mods/mymod |
    /// | libs/kay   | //system/mods/mymod    | //system/libs/kay   |
    pub fn into_ident(self, version: Version, current: &Ident) -> Option<Ident> {
        let path = if self.is_absolute() {
            self.path
        } else {
            if current.path.len() < self.path.len() {
                return None;
            }

            current.path[0..(current.path.len() - self.path.len())]
                .iter()
                .cloned()
                .chain(self.path.into_iter())
                .collect()
        };

        Some(Ident {
            path: path,
            version: version,
        })
    }

    #[inline]
    pub fn is_absolute(&self) -> bool {
        self.is_absolute
    }
}

impl From<Ident> for PackagePath {
    fn from(ident: Ident) -> PackagePath {
        PackagePath {
            is_absolute: true,
            path: ident.path.clone(),
        }
    }
}

impl FromStr for PackagePath {
    type Err = String;

    fn from_str(s: &str) -> Result<PackagePath, String> {
        let is_absolute = s.starts_with("//");
        let s = if is_absolute { &s[2..] } else { s };
        let path = s.split('/')
            .map(|s| s.to_owned())
            .collect();

        Ok(PackagePath {
            is_absolute: is_absolute,
            path: path,
        })
    }
}

/// Absolute identity of a mod.
///
/// This is used to uniquely identify a mod inside the modding system.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ident {
    path: Vec<String>,
    version: Version,
}

impl Ident {
    pub fn new(path: Vec<String>, version: Version) -> Ident {
        Ident {
            path: path,
            version: version,
        }
    }

    #[inline]
    pub fn name(&self) -> &str {
        self.path.last().as_ref().unwrap()
    }

    #[inline]
    pub fn version(&self) -> &Version {
        &self.version
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "/")?;
        for component in &self.path {
            write!(f, "/{}", &component)?;
        }
        write!(f, ":{}", self.version)
    }
}
