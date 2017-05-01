
pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};
use kay::{ID, Recipient, Actor, Fate};
use glium::Frame;

use Renderer;

#[derive(Copy, Clone)]
pub struct SetupInScene {
    pub renderer_id: ID,
    pub scene_id: usize,
}

#[derive(Copy, Clone)]
pub struct RenderToScene {
    pub renderer_id: ID,
    pub scene_id: usize,
}

#[derive(Copy, Clone)]
pub enum Control {
    Setup,
    Render,
    Submit { target_ptr: usize, return_to: ID },
}

#[derive(Copy, Clone)]
pub struct Submitted {
    pub target_ptr: usize,
}

impl Recipient<Control> for Renderer {
    fn receive(&mut self, msg: &Control) -> Fate {
        match *msg {
            Control::Setup => self.control_setup(),
            Control::Render => self.control_render(),
            Control::Submit {
                target_ptr,
                return_to,
            } => self.control_submit(target_ptr, return_to),
        }
    }
}

impl Renderer {
    fn control_setup(&mut self) -> Fate {
        for (scene_id, scene) in self.scenes.iter().enumerate() {
            for renderable in &scene.renderables {
                *renderable <<
                SetupInScene {
                    renderer_id: Renderer::id(),
                    scene_id: scene_id,
                };
            }
        }
        Fate::Live
    }

    fn control_render(&mut self) -> Fate {
        for (scene_id, mut scene) in self.scenes.iter_mut().enumerate() {
            for batch_to_clear in (&mut scene)
                    .batches
                    .values_mut()
                    .filter(|batch| batch.clear_every_frame) {
                batch_to_clear.instances.clear();
            }
            for renderable in &scene.renderables {
                *renderable <<
                RenderToScene {
                    renderer_id: Renderer::id(),
                    scene_id: scene_id,
                };
            }
        }
        Fate::Live
    }

    fn control_submit(&mut self, target_ptr: usize, return_to: ID) -> Fate {
        let mut target = unsafe { Box::from_raw(target_ptr as *mut Frame) };

        for scene in &mut self.scenes {
            self.render_context.submit(scene, &mut *target);
        }

        return_to << Submitted { target_ptr: Box::into_raw(target) as usize };

        Fate::Live
    }
}
