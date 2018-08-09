extern crate descartes;
extern crate compact;
#[macro_use]
extern crate compact_macros;
extern crate itertools;
extern crate lyon_tessellation;

mod mesh;

pub use mesh::{Mesh, Vertex, Instance};
