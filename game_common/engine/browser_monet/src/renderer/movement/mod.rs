pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, Into2d, Into3d, WithUniqueOrthogonal};
use kay::World;

use {Renderer, RendererID, Eye};

#[derive(Copy, Clone)]
pub enum Movement {
    Shift(V3),
    ShiftAbsolute(V3),
    ShiftProjected(P2, P2),
    Zoom(N, P3),
    Yaw(N),
    Pitch(N),
}

pub trait EyeListener {
    fn eye_moved(&mut self, eye: Eye, movement: Movement, world: &mut World);
}

impl Renderer {
    /// Critical
    pub fn move_eye(&mut self, movement: Movement, world: &mut World) {}
}

mod kay_auto;
pub use self::kay_auto::*;
