
pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};
use kay::{ID, Fate, ActorSystem, World};
use glium::Frame;

use super::{Renderer, RendererID};

impl Renderer {
    pub fn setup(&mut self, world: &mut World) {
        for (scene_id, scene) in self.scenes.iter().enumerate() {
            for renderable in &scene.renderables {
                renderable.setup_in_scene(self.id, scene_id, world);
            }
        }
    }

    pub fn render(&mut self, world: &mut World) {
        let self_id = self.id;
        for (scene_id, mut scene) in self.scenes.iter_mut().enumerate() {
            for batch_to_clear in (&mut scene).batches.values_mut().filter(|batch| {
                batch.clear_every_frame
            })
            {
                batch_to_clear.instances.clear();
            }
            for renderable in &scene.renderables {
                renderable.render_to_scene(self_id, scene_id, world);
            }
        }
    }

    pub fn submit(&mut self, target_ptr: usize, return_to: ID, world: &mut World) {
        let mut target = unsafe { Box::from_raw(target_ptr as *mut Frame) };

        for scene in &self.scenes {
            self.render_context.submit(scene, &mut *target);
        }

        world.send(
            return_to,
            Submitted { target_ptr: Box::into_raw(target) as usize },
        );

    }
}

// #[derive(Copy, Clone)]
// pub struct SetupInScene {
//     pub renderer_id: RendererID,
//     pub scene_id: usize,
// }

// #[derive(Copy, Clone)]
// pub struct RenderToScene {
//     pub renderer_id: RendererID,
//     pub scene_id: usize,
// }

// #[derive(Copy, Clone)]
// pub enum Control {
//     Setup,
//     Render,
//     Submit { target_ptr: usize, return_to: ID },
// }

#[derive(Copy, Clone)]
pub struct Submitted {
    pub target_ptr: usize,
}

mod kay_auto;
pub use self::kay_auto::*;