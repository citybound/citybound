pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};
use compact::CVec;
use kay::ID;
use fnv::FnvHashMap;

use renderer::RenderableID;

use Batch;

#[derive(Copy, Clone)]
pub struct Eye {
    pub position: P3,
    pub target: P3,
    pub up: V3,
    pub field_of_view: f32,
}

pub struct Scene {
    pub eye: Eye,
    pub eye_listeners: CVec<ID>,
    pub batches: FnvHashMap<u16, Batch>,
    pub renderables: Vec<RenderableID>,
}

impl Scene {
    pub fn new() -> Scene {
        Scene {
            eye: Eye {
                position: P3::new(-5.0, -5.0, 5.0),
                target: P3::new(0.0, 0.0, 0.0),
                up: V3::new(0.0, 0.0, 1.0),
                field_of_view: 0.3 * ::std::f32::consts::PI,
            },
            eye_listeners: CVec::new(),
            batches: FnvHashMap::default(),
            renderables: Vec::new(),
        }
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}
