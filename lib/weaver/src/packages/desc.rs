
use std::collections::HashMap;
use std::io::{self, Read};
use std::error::Error;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use semver::{Version, VersionReq};

use serde::{de, Deserialize, Deserializer};

use toml;

use packages::{Ident, UniqueIdent};

/// A package.
#[derive(Clone, Debug, Deserialize)]
pub struct Package {
    #[serde(rename="package")]
    pub info: Info,
    pub dependencies: HashMap<Ident, Dependency>,
}

impl Package {
    pub fn unique_ident(&self) -> UniqueIdent {
        UniqueIdent::new(self.info.name, self.info.version)
    }

    /// Parses a package manifest from a string.
    pub fn parse(manifest: &str) -> Result<Package, Box<Error>> {
        let package = toml::from_str(manifest)?;
        Ok(package)
    }

    /// Reads the manifest from disk. Path needs to point to a package folder.
    pub fn read<P>(path: P) -> Result<Package, Box<Error>>
        where P: AsRef<Path>
    {
        let mut manifest = path.as_ref().to_owned();
        manifest.push("Package.toml");
        // TODO there should be a check here to make sure the casing is correct.

        let mut s = String::new();
        let mut file = File::open(&manifest)?;
        file.read_to_string(&mut s)?;

        Package::parse(&s)
    }
}

/// Basic information about a package.
#[derive(Clone, Debug, Deserialize)]
pub struct Info {
    /// Name of the mod, should only contain characters a-z, 0-9 and hyphens.
    /// Will be used by other mods to specify their dependencies.
    pub name: Ident,

    /// The name of the crate which will be shown to users
    #[serde(rename="pretty-name")]
    pub pretty_name: String,

    /// A short description of the mod.
    #[serde(default)]
    pub description: String,

    /// Authors of the mod.
    #[serde(default)]
    pub authors: String,

    /// Semver version of the mod.
    #[serde(deserialize_with = "deserialize_from_str")]
    pub version: Version,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Dependency {
    Managed {
        #[serde(deserialize_with = "deserialize_from_str")]
        version: VersionReq,
    },
    Git { git: String, branch: Option<String> },
}

fn deserialize_from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where D: Deserializer<'de>,
          T: FromStr,
          <T as FromStr>::Err: ::std::fmt::Display
{
    let s = String::deserialize(deserializer)?;
    FromStr::from_str(&s).map_err(de::Error::custom)
}

impl ::std::cmp::PartialEq for Package {
    #[inline]
    fn eq(&self, other: &Package) -> bool {
        self.unique_ident() == other.unique_ident()
    }
}

impl ::std::cmp::Eq for Package {}

impl ::std::hash::Hash for Package {
    #[inline]
    fn hash<H>(&self, state: &mut H)
        where H: ::std::hash::Hasher
    {
        self.unique_ident().hash(state);
    }
}
