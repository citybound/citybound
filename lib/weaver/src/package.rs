
use semver::{SemVerError, Version};
use serde::de::{Deserializer, Deserialize, Error, Visitor};

/// Contains all data about the mod.
#[derive(Deserialize)]
pub struct Package {
    #[serde(rename="mod")]
    pub mod_info: ModInfo,
}

/// Basic information about the mod
#[derive(Deserialize)]
pub struct ModInfo {
    /// Name of the mod, should only contain characters a-z, - and _.
    /// Will be used by other mods to specify dependencies.
    pub name: String,

    /// The name of the crate which will be shown to users
    #[serde(rename="pretty-name")]
    pub pretty_name: String,

    /// A short description of the mod.
    #[serde(default)]
    pub description: String,

    /// Semver version of the mod.
    #[serde(deserialize_with="de_version")]
    pub version: Version,
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
