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
extern crate lazy_static;

mod geometry;
mod renderer;
mod render_context;
mod scene;
mod thing;

pub use glium::backend::glutin_backend::GlutinFacade;

pub use geometry::{Batch, Vertex, Instance};
pub use renderer::{setup, Renderer, SetupInScene, RenderToScene, Control, Submitted, RendererID,
                   Movement, MoveEye, EyeMoved, Project2dTo3d, Projected3d};
pub use render_context::RenderContext;
pub use scene::{Eye, Scene};
pub use thing::Thing;
