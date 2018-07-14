// temporary fix for https://github.com/glium/glium/issues/1620
#![cfg_attr(feature = "cargo-clippy", allow(forget_copy))]

extern crate descartes;
extern crate kay;
extern crate compact;
#[macro_use]
extern crate compact_macros;
extern crate fnv;
extern crate itertools;
extern crate lyon_tessellation;

mod mesh;
mod mesh_actors;
mod renderer;
mod scene;

pub use mesh::{Mesh, Vertex, Instance};
pub use mesh_actors::{Grouper, GrouperID, GrouperIndividual, GrouperIndividualID};
pub use renderer::{setup, Renderer, RendererID, Renderable, RenderableID, TargetProvider,
                   TargetProviderID, Movement, EyeListener, EyeListenerID, ProjectionRequester,
ProjectionRequesterID};
pub use scene::{Eye, Scene, SceneDescription};
