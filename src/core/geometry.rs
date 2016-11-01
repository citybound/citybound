use descartes::{Path, Segment, P2, N, FiniteCurve, WithUniqueOrthogonal};
use kay::{CVec, Compact};
use monet::{Thing, Vertex};

derive_compact! {
    pub struct CPath {
        segments: CVec<Segment>
    }
}

impl Path for CPath {
    fn segments(&self) -> &[Segment] {
        &self.segments
    }

    fn new(vec: Vec<Segment>) -> Self {
        CPath{
            segments: vec.into()
        }
    }
}

fn to_vertex(point: P2, z: N) -> Vertex {
    Vertex{position: [point.x, point.y, z]}
}

const CURVE_LINEARIZATION_MAX_ANGLE : f32 = 0.1;

pub fn path_to_band<P: Path>(path: &P, width: N, z: N) -> Thing {
    let mut vertices = Vec::<Vertex>::new();
    let mut indices = Vec::<u16>::new();
    for segment in path.segments() {
        if segment.is_linear() {
            let first_new_vertex = vertices.len() as u16;
            let orth_direction = segment.center_or_direction.orthogonal();
            vertices.push(to_vertex(segment.start + width * orth_direction, z));
            vertices.push(to_vertex(segment.start - width * orth_direction, z));
            vertices.push(to_vertex(segment.end + width * orth_direction, z));
            vertices.push(to_vertex(segment.end - width * orth_direction, z));

            indices.extend_from_slice(&[first_new_vertex, first_new_vertex + 1, first_new_vertex + 2]);
            indices.extend_from_slice(&[first_new_vertex + 1, first_new_vertex + 3, first_new_vertex + 2]);
        } else {
            let angle_span = segment.length / segment.radius();
            let subdivisions = (angle_span / CURVE_LINEARIZATION_MAX_ANGLE).max(1.0).floor() as usize;
            let distance_per_subdivision = segment.length / (subdivisions as f32);

            let position = segment.start;
            let orth_direction = segment.start_direction().orthogonal();

            vertices.push(to_vertex(position + width * orth_direction, z));
            vertices.push(to_vertex(position - width * orth_direction, z));

            for subdivision in 0..subdivisions {
                let first_new_vertex = vertices.len() as u16;
                let distance = (subdivision + 1) as f32 * distance_per_subdivision;
                let position = segment.along(distance);
                let orth_direction = segment.direction_along(distance).orthogonal();

                vertices.push(to_vertex(position + width * orth_direction, z));
                vertices.push(to_vertex(position - width * orth_direction, z));

                indices.extend_from_slice(&[first_new_vertex - 2, first_new_vertex - 1, first_new_vertex]);
                indices.extend_from_slice(&[first_new_vertex - 1, first_new_vertex + 1, first_new_vertex]);
            }
        }
    }

    Thing::new(vertices, indices)
}