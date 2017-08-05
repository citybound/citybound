
use std::str::FromStr;
use std::fmt;

use regex::Regex;

use semver::Version;

use serde::{de, Deserialize, Deserializer};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ident {
    owner: String,
    name: String,
}

impl Ident {
    fn new<O, N>(owner: O, name: N) -> Ident
        where O: Into<String>,
              N: Into<String>
    {
        Ident {
            owner: owner.into(),
            name: name.into(),
        }
    }

    #[inline]
    pub fn owner(&self) -> &str {
        &self.owner
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    fn is_valid(name: &str) -> bool {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").unwrap();
        }
        RE.is_match(name)
    }
}

impl FromStr for Ident {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Ident, Self::Err> {
        let path = s.split('/').collect::<Vec<_>>();
        match path.as_slice() {
            &[owner, name] if Ident::is_valid(owner) && Ident::is_valid(name) => {
                Ok(Ident::new(owner, name))
            }
            _ => Err("ident is invalid"),
        }
    }
}

impl<'de> Deserialize<'de> for Ident {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(de::Error::custom)
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}

/// Unique identity of a mod.
///
/// This is used to uniquely identify a mod inside the modding system.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UniqueIdent {
    pub path: Ident,
    pub version: Version,
}

impl UniqueIdent {
    pub fn new(path: Ident, version: Version) -> UniqueIdent {
        UniqueIdent {
            path: path,
            version: version,
        }
    }
}

impl fmt::Display for UniqueIdent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.path, self.version)
    }
}
