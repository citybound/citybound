use ::nalgebra::{Point3, Vector3, Isometry3, Perspective3, ToHomogeneous};
use ::kay::{ID, World, Recipient, CVec, Compact, ActorSystem, InMemory};
use std::collections::HashMap;
extern crate glium_text;

pub use glium;
use glium::{index, Surface};
pub use glium::backend::glutin_backend::GlutinFacade;

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 3]
}
implement_vertex!(Vertex, position);

#[derive(Copy, Clone)]
pub struct WorldPosition {
    pub world_position: [f32; 3]
}
implement_vertex!(WorldPosition, world_position);

pub struct Eye {
    pub position: Point3<f32>,
    pub target: Point3<f32>,
    pub up: Vector3<f32>,
    pub field_of_view: f32
}

derive_compact!{
    pub struct Thing {
        vertices: CVec<Vertex>,
        indices: CVec<u16>
    }
}

impl Thing {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u16>) -> Thing {
        Thing{vertices: vertices.into(), indices: indices.into()}
    }
}

impl Clone for Thing {
    fn clone(&self) -> Thing {
        Thing {
            vertices: self.vertices.to_vec().into(),
            indices: self.indices.to_vec().into()
        }
    }
}

pub struct Batch {
    prototype: Thing,
    pub instances: Vec<WorldPosition>
}

impl Batch {
    pub fn new(prototype: Thing, instances: Vec<WorldPosition>) -> Batch {
        Batch{prototype: prototype, instances: instances}
    }
}

pub struct Scene {
    pub eye: Eye,
    pub batches: HashMap<usize, Batch>,
    pub renderables: Vec<ID>,
    pub debug_text: String
}

impl Scene {
    pub fn new() -> Scene {
        Scene{
            eye: Eye{
                position: Point3::new(-5.0, -5.0, 5.0),
                target: Point3::new(0.0, 0.0, 0.0),
                up: Vector3::new(0.0, 0.0, 1.0),
                field_of_view: 0.3 * ::std::f32::consts::PI
            },
            batches: HashMap::new(),
            renderables: Vec::new(),
            debug_text: String::new()
        }
    }
}

pub struct Renderer {
    pub scenes: HashMap<usize, Scene>,
    pub render_context: RenderContext
}

#[derive(Copy, Clone)]
struct Setup;

#[derive(Copy, Clone)]
pub struct SetupInScene {
    pub renderer_id: ID,
    pub scene_id: usize
}

#[derive(Copy, Clone)]
pub struct Render;

#[derive(Copy, Clone)]
pub struct RenderToScene {
    pub renderer_id: ID,
    pub scene_id: usize
}

derive_compact!{
    pub struct AddBatch {
        scene_id: usize,
        batch_id: usize,
        thing: Thing
    }
}

impl AddBatch {
    pub fn new(scene_id: usize, batch_id: usize, thing: Thing) -> AddBatch {
        AddBatch{scene_id: scene_id, batch_id: batch_id, thing: thing}
    }
}

#[derive(Copy, Clone)]
pub struct AddInstance {
    pub scene_id: usize,
    pub batch_id: usize,
    pub position: WorldPosition
}

#[derive(Copy, Clone)]
pub struct Submit;

recipient!{Renderer, (&mut self, world: &mut World, self_id: ID) {
    Setup: _ => {
        for (scene_id, scene) in &self.scenes {
            for renderable in &scene.renderables {
                world.send(*renderable, SetupInScene{renderer_id: self_id, scene_id: *scene_id});
            }
        }
    },

    Render: _ => {
        for (scene_id, mut scene) in &mut self.scenes {
            for batch in (&mut scene).batches.values_mut() {
                batch.instances.clear();
            }
            for renderable in &scene.renderables {
                world.send(*renderable, RenderToScene{renderer_id: self_id, scene_id: *scene_id});
            }
        }
    },

    Submit: _ => {
        for scene in self.scenes.values() {
            self.render_context.submit(scene);
        }
    },

    AddBatch: &AddBatch{scene_id, batch_id, ref thing} => {
        self.scenes.get_mut(&scene_id).unwrap().batches.insert(batch_id, Batch::new(thing.clone(), Vec::new()));
    },

    AddInstance: &AddInstance{scene_id, batch_id, position} => {
        self.scenes.get_mut(&scene_id).unwrap().batches.get_mut(&batch_id).unwrap().instances.push(position);
    }
}}

