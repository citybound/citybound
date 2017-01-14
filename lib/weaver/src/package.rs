
use semver::{SemVerError, Version};
use serde::de::{Deserializer, Deserialize, Error, Visitor};

use std::collections::HashMap;
use std::ascii::AsciiExt;

/// Contains all data about the mod.
#[derive(Clone, Debug, Deserialize)]
pub struct PackageDesc {
    #[serde(rename="mod")]
    pub mod_info: ModInfo,
    pub dependencies: HashMap<String, Dependency>,
}

/// Basic information about the mod
#[derive(Clone, Debug, Deserialize)]
pub struct ModInfo {
    /// Name of the mod, should only contain characters a-z, 0-9 and hyphens.
    /// Will be used by other mods to specify their dependencies.
    pub name: String,

    /// The name of the crate which will be shown to users
    #[serde(rename="pretty-name")]
    pub pretty_name: String,

    /// A short description of the mod.
    #[serde(default)]
    pub description: String,

    #[serde(deserialize_with="de_version")]
    pub version: Version,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Dependency {
    #[serde(deserialize_with="de_version")]
    pub version: Version,
}

#[derive(Clone, Debug)]
pub struct Package {
    ident: String,

    name: String,
    pretty_name: String,
    description: String,
    version: Version,

    dependencies: HashMap<String, Dependency>,
}

impl Package {
    pub fn new(desc: PackageDesc) -> Result<Package, String> {
        if !Package::name_is_valid(&desc.mod_info.name) {
            return Err(format!("package name can only contain characters a-z, 0-9 and hyphens, \
                                \"{}\"",
                               &desc.mod_info.name));
        }

        let ident = format!("{}:{}", &desc.mod_info.name, &desc.mod_info.version);
        Ok(Package {
            ident: ident,

            name: desc.mod_info.name,
            pretty_name: desc.mod_info.pretty_name,
            description: desc.mod_info.description,
            version: desc.mod_info.version,

            dependencies: desc.dependencies,
        })
    }

    fn name_is_valid(name: &str) -> bool {
        match name.chars().next() {
            Some(c) if c.is_digit(10) => return false,
            None => return false,
            _ => (),
        }

        for c in name.chars() {
            if !(c.is_alphanumeric() || c == '-') || !c.is_lowercase() || !c.is_ascii() {
                return false;
            }
        }

        true
    }

    #[inline]
    pub fn ident(&self) -> &str {
        &self.ident
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn pretty_name(&self) -> &str {
        &self.pretty_name
    }

    #[inline]
    pub fn description(&self) -> &str {
        &self.description
    }

    #[inline]
    pub fn version(&self) -> &Version {
        &self.version
    }
}

fn de_version<D>(deserializer: &mut D) -> Result<Version, D::Error>
    where D: Deserializer
{
    VersionDeserialize::deserialize(deserializer).map(|v| v.0)
}

struct VersionVisitor;
struct VersionDeserialize(Version);

impl VersionDeserialize {
    fn parse<E>(v: &str) -> Result<VersionDeserialize, E>
        where E: Error
    {
        Version::parse(v)
            .map_err(|err| {
                let SemVerError::ParseError(v) = err;
                Error::custom(v)
            })
            .map(VersionDeserialize)
    }
}

impl Visitor for VersionVisitor {
    type Value = VersionDeserialize;

    fn visit_str<E>(&mut self, v: &str) -> Result<Self::Value, E>
        where E: Error
    {
        VersionDeserialize::parse(v)
    }

    fn visit_string<E>(&mut self, v: String) -> Result<Self::Value, E>
        where E: Error
    {
        VersionDeserialize::parse(&v)
    }
}

impl Deserialize for VersionDeserialize {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: Deserializer
    {
        deserializer.deserialize_str(VersionVisitor)
    }
}
