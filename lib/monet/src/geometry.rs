
pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};

use glium::{self, index};
use glium::backend::glutin_backend::GlutinFacade;

use ::Thing;

pub struct Batch {
    pub vertices: glium::VertexBuffer<Vertex>,
    pub indices: glium::IndexBuffer<u16>,
    pub instances: Vec<Instance>,
    pub clear_every_frame: bool,
    pub is_decal: bool,
}

impl Batch {
    pub fn new(prototype: Thing, window: &GlutinFacade) -> Batch {
        Batch {
            vertices: glium::VertexBuffer::new(window, &prototype.vertices).unwrap(),
            indices: glium::IndexBuffer::new(window,
                                             index::PrimitiveType::TrianglesList,
                                             &prototype.indices)
                .unwrap(),
            instances: Vec::new(),
            clear_every_frame: true,
            is_decal: false,
        }
    }

    pub fn new_thing(thing: Thing,
                     instance: Instance,
                     is_decal: bool,
                     window: &GlutinFacade)
                     -> Batch {
        Batch {
            vertices: glium::VertexBuffer::new(window, &thing.vertices).unwrap(),
            indices: glium::IndexBuffer::new(window,
                                             index::PrimitiveType::TrianglesList,
                                             &thing.indices)
                .unwrap(),
            instances: vec![instance],
            clear_every_frame: false,
            is_decal: is_decal,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
}

implement_vertex!(Vertex, position);

#[derive(Copy, Clone)]
pub struct Instance {
    pub instance_position: [f32; 3],
    pub instance_direction: [f32; 2],
    pub instance_color: [f32; 3],
}

implement_vertex!(Instance,
                  instance_position,
                  instance_direction,
                  instance_color);

impl Instance {
    pub fn with_color(color: [f32; 3]) -> Instance {
        Instance {
            instance_position: [0.0, 0.0, 0.0],
            instance_direction: [1.0, 0.0],
            instance_color: color,
        }
    }
}
