pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, Into2d, Into3d, WithUniqueOrthogonal,
try_inverse};
use kay::World;

use {Renderer, RendererID};

pub trait ProjectionRequester {
    fn projected_3d(&mut self, position_3d: P3, world: &mut World);
}

impl Renderer {
    /// Critical
    pub fn project_2d_to_3d(
        &mut self,
        position_2d: P2,
        requester: ProjectionRequesterID,
        world: &mut World,
    ) {
    }

    pub fn project(&self, position_2d: P2) -> P3 {
        P3::new(0.0, 0.0, 0.0)
    }
}

mod kay_auto;
pub use self::kay_auto::*;
