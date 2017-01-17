#![feature(plugin, conservative_impl_trait)]
#![plugin(clippy)]
#![allow(no_effect, unnecessary_operation)]

extern crate descartes;
#[macro_use]
pub extern crate glium;
extern crate kay;
extern crate compact;
#[macro_use]
extern crate compact_macros;
extern crate rusttype;
extern crate fnv;
extern crate unicode_normalization;
#[macro_use]
extern crate lazy_static;

use kay::{ActorSystem, Individual};

mod geometry;
mod renderer;
mod render_context;
mod scene;
mod thing;
mod text;

pub use glium::backend::glutin_backend::GlutinFacade;

pub use geometry::{Batch, Vertex, Instance};
pub use renderer::{Renderer, SetupInScene, RenderToScene, Control, AddBatch, AddInstance,
                   AddSeveralInstances, Movement, MoveEye, EyeMoved, AddEyeListener, AddDebugText,
                   UpdateThing, Project2dTo3d, Projected3d};
pub use render_context::RenderContext;
pub use scene::{Eye, Scene};
pub use thing::Thing;
pub use text::{TextRenderer, TextVertex, Font, FontBank, RichText, FontDescription, Formatting,
               Glyph, GlyphIter};

pub fn setup(system: &mut ActorSystem, renderer: Renderer) {
    system.add_individual(renderer);
    Renderer::handle_critically::<Control>();
    Renderer::handle_critically::<AddBatch>();
    Renderer::handle_critically::<AddInstance>();
    Renderer::handle_critically::<AddSeveralInstances>();
    Renderer::handle_critically::<MoveEye>();
    Renderer::handle_critically::<AddEyeListener>();
    Renderer::handle_critically::<AddDebugText>();
    Renderer::handle_critically::<UpdateThing>();
    Renderer::handle_critically::<Project2dTo3d>();

    Renderer::id() << Control::Setup;
}
