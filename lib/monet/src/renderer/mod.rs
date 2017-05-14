
pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};
use compact::CVec;
use kay::{ID, Fate};

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

#[derive(Copy, Clone)]
pub struct AddEyeListener {
    pub scene_id: usize,
    pub listener: ID,
}

#[derive(Compact, Clone)]
pub struct AddBatch {
    pub scene_id: usize,
    pub batch_id: u16,
    pub thing: Thing,
}

#[derive(Compact, Clone)]
pub struct UpdateThing {
    pub scene_id: usize,
    pub thing_id: u16,
    pub thing: Thing,
    pub instance: Instance,
    pub is_decal: bool,
}

#[derive(Copy, Clone)]
pub struct AddInstance {
    pub scene_id: usize,
    pub batch_id: u16,
    pub instance: Instance,
}

#[derive(Compact, Clone)]
pub struct AddSeveralInstances {
    pub scene_id: usize,
    pub batch_id: u16,
    pub instances: CVec<Instance>,
}

use kay::ActorSystem;

pub fn setup(system: &mut ActorSystem, initial: Renderer) {
    system.add(initial, |mut the_renderer| {

        the_renderer.on_critical(|&AddEyeListener { scene_id, listener }, renderer, _| {
            renderer.scenes[scene_id].eye_listeners.push(listener);
            Fate::Live
        });

        the_renderer.on_critical(|&AddBatch { scene_id, batch_id, ref thing }, renderer, _| {
            let window = &renderer.render_context.window;
            renderer.scenes[scene_id]
                .batches
                .insert(batch_id, Batch::new(thing.clone(), window));
            Fate::Live
        });

        the_renderer.on_critical(|&UpdateThing {
                                       scene_id,
                                       thing_id,
                                       ref thing,
                                       instance,
                                       is_decal,
                                   },
                                  renderer,
                                  _| {
            let thing = Batch::new_thing(thing.clone(),
                                         instance,
                                         is_decal,
                                         &renderer.render_context.window);
            renderer.scenes[scene_id].batches.insert(thing_id, thing);
            Fate::Live
        });

        the_renderer.on_critical(|&AddInstance { scene_id, batch_id, instance }, renderer, _| {
            renderer.scenes[scene_id]
                .batches
                .get_mut(&batch_id)
                .unwrap()
                .instances
                .push(instance);
            Fate::Live
        });

        the_renderer.on_critical(|&AddSeveralInstances { scene_id, batch_id, ref instances },
                                  renderer,
                                  _| {
            renderer.scenes[scene_id]
                .batches
                .get_mut(&batch_id)
                .unwrap()
                .instances
                .extend_from_slice(instances);
            Fate::Live
        });

    });
    control::setup(system);
    movement::setup(system);
    project::setup(system);
}
