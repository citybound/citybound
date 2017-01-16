
pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};
use compact::CVec;
use kay::{ID, Recipient, Individual, Fate};

use glium::backend::glutin_backend::GlutinFacade;

use ::{Batch, Instance, Scene, Thing, RenderContext};

mod control;
mod movement;
mod project;

pub use self::control::{SetupInScene, RenderToScene, Control};
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

impl Individual for Renderer {}

#[derive(Copy, Clone)]
pub struct AddEyeListener {
    pub scene_id: usize,
    pub listener: ID,
}

impl Recipient<AddEyeListener> for Renderer {
    fn receive(&mut self, msg: &AddEyeListener) -> Fate {
        match *msg {
            AddEyeListener { scene_id, listener } => {
                self.scenes[scene_id].eye_listeners.push(listener);
                Fate::Live
            }
        }
    }
}

#[derive(Compact, Clone)]
pub struct AddBatch {
    pub scene_id: usize,
    pub batch_id: u16,
    pub thing: Thing,
}

impl Recipient<AddBatch> for Renderer {
    fn receive(&mut self, msg: &AddBatch) -> Fate {
        match *msg {
            AddBatch { scene_id, batch_id, ref thing } => {
                let window = &self.render_context.window;
                self.scenes[scene_id].batches.insert(batch_id, Batch::new(thing.clone(), window));
                Fate::Live
            }
        }
    }
}

#[derive(Compact, Clone)]
pub struct UpdateThing {
    pub scene_id: usize,
    pub thing_id: u16,
    pub thing: Thing,
    pub instance: Instance,
    pub is_decal: bool,
}

impl Recipient<UpdateThing> for Renderer {
    fn receive(&mut self, msg: &UpdateThing) -> Fate {
        match *msg {
            UpdateThing { scene_id, thing_id, ref thing, instance, is_decal } => {
                let thing = Batch::new_thing(thing.clone(),
                                             instance,
                                             is_decal,
                                             &self.render_context.window);
                self.scenes[scene_id].batches.insert(thing_id, thing);
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct AddInstance {
    pub scene_id: usize,
    pub batch_id: u16,
    pub instance: Instance,
}

impl Recipient<AddInstance> for Renderer {
    fn receive(&mut self, msg: &AddInstance) -> Fate {
        match *msg {
            AddInstance { scene_id, batch_id, instance } => {
                self.scenes[scene_id].batches.get_mut(&batch_id).unwrap().instances.push(instance);
                Fate::Live
            }
        }
    }
}

#[derive(Compact, Clone)]
pub struct AddSeveralInstances {
    pub scene_id: usize,
    pub batch_id: u16,
    pub instances: CVec<Instance>,
}

impl Recipient<AddSeveralInstances> for Renderer {
    fn receive(&mut self, msg: &AddSeveralInstances) -> Fate {
        match *msg {
            AddSeveralInstances { scene_id, batch_id, ref instances } => {
                self.scenes[scene_id]
                    .batches
                    .get_mut(&batch_id)
                    .unwrap()
                    .instances
                    .extend_from_slice(instances);
                Fate::Live
            }
        }
    }
}

#[derive(Compact, Clone)]
pub struct AddDebugText {
    pub scene_id: usize,
    pub key: CVec<char>,
    pub text: CVec<char>,
    pub color: [f32; 4],
    pub persistent: bool,
}

impl Recipient<AddDebugText> for Renderer {
    fn receive(&mut self, msg: &AddDebugText) -> Fate {
        match *msg {
            AddDebugText { scene_id, ref key, ref text, ref color, persistent } => {
                let target = if persistent {
                    &mut self.scenes[scene_id].persistent_debug_text
                } else {
                    &mut self.scenes[scene_id].debug_text
                };
                target.insert(key.iter().cloned().collect(),
                              (text.iter().cloned().collect(), *color));
                Fate::Live
            }
        }
    }
}
