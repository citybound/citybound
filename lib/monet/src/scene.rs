pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};
use compact::CVec;
use kay::ID;
use fnv::FnvHashMap;

use renderer::RenderableID;
use renderer::movement::EyeListenerID;

use Batch;

#[derive(Copy, Clone)]
pub struct Eye {
    pub position: P3,
    pub target: P3,
    pub up: V3,
    pub field_of_view: f32,
}

#[derive(Compact, Clone)]
pub struct SceneDescription {
    pub eye: Eye,
    pub renderables: CVec<RenderableID>,
}

impl SceneDescription {
    pub fn new(renderables: CVec<RenderableID>) -> SceneDescription {
        SceneDescription {
            eye: Eye {
                position: P3::new(-5.0, -5.0, 5.0),
                target: P3::new(0.0, 0.0, 0.0),
                up: V3::new(0.0, 0.0, 1.0),
                field_of_view: 0.3 * ::std::f32::consts::PI,
            },
            renderables: renderables,
        }
    }

    pub fn to_scene(&self) -> Scene {
        Scene {
            description: self.clone(),
            eye_listeners: CVec::new(),
            batches: FnvHashMap::default(),
        }
    }
}

pub struct Scene {
    description: SceneDescription,
    pub eye_listeners: CVec<EyeListenerID>,
    pub batches: FnvHashMap<u16, Batch>,
}

impl ::std::ops::Deref for Scene {
    type Target = SceneDescription;

    fn deref(&self) -> &SceneDescription {
        &self.description
    }
}

impl ::std::ops::DerefMut for Scene {
    fn deref_mut(&mut self) -> &mut SceneDescription {
        &mut self.description
    }
}