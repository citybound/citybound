pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, Into2d, Into3d, WithUniqueOrthogonal,
Area, LinePath};

use compact::CVec;

#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
}

#[derive(Copy, Clone)]
pub struct Instance {
    pub instance_position: [f32; 3],
    pub instance_direction: [f32; 2],
    pub instance_color: [f32; 3],
}

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

impl Mesh {
    pub fn from_area(area: &Area) -> Mesh {
        let path_iterator = PathIter::new(area.primitives.iter().flat_map(|primitive| {
            primitive
                .boundary
                .path()
                .segments()
                .with_position()
                .flat_map(|segment_with_position| {
                    let initial_move = match segment_with_position {
                        Position::First(segment) | Position::Only(segment) => Some(
                            PathEvent::MoveTo(point(segment.start().x, segment.start().y)),
                        ),
                        _ => None,
                    };

                    let segment = segment_with_position.into_inner();

                    initial_move
                        .into_iter()
                        .chain(Some(PathEvent::LineTo(point(
                            segment.end().x,
                            segment.end().y,
                        ))))
                        .collect::<Vec<_>>()
                })
        }));

        let mut tesselator = FillTessellator::new();
        let mut output = Mesh::empty();

        tesselator
            .tessellate_path(path_iterator, &FillOptions::default(), &mut output)
            .unwrap();

        output
    }

    pub fn from_path_as_band(path: &LinePath, width: N, z: N) -> Mesh {
        Self::from_path_as_band_asymmetric(path, width / 2.0, width / 2.0, z)
    }

    pub fn from_path_as_band_asymmetric(
        path: &LinePath,
        width_left: N,
        width_right: N,
        z: N,
    ) -> Mesh {
        fn to_vertex(point: P2, z: N) -> Vertex {
            Vertex {
                position: [point.x, point.y, z],
            }
        }

        let left_points = path
            .shift_orthogonally_vectors()
            .zip(path.points.iter())
            .map(|(shift, point)| point - width_left * shift.0);
        let right_points = path
            .shift_orthogonally_vectors()
            .zip(path.points.iter())
            .map(|(shift, point)| point + width_right * shift.0);

        let vertices = left_points
            .chain(right_points)
            .map(|p| to_vertex(p, z))
            .collect::<Vec<_>>();

        let left_len = path.points.len();

        let indices = (0..(left_len - 1))
            .flat_map(|left_i| {
                let left_i = left_i as u16;
                let right_i = left_i + left_len as u16;

                vec![
                    left_i,
                    right_i.min(vertices.len() as u16 - 1),
                    left_i + 1,
                    left_i + 1,
                    right_i.min(vertices.len() as u16 - 1),
                    (right_i + 1).min(vertices.len() as u16 - 1),
                ]
            })
            .collect();

        Mesh::new(vertices, indices)
    }
}
