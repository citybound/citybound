//! Module containing helpers for loading, defining and working
//! with mods.

use toml;
use serde::Deserialize;

use std::io::{self, Read};
use std::fs::File;
use std::ascii::AsciiExt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub mod desc;
mod hold;
mod path;
mod serde;

use self::desc::PackageDesc;
pub use self::hold::PackageHold;
pub use self::path::{PackagePath, Ident};
use self::serde::de_version;

#[derive(Clone, Debug)]
pub struct Package {
    path: PathBuf,
    ident: Ident,

    pretty_name: String,
    description: String,

    dependencies: Vec<Dependency>,
}

impl Package {
    pub fn new<P>(path: P, namespace: &[&str], desc: PackageDesc) -> Result<Package, String>
        where P: AsRef<Path>
    {
        if !Package::name_is_valid(&desc.mod_info.name) {
            return Err(format!("package name can only contain characters a-z, 0-9 and hyphens, \
                                \"{}\"",
                               &desc.mod_info.name));
        }

        let ident = {
            let mut path = namespace.iter().map(|&s| s.to_owned()).collect::<Vec<_>>();
            path.push(desc.mod_info.name.to_owned());
            Ident::new(path, desc.mod_info.version)
        };

        let deps = desc.dependencies
            .into_iter()
            .map(|(n, d)| {
                /// How do we return errors from here?
                let ident = PackagePath::from_str(&n)
                    .unwrap()
                    .into_ident(d.version, &ident)
                    .unwrap();
                Dependency { ident: ident }
            })
            .collect();

        Ok(Package {
            path: path.as_ref().to_owned(),
            ident: ident,

            pretty_name: desc.mod_info.pretty_name,
            description: desc.mod_info.description,

            dependencies: deps,
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
    pub fn ident(&self) -> &Ident {
        &self.ident
    }

    #[inline]
    pub fn path(&self) -> &Path {
        &self.path
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
    #[allow(needless_lifetimes)]
    pub fn dependencies<'a>(&'a self) -> impl Iterator<Item = &'a Dependency> {
        self.dependencies.iter()
    }

    pub fn read_package_manifest<P>(path: P, namespace: &[&str]) -> io::Result<Package>
        where P: AsRef<Path>
    {
        let mut manifest = path.as_ref().to_owned();
        manifest.push("Mod.toml");
        // TODO: there should be a check here to make sure the casing is correct.

        let mut s = String::new();
        let mut file = File::open(&manifest)?;
        file.read_to_string(&mut s)?;

        let mut parser = toml::Parser::new(&s);
        let table;
        match parser.parse() {
            Some(t) => table = t,
            None => {
                for error in &parser.errors {
                    let (line, col) = parser.to_linecol(error.lo);
                    println!("TOML parsing error: {}\nLine: {} Col: {}",
                             &error.desc,
                             line,
                             col);
                }
                let err = io::Error::new(io::ErrorKind::InvalidData, "malformed toml");
                return Err(err);
            }
        }

        let mut decoder = toml::Decoder::new(toml::Value::Table(table));
        let desc = Deserialize::deserialize(&mut decoder)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        Package::new(path, namespace, desc)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }
}

impl ::std::borrow::Borrow<Ident> for Package {
    #[inline]
    fn borrow(&self) -> &Ident {
        self.ident()
    }
}

impl ::std::cmp::PartialEq for Package {
    #[inline]
    fn eq(&self, other: &Package) -> bool {
        self.ident() == other.ident()
    }
}

impl ::std::cmp::Eq for Package {}

impl ::std::hash::Hash for Package {
    #[inline]
    fn hash<H>(&self, state: &mut H)
        where H: ::std::hash::Hasher
    {
        self.ident().hash(state);
    }
}

#[derive(Clone, Debug)]
pub struct Dependency {
    ident: Ident,
}

impl Dependency {
    #[inline]
    pub fn ident(&self) -> &Ident {
        &self.ident
    }
}
