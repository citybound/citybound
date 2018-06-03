pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, Into2d, Into3d, WithUniqueOrthogonal,
Path, Area, Band, FiniteCurve};

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
pub struct Mesh {
    pub vertices: CVec<Vertex>,
    pub indices: CVec<u16>,
}

impl Mesh {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u16>) -> Mesh {
        Mesh {
            vertices: vertices.into(),
            indices: indices.into(),
        }
    }

    pub fn empty() -> Mesh {
        Mesh {
            vertices: CVec::new(),
            indices: CVec::new(),
        }
    }
}

impl Clone for Mesh {
    fn clone(&self) -> Mesh {
        Mesh {
            vertices: self.vertices.to_vec().into(),
            indices: self.indices.to_vec().into(),
        }
    }
}

impl ::std::ops::Add for Mesh {
    type Output = Mesh;

    fn add(mut self, rhs: Mesh) -> Mesh {
        let self_n_vertices = self.vertices.len();
        self.vertices.extend_from_copy_slice(&rhs.vertices);
        self.indices
            .extend(rhs.indices.iter().map(|i| *i + self_n_vertices as u16));
        self
    }
}

impl ::std::ops::AddAssign for Mesh {
    fn add_assign(&mut self, rhs: Mesh) {
        let self_n_vertices = self.vertices.len();
        for vertex in rhs.vertices.iter().cloned() {
            self.vertices.push(vertex);
        }
        for index in rhs.indices.iter() {
            self.indices.push(index + self_n_vertices as u16)
        }
    }
}

impl ::std::iter::Sum for Mesh {
    fn sum<I: Iterator<Item = Mesh>>(iter: I) -> Mesh {
        let mut summed_mesh = Mesh {
            vertices: CVec::new(),
            indices: CVec::new(),
        };
        for mesh in iter {
            summed_mesh += mesh;
        }
        summed_mesh
    }
}

impl<'a> ::std::ops::AddAssign<&'a Mesh> for Mesh {
    fn add_assign(&mut self, rhs: &'a Mesh) {
        let self_n_vertices = self.vertices.len();
        for vertex in rhs.vertices.iter().cloned() {
            self.vertices.push(vertex);
        }
        for index in rhs.indices.iter() {
            self.indices.push(index + self_n_vertices as u16)
        }
    }
}

impl<'a> ::std::iter::Sum<&'a Mesh> for Mesh {
    fn sum<I: Iterator<Item = &'a Mesh>>(iter: I) -> Mesh {
        let mut summed_mesh = Mesh {
            vertices: CVec::new(),
            indices: CVec::new(),
        };
        for mesh in iter {
            summed_mesh += mesh;
        }
        summed_mesh
    }
}

use itertools::{Itertools, Position};
use lyon_tessellation::{FillTessellator, FillOptions, FillVertex, GeometryBuilder};
use lyon_tessellation::geometry_builder::{VertexId, Count};
use lyon_tessellation::path::iterator::PathIter;
use lyon_tessellation::path::PathEvent;
use lyon_tessellation::math::point;

impl GeometryBuilder<FillVertex> for Mesh {
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

impl Mesh {
    pub fn from_area(area: &Area) -> Mesh {
        let path_iterator =
            PathIter::new(area.primitives.iter().flat_map(|primitive| {
                primitive.boundary.segments.iter().with_position().flat_map(
                    |segment_with_position| {
                        let initial_move = match segment_with_position {
                            Position::First(segment) | Position::Only(segment) => Some(
                                PathEvent::MoveTo(point(segment.start().x, segment.start().y)),
                            ),
                            _ => None,
                        };

                        let segment = segment_with_position.into_inner();

                        if segment.is_linear() {
                            initial_move
                                .into_iter()
                                .chain(Some(PathEvent::LineTo(point(
                                    segment.end().x,
                                    segment.end().y,
                                ))))
                                .collect::<Vec<_>>()
                        } else {
                            let angle_span = segment.length / segment.radius();
                            let subdivisions = (angle_span / CURVE_LINEARIZATION_MAX_ANGLE)
                                .max(1.0)
                                .floor() as usize;
                            let distance_per_subdivision = segment.length / (subdivisions as f32);

                            initial_move
                                .into_iter()
                                .chain((0..subdivisions).into_iter().map(|subdivision| {
                                    let distance =
                                        (subdivision + 1) as f32 * distance_per_subdivision;
                                    let position = segment.along(distance);

                                    PathEvent::LineTo(point(position.x, position.y))
                                }))
                                .collect::<Vec<_>>()
                        }
                    },
                )
            }));

        let mut tesselator = FillTessellator::new();
        let mut output = Mesh::empty();

        tesselator
            .tessellate_path(path_iterator, &FillOptions::default(), &mut output)
            .unwrap();

        output
    }

