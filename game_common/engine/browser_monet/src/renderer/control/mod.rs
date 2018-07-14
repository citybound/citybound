pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, Into2d, Into3d, WithUniqueOrthogonal};
use kay::{World, External};

use super::{Renderer, RendererID};

impl Renderer {
    /// Critical
    pub fn setup(&mut self, world: &mut World) {}

    /// Critical
    pub fn prepare_render(&mut self, world: &mut World) {}

    /// Critical
    pub fn render(&mut self, world: &mut World) {}

    /// Critical
    pub fn submit(&mut self, given_target: (), return_to: TargetProviderID, world: &mut World) {}
}

pub trait TargetProvider {
    fn submitted(&mut self, target: (), world: &mut World);
}

mod kay_auto;
pub use self::kay_auto::*;