impl Renderer {
    pub fn new (window: GlutinFacade) -> Renderer {
        Renderer {
            scenes: HashMap::new(),
            render_context: RenderContext::new(window)
        }
    }
}

pub fn setup(system: &mut ActorSystem, renderer: Renderer) {
    system.add_individual(renderer);
    system.add_individual_inbox::<Setup, Renderer>(InMemory("setup", 512 * 8, 4));
    system.add_individual_inbox::<Render, Renderer>(InMemory("render", 512 * 8, 4));
    system.add_individual_inbox::<Submit, Renderer>(InMemory("submit", 512 * 8, 4));
    system.add_individual_inbox::<AddBatch, Renderer>(InMemory("add_batch", 512 * 8, 4));
    system.add_individual_inbox::<AddInstance, Renderer>(InMemory("add_instance", 512 * 8, 4));

    system.world().send_to_individual::<Setup, Renderer>(Setup);
}

pub struct RenderContext {
    pub window: GlutinFacade,
    batch_program: glium::Program,
    text_system: glium_text::TextSystem,
    font: glium_text::FontTexture
}

impl RenderContext {
    pub fn new (window: GlutinFacade) -> RenderContext {
        RenderContext{
            batch_program: program!(&window,
                140 => {
                    vertex: include_str!("shader/solid_batch_140.glslv"),
                    fragment: include_str!("shader/solid_140.glslf")
                }
            ).unwrap(),
            text_system: glium_text::TextSystem::new(&window),
            font: glium_text::FontTexture::new(
                &window,
                ::std::fs::File::open(&::std::path::Path::new("fonts/ClearSans-Regular.ttf")).unwrap(),
                64
            ).unwrap(),
            window: window,
        }
    }

    pub fn submit (&self, scene: &Scene) {
        let mut target = self.window.draw();

        let view : [[f32; 4]; 4] = *Isometry3::look_at_rh(
            &scene.eye.position,
            &scene.eye.target,
            &scene.eye.up
        ).to_homogeneous().as_ref();
        let perspective : [[f32; 4]; 4] = *Perspective3::new(
            target.get_dimensions().0 as f32 / target.get_dimensions().1 as f32,
            scene.eye.field_of_view,
            0.1,
            1000.0
        ).to_matrix().as_ref();

        let model = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0f32]
        ];
        
        let uniforms = uniform! {
            model: model,
            view: view,
            perspective: perspective
        };

        let params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::draw_parameters::DepthTest::IfLess,
                write: true,
                .. Default::default()
            },
            .. Default::default()
        };
        
        // draw a frame
        target.clear_color_and_depth((1.0, 1.0, 1.0, 1.0), 1.0);

        for batch in scene.batches.values() {
            let vertices = glium::VertexBuffer::new(&self.window, &batch.prototype.vertices).unwrap();
            let indices = glium::IndexBuffer::new(&self.window, index::PrimitiveType::TrianglesList, &batch.prototype.indices).unwrap();
            let instances = glium::VertexBuffer::dynamic(&self.window, batch.instances.as_slice()).unwrap();
            target.draw((&vertices, instances.per_instance().unwrap()), &indices, &self.batch_program, &uniforms, &params).unwrap();
        }

        let text = glium_text::TextDisplay::new(&self.text_system, &self.font, scene.debug_text.as_str());
        let text_matrix = [
            [0.05, 0.0, 0.0, 0.0],
            [0.0, 0.05, 0.0, 0.0],
            [0.0, 0.0, 0.05, 0.0],
            [-0.9, 0.8, 0.0, 1.0f32]
        ];

        glium_text::draw(&text, &self.text_system, &mut target, text_matrix, (0.0, 0.0, 0.0, 1.0));

        target.finish().unwrap();
    }
}