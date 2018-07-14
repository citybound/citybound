use std::fs::{File, OpenOptions, create_dir_all};
use std::path::PathBuf;

#[derive(Copy, Clone)]
pub struct Environment {
    pub name: &'static str,
    pub version: &'static str,
    pub author: &'static str,
}

impl Environment {}
