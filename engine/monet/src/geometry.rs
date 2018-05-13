pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, Into2d, Into3d, WithUniqueOrthogonal,
                    Path, SimpleShape, FiniteCurve};

use glium::{self, index};
use glium::backend::glutin::Display;

use compact::CVec;

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

implement_vertex!(
    Instance,
    instance_position,
    instance_direction,
    instance_color
);

impl Instance {
    pub fn with_color(color: [f32; 3]) -> Instance {
        Instance {
            instance_position: [0.0, 0.0, 0.0],
            instance_direction: [1.0, 0.0],
            instance_color: color,
        }
    }
}

#[derive(Compact, Debug)]
pub struct Geometry {
    pub vertices: CVec<Vertex>,
    pub indices: CVec<u16>,
}

impl Geometry {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u16>) -> Geometry {
        Geometry {
            vertices: vertices.into(),
            indices: indices.into(),
        }
    }

    pub fn empty() -> Geometry {
        Geometry {
            vertices: CVec::new(),
            indices: CVec::new(),
        }
    }
}

impl Clone for Geometry {
    fn clone(&self) -> Geometry {
        Geometry {
            vertices: self.vertices.to_vec().into(),
            indices: self.indices.to_vec().into(),
        }
    }
}

impl ::std::ops::Add for Geometry {
    type Output = Geometry;

    fn add(mut self, rhs: Geometry) -> Geometry {
        let self_n_vertices = self.vertices.len();
        self.vertices.extend_from_copy_slice(&rhs.vertices);
        self.indices.extend(rhs.indices.iter().map(|i| {
            *i + self_n_vertices as u16
        }));
        self
    }
}

impl ::std::ops::AddAssign for Geometry {
    fn add_assign(&mut self, rhs: Geometry) {
        let self_n_vertices = self.vertices.len();
        for vertex in rhs.vertices.iter().cloned() {
            self.vertices.push(vertex);
        }
        for index in rhs.indices.iter() {
            self.indices.push(index + self_n_vertices as u16)
        }
    }
}

impl ::std::iter::Sum for Geometry {
    fn sum<I: Iterator<Item = Geometry>>(iter: I) -> Geometry {
        let mut summed_geometry = Geometry {
            vertices: CVec::new(),
            indices: CVec::new(),
        };
        for geometry in iter {
            summed_geometry += geometry;
        }
        summed_geometry
    }
}

impl<'a> ::std::ops::AddAssign<&'a Geometry> for Geometry {
    fn add_assign(&mut self, rhs: &'a Geometry) {
        let self_n_vertices = self.vertices.len();
        for vertex in rhs.vertices.iter().cloned() {
            self.vertices.push(vertex);
        }
        for index in rhs.indices.iter() {
            self.indices.push(index + self_n_vertices as u16)
        }
    }
}

impl<'a> ::std::iter::Sum<&'a Geometry> for Geometry {
    fn sum<I: Iterator<Item = &'a Geometry>>(iter: I) -> Geometry {
        let mut summed_geometry = Geometry {
            vertices: CVec::new(),
            indices: CVec::new(),
        };
        for geometry in iter {
            summed_geometry += geometry;
        }
        summed_geometry
    }
}

use itertools::{Itertools, Position};
use lyon_tessellation::{FillTessellator, FillOptions, FillVertex, GeometryBuilder};
use lyon_tessellation::geometry_builder::{VertexId, Count};
use lyon_tessellation::path::iterator::PathIter;
use lyon_tessellation::path::PathEvent;
use lyon_tessellation::math::{point, vector, Angle};

impl GeometryBuilder<FillVertex> for Geometry {
    fn begin_geometry(&mut self) {}
    fn end_geometry(&mut self) -> Count {
        Count {
            vertices: self.vertices.len() as u32,
            indices: self.indices.len() as u32,
        }
    }
    fn abort_geometry(&mut self) {}
    fn add_vertex(&mut self, input: FillVertex) -> VertexId {
        let id = self.vertices.len();
        self.vertices.push(Vertex {
            position: [input.position.x, input.position.y, 0.0],
        });
        VertexId(id as u16)
    }
    fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
        self.indices.push(a.0);
        self.indices.push(b.0);
        self.indices.push(c.0);
    }
}

const CURVE_LINEARIZATION_MAX_ANGLE: f32 = 0.03;

impl Geometry {
    pub fn from_shape<S: SimpleShape>(shape: &S) -> Geometry {
        let path_iterator =
            PathIter::new(shape.outline().segments().iter().with_position().flat_map(
                |segment_with_position| {
                    let initial_move = match segment_with_position {
                        Position::First(segment) |
                        Position::Only(segment) => {
                            Some(PathEvent::MoveTo(
                                point(segment.start().x, segment.start().y),
                            ))
                        }
                        _ => None,
                    };

                    let segment = segment_with_position.into_inner();

                    if segment.is_linear() {
                        initial_move
                            .into_iter()
                            .chain(Some(
                                PathEvent::LineTo(point(segment.end().x, segment.end().y)),
                            ))
                            .collect::<Vec<_>>()
                    } else {
                        let angle_span = segment.length / segment.radius();
                        let subdivisions = (angle_span / CURVE_LINEARIZATION_MAX_ANGLE)
                            .max(1.0)
                            .floor() as usize;
                        let distance_per_subdivision = segment.length / (subdivisions as f32);

                        (0..subdivisions)
                            .into_iter()
                            .map(|subdivision| {
                                let distance = (subdivision + 1) as f32 * distance_per_subdivision;
                                let position = segment.along(distance);

                                PathEvent::LineTo(point(position.x, position.y))
                            })
                            .collect::<Vec<_>>()
                    }
                },
            ));

        let mut tesselator = FillTessellator::new();
        let mut output = Geometry::empty();

        tesselator
            .tessellate_path(path_iterator, &FillOptions::default(), &mut output)
            .unwrap();

        output
    }
}

pub struct Batch {
    pub vertices: glium::VertexBuffer<Vertex>,
    pub indices: glium::IndexBuffer<u16>,
    pub instances: Vec<Instance>,
    pub clear_every_frame: bool,
    pub full_frame_instance_end: Option<usize>,
    pub is_decal: bool,
    pub frame: usize,
}

impl Batch {
    pub fn new(prototype: &Geometry, window: &Display) -> Batch {
        Batch {
            vertices: glium::VertexBuffer::new(window, &prototype.vertices).unwrap(),
            indices: glium::IndexBuffer::new(
                window,
                index::PrimitiveType::TrianglesList,
                &prototype.indices,
            ).unwrap(),
            instances: Vec::new(),
            full_frame_instance_end: None,
            clear_every_frame: true,
            is_decal: false,
            frame: 0,
        }
    }

    pub fn new_individual(
        geometry: &Geometry,
        instance: Instance,
        is_decal: bool,
        window: &Display,
    ) -> Batch {
        Batch {
            vertices: glium::VertexBuffer::new(window, &geometry.vertices).unwrap(),
            indices: glium::IndexBuffer::new(
                window,
                index::PrimitiveType::TrianglesList,
                &geometry.indices,
            ).unwrap(),
            instances: vec![instance],
            clear_every_frame: false,
            full_frame_instance_end: None,
            is_decal: is_decal,
            frame: 0,
        }
    }
}
