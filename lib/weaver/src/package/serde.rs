
use semver::{SemVerError, Version};
use serde::de::{Deserializer, Deserialize, Error, Visitor};

pub fn de_version<D>(deserializer: &mut D) -> Result<Version, D::Error>
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
