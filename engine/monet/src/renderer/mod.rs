pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, Into2d, Into3d, WithUniqueOrthogonal};
use compact::CVec;
use kay::{World, ActorSystem, External};

use glium::backend::glutin::Display;

use {Batch, Instance, Scene, SceneDescription, Mesh, RenderContext};

mod control;
pub mod movement;
mod project;

pub use self::control::{TargetProvider, TargetProviderID};
pub use self::movement::{Movement, EyeListener, EyeListenerID};
pub use self::project::{ProjectionRequester, ProjectionRequesterID};

#[derive(Compact, Clone)]
pub struct Renderer {
    id: RendererID,
    inner: External<RendererState>,
}

pub struct RendererState {
    pub current_frame: usize,
    pub scene: Scene,
    pub render_context: RenderContext,
}

impl ::std::ops::Deref for Renderer {
    type Target = RendererState;

    fn deref(&self) -> &RendererState {
        &self.inner
    }
}

impl ::std::ops::DerefMut for Renderer {
    fn deref_mut(&mut self) -> &mut RendererState {
        &mut self.inner
    }
}

impl Renderer {
    pub fn spawn(
        id: RendererID,
        window: &External<Display>,
        scene_description: &SceneDescription,
        clear_color: (f32, f32, f32, f32),
        world: &mut World,
    ) -> Renderer {
        id.setup(world);
        Renderer {
            id,
            inner: External::new(RendererState {
                current_frame: 0,
                scene: scene_description.to_scene(),
                render_context: RenderContext::new(window, clear_color),
            }),
        }
    }
}

impl Renderer {
    /// Critical
    pub fn add_eye_listener(&mut self, listener: EyeListenerID, _: &mut World) {
        self.scene.eye_listeners.push(listener);
    }

    /// Critical
    pub fn add_batch(&mut self, batch_id: u32, prototype: &Mesh, _: &mut World) {
        let batch = {
            let inner = &mut *self.inner;
            let window = &inner.render_context.window;
            Batch::new(prototype, window)
        };
        self.scene.batches.insert(batch_id, batch);
    }

    /// Critical
    pub fn update_individual(
        &mut self,
        individual_id: u32,
        mesh: &Mesh,
        instance_info: &Instance,
        is_decal: bool,
        _: &mut World,
    ) {
        let individual = {
            let inner = &mut *self.inner;
            let window = &inner.render_context.window;
            Batch::new_individual(mesh, *instance_info, is_decal, window)
        };
        self.scene.batches.insert(individual_id, individual);
    }

    /// Critical
    pub fn add_instance(
        &mut self,
        batch_id: u32,
        frame: usize,
        instance_info: Instance,
        _: &mut World,
    ) {
        let batch = self.scene.batches.get_mut(&batch_id).unwrap();

        if batch.clear_every_frame && batch.frame < frame {
            if let Some(end) = batch.full_frame_instance_end {
                // finished a second frame, remove first from double-buffer
                batch.instances = batch.instances.split_off(end);
            }
            batch.full_frame_instance_end = Some(batch.instances.len());
            batch.frame = frame;
        }

        batch.instances.push(instance_info);
    }

    /// Critical
    pub fn add_several_instances(
        &mut self,
        batch_id: u32,
        frame: usize,
        instances: &CVec<Instance>,
        _: &mut World,
    ) {
        let batch = self.scene.batches.get_mut(&batch_id).unwrap();

        if batch.clear_every_frame && batch.frame < frame {
            if let Some(end) = batch.full_frame_instance_end {
                // finished a second frame, remove first from double-buffer
                batch.instances = batch.instances.split_off(end);
            }
            batch.full_frame_instance_end = Some(batch.instances.len());
            batch.frame = frame;
        }

        batch.instances.extend_from_slice(instances);
    }
}

pub trait Renderable {
    fn init(&mut self, _renderer_id: RendererID, _world: &mut World) {}
    fn prepare_render(&mut self, _renderer_id: RendererID, _frame: usize, _world: &mut World) {}
    fn render(&mut self, renderer_id: RendererID, frame: usize, world: &mut World);
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<Renderer>();
    auto_setup(system);
    control::auto_setup(system);
    movement::auto_setup(system);
    project::auto_setup(system);
    super::mesh_actors::setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
