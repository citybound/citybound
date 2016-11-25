use descartes::{Path, Band, Segment, P2, N, FiniteCurve, WithUniqueOrthogonal};
use kay::{CVec, Compact, Individual};
use monet::{Thing, Vertex, Renderer, UpdateThing, Instance};

#[derive(Compact, Clone)]
pub struct CPath {
    segments: CVec<Segment>
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

#[derive(Clone)]
pub enum AnyShape{
    Circle(::descartes::Circle),
    Band(::descartes::Band<CPath>),
    Everywhere
}

impl ::descartes::Shape for AnyShape {
    fn contains(&self, point: P2) -> bool {
        match *self {
            AnyShape::Circle(circle) => circle.contains(point),
            AnyShape::Band(ref band) => band.contains(point),
            AnyShape::Everywhere => true
        }
    }
}

impl Compact for AnyShape {
    fn is_still_compact(&self) -> bool {match *self {
        AnyShape::Band(Band{ref path, ..}) => path.is_still_compact(),
        _ => true
    }}

    fn dynamic_size_bytes(&self) -> usize {match *self {
        AnyShape::Band(Band{ref path, ..}) => path.dynamic_size_bytes(),
        _ => 0
    }}

    unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
        *self = source.clone();
        if let AnyShape::Band(Band{ref mut path, ..}) = *self {
            if let AnyShape::Band(Band{path: ref source_path, ..}) = *source {
                path.compact_from(source_path, new_dynamic_part);
            } else {unreachable!()}
        }
    }

    unsafe fn decompact(&self) -> AnyShape {match *self {
        AnyShape::Band(Band{ref path, width}) => AnyShape::Band(Band{path: path.decompact(), width: width}),
        AnyShape::Circle(circle) => AnyShape::Circle(circle),
        AnyShape::Everywhere => AnyShape::Everywhere
    }}
}

fn to_vertex(point: P2, z: N) -> Vertex {
    Vertex{position: [point.x, point.y, z]}
}

const CURVE_LINEARIZATION_MAX_ANGLE : f32 = 0.03;

pub fn band_to_thing<P: Path>(band: &Band<P>, z: N) -> Thing {
    let mut vertices = Vec::<Vertex>::new();
    let mut indices = Vec::<u16>::new();
    for segment in band.path.segments() {
        if segment.is_linear() {
            let first_new_vertex = vertices.len() as u16;
            let orth_direction = segment.center_or_direction.orthogonal();
            vertices.push(to_vertex(segment.start + band.width * orth_direction, z));
            vertices.push(to_vertex(segment.start - band.width * orth_direction, z));
            vertices.push(to_vertex(segment.end + band.width * orth_direction, z));
            vertices.push(to_vertex(segment.end - band.width * orth_direction, z));

            indices.extend_from_slice(&[first_new_vertex, first_new_vertex + 1, first_new_vertex + 2]);
            indices.extend_from_slice(&[first_new_vertex + 1, first_new_vertex + 3, first_new_vertex + 2]);
        } else {
            let angle_span = segment.length / segment.radius();
            let subdivisions = (angle_span / CURVE_LINEARIZATION_MAX_ANGLE).max(1.0).floor() as usize;
            let distance_per_subdivision = segment.length / (subdivisions as f32);

            let position = segment.start;
            let orth_direction = segment.start_direction().orthogonal();

            vertices.push(to_vertex(position + band.width * orth_direction, z));
            vertices.push(to_vertex(position - band.width * orth_direction, z));

            for subdivision in 0..subdivisions {
                let first_new_vertex = vertices.len() as u16;
                let distance = (subdivision + 1) as f32 * distance_per_subdivision;
                let position = segment.along(distance);
                let orth_direction = segment.direction_along(distance).orthogonal();

                vertices.push(to_vertex(position + band.width * orth_direction, z));
                vertices.push(to_vertex(position - band.width * orth_direction, z));

                indices.extend_from_slice(&[first_new_vertex - 2, first_new_vertex - 1, first_new_vertex]);
                indices.extend_from_slice(&[first_new_vertex - 1, first_new_vertex + 1, first_new_vertex]);
            }
        }
    }

    Thing::new(vertices, indices)
}

pub fn dash_path<P: Path>(path: P, dash_length: f32, gap_length: f32) -> Vec<P> {
    let mut on_dash = true;
    let mut position = 0.0;
    let mut dashes = Vec::new();

    while position < path.length() {
        let old_position = position;
        if on_dash {
            position += dash_length;
            if let Some(dash) = path.subsection(old_position, position) {
                dashes.push(dash)
            }
        } else {
            position += gap_length;
        }

        on_dash = !on_dash;
    }

    dashes
}

static mut LAST_DEBUG_THING: u16 = 0;

pub fn add_debug_path(path: CPath, color: [f32; 3], z: f32) {
    Renderer::id() << UpdateThing{
        scene_id: 0,
        thing_id: 4000 + unsafe{LAST_DEBUG_THING},
        thing: band_to_thing(&Band::new(path, 0.2), z),
        instance: Instance::with_color(color)
    };
    unsafe{LAST_DEBUG_THING += 1}
}

pub fn add_debug_point(point: P2, color: [f32; 3], z: f32) {
    Renderer::id() << UpdateThing{
        scene_id: 0,
        thing_id: 4000 + unsafe{LAST_DEBUG_THING},
        thing: Thing::new(
                vec![
                    Vertex{position: [point.x + -0.5, point.y + -0.5, z]},
                    Vertex{position: [point.x + 0.5, point.y + -0.5, z]},
                    Vertex{position: [point.x + 0.5, point.y + 0.5, z]},
                    Vertex{position: [point.x + -0.5, point.y + 0.5, z]}
                ],
                vec![
                    0, 1, 2,
                    2, 3, 0
                ]
            ),
        instance: Instance::with_color(color)
    };
    unsafe{LAST_DEBUG_THING += 1}
}