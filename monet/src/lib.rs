#[macro_use]
pub extern crate glium;
extern crate glium_text;
extern crate nalgebra;
use nalgebra::{Point3, Vector3, Isometry3, Perspective3, ToHomogeneous};

use glium::{index, Surface};
use glium::backend::glutin_backend::GlutinFacade;

use std::collections::HashMap;

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 3]
}

implement_vertex!(Vertex, position);

pub struct Eye {
    pub position: Point3<f32>,
    pub target: Point3<f32>,
    pub up: Vector3<f32>,
    pub field_of_view: f32
}

pub struct Thing {
    vertices: Vec<Vertex>,
    indices: Vec<u16>
}

impl Thing {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u16>) -> Thing {
        Thing{vertices: vertices, indices: indices}
    }
}

pub struct Scene {
    eye: Eye,
    pub things: HashMap<&'static str, Thing>,
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
            things: HashMap::with_capacity(3000),
            debug_text: String::new()
        }
    }
}

pub struct Renderer<'a> {
    window: &'a GlutinFacade,
    program: glium::Program,
    text_system: glium_text::TextSystem,
    font: glium_text::FontTexture
}

impl<'a> Renderer<'a> {
    pub fn new (window: &'a GlutinFacade) -> Renderer<'a> {
        Renderer{
            window: window,
            program: program!(window,
                140 => {
                    vertex: include_str!("shader/solid_140.glslv"),
                    fragment: include_str!("shader/solid_140.glslf"),
                },
            ).unwrap(),
            text_system: glium_text::TextSystem::new(window),
            font: glium_text::FontTexture::new(
                window,
                std::fs::File::open(&std::path::Path::new("resources/ClearSans-Regular.ttf")).unwrap(),
                64
            ).unwrap()
        }
    }

    pub fn draw (&self, scene: Scene) {
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
        target.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);

        for thing in scene.things.values() {
            let vertices = glium::VertexBuffer::new(self.window, thing.vertices.as_slice()).unwrap();
            let indices = glium::IndexBuffer::new(self.window, index::PrimitiveType::TrianglesList, thing.indices.as_slice()).unwrap();
            target.draw(&vertices, &indices, &self.program, &uniforms, &params).unwrap();
        }

        let text = glium_text::TextDisplay::new(&self.text_system, &self.font, scene.debug_text.as_str());
        let text_matrix = [
            [0.05, 0.0, 0.0, 0.0],
            [0.0, 0.05, 0.0, 0.0],
            [0.0, 0.0, 0.05, 0.0],
            [-0.9, 0.8, 0.0, 1.0f32]
        ];

        glium_text::draw(&text, &self.text_system, &mut target, text_matrix, (1.0, 1.0, 0.0, 1.0));

        target.finish().unwrap();
    }
}