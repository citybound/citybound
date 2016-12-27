
pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};
use kay::{ID, Recipient, Individual, Fate};

use ::Renderer;

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
    Submit,
}

impl Recipient<Control> for Renderer {
    fn receive(&mut self, msg: &Control) -> Fate {
        match *msg {
            Control::Setup => self.control_setup(),
            Control::Render => self.control_render(),
            Control::Submit => self.control_submit(),
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

    fn control_submit(&mut self) -> Fate {
        for scene in &mut self.scenes {
            self.render_context.submit(scene);
            scene.debug_text.clear();
        }
        Fate::Live
    }
}
