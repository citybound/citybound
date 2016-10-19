#[macro_use]
pub extern crate glium;
extern crate glium_text;
extern crate nalgebra;
use nalgebra::{Point3, Vector3, Isometry3, Perspective3, ToHomogeneous};

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

#[derive(Clone)]
pub struct Thing {
    vertices: Vec<Vertex>,
    indices: Vec<u16>
}

impl Thing {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u16>) -> Thing {
        Thing{vertices: vertices, indices: indices}
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
    pub batches: std::collections::HashMap<usize, Batch>,
    pub debug_text: String
}

impl Scene {
    pub fn new() -> Scene {
        Scene{
            eye: Eye{
                position: Point3::new(-5.0, -5.0, 5.0),
                target: Point3::new(0.0, 0.0, 0.0),
                up: Vector3::new(0.0, 0.0, 1.0),
                field_of_view: 0.3 * std::f32::consts::PI
            },
            batches: std::collections::HashMap::new(),
            debug_text: String::new()
        }
    }
}

pub struct Renderer {
    pub window: GlutinFacade,
    batch_program: glium::Program,
    text_system: glium_text::TextSystem,
    font: glium_text::FontTexture
}

impl Renderer {
    pub fn new (window: GlutinFacade) -> Renderer {
        Renderer{
            batch_program: program!(&window,
                140 => {
                    vertex: include_str!("shader/solid_batch_140.glslv"),
                    fragment: include_str!("shader/solid_140.glslf")
                }
            ).unwrap(),
            text_system: glium_text::TextSystem::new(&window),
            font: glium_text::FontTexture::new(
                &window,
                std::fs::File::open(&std::path::Path::new("fonts/ClearSans-Regular.ttf")).unwrap(),
                64
            ).unwrap(),
            window: window,
        }
    }

    pub fn draw (&self, scene: &Scene) {
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
            let vertices = glium::VertexBuffer::new(&self.window, batch.prototype.vertices.as_slice()).unwrap();
            let indices = glium::IndexBuffer::new(&self.window, index::PrimitiveType::TrianglesList, batch.prototype.indices.as_slice()).unwrap();
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