    pub fn from_band(band: &Band, z: N) -> Mesh {
        fn to_vertex(point: P2, z: N) -> Vertex {
            Vertex {
                position: [point.x, point.y, z],
            }
        }

        let mut vertices = Vec::<Vertex>::new();
        let mut indices = Vec::<u16>::new();
        for segment in &band.path.segments {
            if segment.is_linear() {
                let first_new_vertex = vertices.len() as u16;
                let orth_direction = segment.center_or_direction.orthogonal();
                vertices.push(to_vertex(
                    segment.start + band.width_right * orth_direction,
                    z,
                ));
                vertices.push(to_vertex(
                    segment.start - band.width_left * orth_direction,
                    z,
                ));
                vertices.push(to_vertex(
                    segment.end + band.width_right * orth_direction,
                    z,
                ));
                vertices.push(to_vertex(segment.end - band.width_left * orth_direction, z));

                indices.extend_from_slice(&[
                    first_new_vertex,
                    first_new_vertex + 1,
                    first_new_vertex + 2,
                ]);
                indices.extend_from_slice(&[
                    first_new_vertex + 1,
                    first_new_vertex + 3,
                    first_new_vertex + 2,
                ]);
            } else {
                let angle_span = segment.length / segment.radius();
                let subdivisions = (angle_span / CURVE_LINEARIZATION_MAX_ANGLE)
                    .max(1.0)
                    .floor() as usize;
                let distance_per_subdivision = segment.length / (subdivisions as f32);

                let position = segment.start;
                let orth_direction = segment.start_direction().orthogonal();

                vertices.push(to_vertex(position + band.width_right * orth_direction, z));
                vertices.push(to_vertex(position - band.width_left * orth_direction, z));

                for subdivision in 0..subdivisions {
                    let first_new_vertex = vertices.len() as u16;
                    let distance = (subdivision + 1) as f32 * distance_per_subdivision;
                    let position = segment.along(distance);
                    let orth_direction = segment.direction_along(distance).orthogonal();

                    vertices.push(to_vertex(position + band.width_right * orth_direction, z));
                    vertices.push(to_vertex(position - band.width_left * orth_direction, z));

                    indices.extend_from_slice(&[
                        first_new_vertex - 2,
                        first_new_vertex - 1,
                        first_new_vertex,
                    ]);
                    indices.extend_from_slice(&[
                        first_new_vertex - 1,
                        first_new_vertex + 1,
                        first_new_vertex,
                    ]);
                }
            }
        }

        Mesh::new(vertices, indices)
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
    pub fn new(prototype: &Mesh, window: &Display) -> Batch {
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
        mesh: &Mesh,
        instance: Instance,
        is_decal: bool,
        window: &Display,
    ) -> Batch {
        Batch {
            vertices: glium::VertexBuffer::new(window, &mesh.vertices).unwrap(),
            indices: glium::IndexBuffer::new(
                window,
                index::PrimitiveType::TrianglesList,
                &mesh.indices,
            ).unwrap(),
            instances: vec![instance],
            clear_every_frame: false,
            full_frame_instance_end: None,
            is_decal,
            frame: 0,
        }
    }
}
