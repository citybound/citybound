#![feature(conservative_impl_trait)]
#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

extern crate descartes;
#[macro_use]
pub extern crate glium;
extern crate kay;
extern crate compact;
#[macro_use]
extern crate compact_macros;
extern crate fnv;
extern crate itertools;

mod geometry;
mod renderer;
mod render_context;
mod scene;

pub use glium::backend::glutin::Display;

pub use geometry::{Geometry, Batch, Vertex, Instance, Grouper, GrouperID, GrouperIndividual,
                   GrouperIndividualID};
pub use renderer::{setup, Renderer, RendererID, Renderable, RenderableID, TargetProvider,
                   TargetProviderID, Movement, EyeListener, EyeListenerID, ProjectionRequester,
                   ProjectionRequesterID};
pub use render_context::RenderContext;
pub use scene::{Eye, Scene, SceneDescription};
