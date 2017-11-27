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
                   GrouperIndividualID, MSG_GrouperIndividual_render_to_grouper};
pub use renderer::{setup, Renderer, RendererID, Renderable, RenderableID, TargetProvider,
                   TargetProviderID, MSG_TargetProvider_submitted, Movement, EyeListener,
                   EyeListenerID, MSG_EyeListener_eye_moved, MSG_Renderable_setup_in_scene,
                   MSG_Renderable_render_to_scene, ProjectionRequester, ProjectionRequesterID,
                   MSG_ProjectionRequester_projected_3d};
pub use render_context::RenderContext;
pub use scene::{Eye, Scene, SceneDescription};
