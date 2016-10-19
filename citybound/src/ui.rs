use ::monet::glium::{DisplayBuild, glutin};
use kay::{ID, Known, Message, Recipient, World, ActorSystem, InMemory, CVec, Compact};
use ::monet::{Renderer, Scene, WorldPosition, GlutinFacade};
use ::monet::glium::glutin::{Event};

#[derive(Copy, Clone)]
pub struct Render {
    pub render_manager_id: ID
}
message!(Render, ::type_ids::Messages::Render);

pub struct RenderManager {
    pub scene: Scene,
    pub renderables: Vec<ID> 
}
impl Known for RenderManager {fn type_id() -> usize {::type_ids::Recipients::RenderManager as usize}}

derive_compact!{
    pub struct AddRenderable{
        id: ID,
        batches: CVec<(::type_ids::RenderBatches, ::monet::Thing)>
    }
}
message!(AddRenderable, ::type_ids::Messages::AddRenderable);

impl AddRenderable{
    pub fn new(id: ID, batches: Vec<(::type_ids::RenderBatches, ::monet::Thing)>) -> AddRenderable{
        let mut compact_batches = CVec::new();
        for batch in batches {
            compact_batches.push(batch);
        }
        AddRenderable{
            id: id,
            batches: compact_batches
        }
    }
}

#[derive(Copy, Clone)]
pub struct StartFrame;
message!(StartFrame, ::type_ids::Messages::StartFrame);

#[derive(Copy, Clone)]
pub struct InstancePosition {
    pub batch_id: usize,
    pub position: WorldPosition
}
message!(InstancePosition, ::type_ids::Messages::InstancePosition);

recipient!(RenderManager, (&mut self, world: &mut World, self_id: ID) {
    AddRenderable: &AddRenderable{id, ref batches} => {
        self.renderables.push(id);
        for batch in batches {
            let &(batch_id, ref thing) = batch;
            self.scene.batches.insert(batch_id as usize, ::monet::Batch::new(thing.clone(), Vec::new()));
        }
    },

    StartFrame: _ => {
        for batch in &mut self.scene.batches.values_mut() {
            batch.instances.clear();
        }

        for renderable in &self.renderables {
            world.send(*renderable, Render {
                render_manager_id: self_id
            })
        }
    },

    InstancePosition: &InstancePosition{batch_id, position} => {
        self.scene.batches.get_mut(&batch_id).unwrap().instances.push(position)
    }
});

pub fn setup(system: &mut ActorSystem) {
    let mut render_manager = RenderManager{
        scene: Scene::new(),
        renderables: Vec::new()
    };
    render_manager.scene.eye.position *= 30.0;
    system.add_individual(render_manager, ::type_ids::Recipients::RenderManager as usize);
    system.add_individual_inbox::<AddRenderable, RenderManager>(InMemory("add_renderable", 512 * 8, 4), ::type_ids::Recipients::RenderManager as usize);
    system.add_individual_inbox::<InstancePosition, RenderManager>(InMemory("instance_position", 512 * 8, 4), ::type_ids::Recipients::RenderManager as usize);
    system.add_individual_inbox::<StartFrame, RenderManager>(InMemory("start_frame", 512 * 8, 4), ::type_ids::Recipients::RenderManager as usize);
}

pub fn setup_window_and_renderer() -> Renderer {
    let window = glutin::WindowBuilder::new()
        .with_title("Citybound".to_string())
        .with_dimensions(512, 512)
        .with_multitouch()
        .with_vsync().build_glium().unwrap();

    Renderer::new(window)
}

pub fn process_events(window: &GlutinFacade) -> bool {
    for event in window.poll_events() {
        match event {
            Event::Closed => return false,
            _ => {}
        }
    }
    true
}

pub fn finish_frame(system: &mut ActorSystem, renderer: &Renderer) {
    let render_manager = system.get_individual::<RenderManager>(::type_ids::Recipients::RenderManager as usize);
    renderer.draw(&render_manager.scene);
}