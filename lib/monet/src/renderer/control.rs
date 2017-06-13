
pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};
use kay::{ID, Fate, ActorSystem};
use glium::Frame;

use super::{Renderer, RendererID};

#[derive(Copy, Clone)]
pub struct SetupInScene {
    pub renderer_id: RendererID,
    pub scene_id: usize,
}

#[derive(Copy, Clone)]
pub struct RenderToScene {
    pub renderer_id: RendererID,
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

pub fn setup(system: &mut ActorSystem) {
    system.extend::<Renderer, _>(|mut the_renderer| {
        let renderer_id = the_renderer.world().id::<Renderer>();
        let typed_renderer_id = Renderer::id(&mut the_renderer.world());

        the_renderer.on_critical(move |ctrl, renderer, world| match *ctrl {
                                     Control::Setup => {
            for (scene_id, scene) in renderer.scenes.iter().enumerate() {
                for renderable in &scene.renderables {
                    world.send(*renderable,
                               SetupInScene { renderer_id: typed_renderer_id, scene_id });
                }
            }
            Fate::Live
        }
                                     Control::Render => {
            for (scene_id, mut scene) in renderer.scenes.iter_mut().enumerate() {
                for batch_to_clear in (&mut scene)
                        .batches
                        .values_mut()
                        .filter(|batch| batch.clear_every_frame) {
                    batch_to_clear.instances.clear();
                }
                for renderable in &scene.renderables {
                    world.send(*renderable,
                               RenderToScene { renderer_id: typed_renderer_id, scene_id });
                }
            }
            Fate::Live
        }
                                     Control::Submit { target_ptr, return_to } => {
            let mut target = unsafe { Box::from_raw(target_ptr as *mut Frame) };

            for scene in &mut renderer.scenes {
                renderer.render_context.submit(scene, &mut *target);
            }

            world.send(return_to,
                       Submitted { target_ptr: Box::into_raw(target) as usize });

            Fate::Live
        }
                                 });

        the_renderer.world().send(renderer_id, Control::Setup);
    });
}
