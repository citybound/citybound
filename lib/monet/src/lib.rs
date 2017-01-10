#![feature(plugin, conservative_impl_trait)]
#![plugin(clippy)]
#![allow(no_effect, unnecessary_operation)]

extern crate descartes;
#[macro_use]
pub extern crate glium;
extern crate kay;
#[macro_use]
extern crate kay_macros;
extern crate rusttype;
extern crate fnv;
extern crate unicode_normalization;
#[macro_use]
extern crate lazy_static;
extern crate image;

use kay::{ActorSystem, Individual};

mod geometry;
mod renderer;
mod render_context;
mod scene;
mod thing;
mod text;
mod sdf;

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
    system.add_unclearable_inbox::<Control, Renderer>();
    system.add_unclearable_inbox::<AddBatch, Renderer>();
    system.add_unclearable_inbox::<AddInstance, Renderer>();
    system.add_unclearable_inbox::<AddSeveralInstances, Renderer>();
    system.add_unclearable_inbox::<MoveEye, Renderer>();
    system.add_unclearable_inbox::<AddEyeListener, Renderer>();
    system.add_unclearable_inbox::<AddDebugText, Renderer>();
    system.add_unclearable_inbox::<UpdateThing, Renderer>();
    system.add_unclearable_inbox::<Project2dTo3d, Renderer>();

    Renderer::id() << Control::Setup;
}
