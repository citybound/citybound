//! These descriptions are useful when defining or deserializing mods.

use semver::Version;

use std::collections::HashMap;

use super::de_version;

/// Contains all data about a mod.
#[derive(Clone, Debug, Deserialize)]
pub struct PackageDesc {
    #[serde(rename="mod")]
    pub mod_info: ModInfo,
    pub dependencies: HashMap<String, DependencyDesc>,
}

/// Basic information about a mod.
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

    /// Semver version of the mod.
    #[serde(deserialize_with="de_version")]
    pub version: Version,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DependencyDesc {
    #[serde(deserialize_with="de_version")]
    pub version: Version,
}
