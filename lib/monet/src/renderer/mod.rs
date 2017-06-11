
pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};
use compact::CVec;
use kay::{ID, Fate, World, ActorSystem};

use glium::backend::glutin_backend::GlutinFacade;

use {Batch, Instance, Scene, Thing, RenderContext};

mod control;
mod movement;
mod project;

pub use self::control::{SetupInScene, RenderToScene, Control, Submitted};
pub use self::movement::{Movement, MoveEye, EyeMoved};
pub use self::project::{Project2dTo3d, Projected3d};


pub struct Renderer {
    pub scenes: Vec<Scene>,
    pub render_context: RenderContext,
}

impl Renderer {
    pub fn new(window: GlutinFacade) -> Renderer {
        Renderer {
            scenes: Vec::new(),
            render_context: RenderContext::new(window),
        }
    }
}

// Define Actor
impl Renderer {
    pub fn id(world: &mut World) -> RendererID {
        RendererID::in_world(world)
    }

    /// Critical
    pub fn add_eye_listener(&mut self, scene_id: usize, listener: ID, _: &mut World) {
        self.scenes[scene_id].eye_listeners.push(listener);
    }

    /// Critical
    pub fn add_batch(&mut self, scene_id: usize, batch_id: u16, thing: &Thing, _: &mut World) {
        let window = &self.render_context.window;
        self.scenes[scene_id]
            .batches
            .insert(batch_id, Batch::new(thing.clone(), window));
    }

    /// Critical
    pub fn update_thing(&mut self,
                        scene_id: usize,
                        thing_id: u16,
                        thing: &Thing,
                        instance: &Instance,
                        is_decal: bool,
                        _: &mut World) {
        let thing = Batch::new_thing(thing.clone(),
                                     *instance,
                                     is_decal,
                                     &self.render_context.window);
        self.scenes[scene_id].batches.insert(thing_id, thing);
    }

    /// Critical
    pub fn add_instance(&mut self,
                        scene_id: usize,
                        batch_id: u16,
                        instance: Instance,
                        _: &mut World) {
        self.scenes[scene_id]
            .batches
            .get_mut(&batch_id)
            .unwrap()
            .instances
            .push(instance);
    }

    /// Critical
    pub fn add_several_instances(&mut self,
                                 scene_id: usize,
                                 batch_id: u16,
                                 instances: &CVec<Instance>,
                                 _: &mut World) {
        self.scenes[scene_id]
            .batches
            .get_mut(&batch_id)
            .unwrap()
            .instances
            .extend_from_slice(instances);
    }
}

pub fn setup(system: &mut ActorSystem, initial: Renderer) {
    auto_setup(system, (initial,));
    control::setup(system);
    movement::setup(system);
    project::setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;