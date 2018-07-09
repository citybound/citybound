pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, Into2d, Into3d, WithUniqueOrthogonal};
use compact::CVec;
use kay::{World, ActorSystem, External};

use {Instance, Scene, SceneDescription, Mesh};

mod control;
pub mod movement;
mod project;

pub use self::control::{TargetProvider, TargetProviderID};
pub use self::movement::{Movement, EyeListener, EyeListenerID};
pub use self::project::{ProjectionRequester, ProjectionRequesterID};

#[derive(Compact, Clone)]
pub struct Renderer {
    id: RendererID,
}

pub struct RendererState {
    pub current_frame: usize,
    pub scene: Scene,
}

impl Renderer {
    pub fn spawn(
        id: RendererID,
        scene_description: &SceneDescription,
        clear_color: (f32, f32, f32, f32),
        world: &mut World,
    ) -> Renderer {
        Renderer { id }
    }
}

impl Renderer {
    /// Critical
    pub fn add_eye_listener(&mut self, listener: EyeListenerID, _: &mut World) {}

    /// Critical
    pub fn add_batch(&mut self, batch_id: u32, prototype: &Mesh, _: &mut World) {}

    /// Critical
    pub fn update_individual(
        &mut self,
        individual_id: u32,
        mesh: &Mesh,
        instance_info: &Instance,
        is_decal: bool,
        _: &mut World,
    ) {
    }

    /// Critical
    pub fn add_instance(
        &mut self,
        batch_id: u32,
        frame: usize,
        instance_info: Instance,
        _: &mut World,
    ) {
    }

    /// Critical
    pub fn add_several_instances(
        &mut self,
        batch_id: u32,
        frame: usize,
        instances: &CVec<Instance>,
        _: &mut World,
    ) {
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
