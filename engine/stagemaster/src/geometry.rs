use descartes::{SimpleShape, Path, Band, Segment, P2, N, FiniteCurve, WithUniqueOrthogonal};
use compact::{CVec, Compact};
use monet::{Geometry, Vertex, RendererID, Instance};

#[derive(Compact, Clone)]
pub struct CPath {
    segments: CVec<Segment>,
}

impl Path for CPath {
    fn segments(&self) -> &[Segment] {
        &self.segments
    }

    fn new(vec: Vec<Segment>) -> Self {
        CPath { segments: vec.into() }
    }
}

#[derive(Compact, Clone)]
pub struct CShape {
    outline: CPath,
}

impl SimpleShape for CShape {
    type P = CPath;

    fn outline(&self) -> &CPath {
        &self.outline
    }

    fn new(outline: CPath) -> Self {
        CShape { outline }
    }
}

#[derive(Clone)]
pub enum AnyShape {
    Circle(::descartes::Circle),
    Band(::descartes::Band<CPath>),
    Everywhere,
}

impl ::descartes::Shape for AnyShape {
    fn location_of(&self, point: P2) -> ::descartes::PointOnShapeLocation {
        match *self {
            AnyShape::Circle(circle) => circle.location_of(point),
            AnyShape::Band(ref band) => band.location_of(point),
            AnyShape::Everywhere => ::descartes::PointOnShapeLocation::Inside,
        }
    }
}

impl Compact for AnyShape {
    fn is_still_compact(&self) -> bool {
        match *self {
            AnyShape::Band(Band { ref path, .. }) => path.is_still_compact(),
            _ => true,
        }
    }

    fn dynamic_size_bytes(&self) -> usize {
        match *self {
            AnyShape::Band(Band { ref path, .. }) => path.dynamic_size_bytes(),
            _ => 0,
        }
    }

    unsafe fn compact(source: *mut Self, dest: *mut Self, new_dynamic_part: *mut u8) {
        ::std::ptr::copy_nonoverlapping(source, dest, 1);
        if let AnyShape::Band(Band { ref mut path, width }) = *source {
            if let AnyShape::Band(Band {
                                      path: ref mut dest_path,
                                      width: ref mut dest_width,
                                  }) = *dest
            {
                Compact::compact(path, dest_path, new_dynamic_part);
                *dest_width = width;
            }
        }
    }

    unsafe fn decompact(source: *const Self) -> AnyShape {
        match *source {
            AnyShape::Band(Band { ref path, width }) => {
                AnyShape::Band(Band {
                    path: Compact::decompact(path),
                    width: width,
                })
            }
            AnyShape::Circle(circle) => AnyShape::Circle(circle),
            AnyShape::Everywhere => AnyShape::Everywhere,
        }
    }
}

fn to_vertex(point: P2, z: N) -> Vertex {
    Vertex { position: [point.x, point.y, z] }
}

const CURVE_LINEARIZATION_MAX_ANGLE: f32 = 0.03;

pub fn band_to_geometry<P: Path>(band: &Band<P>, z: N) -> Geometry {
    let mut vertices = Vec::<Vertex>::new();
    let mut indices = Vec::<u16>::new();
    for segment in band.path.segments() {
        if segment.is_linear() {
            let first_new_vertex = vertices.len() as u16;
            let orth_direction = segment.center_or_direction.orthogonal();
            vertices.push(to_vertex(
                segment.start + band.width / 2.0 * orth_direction,
                z,
            ));
            vertices.push(to_vertex(
                segment.start - band.width / 2.0 * orth_direction,
                z,
            ));
            vertices.push(to_vertex(
                segment.end + band.width / 2.0 * orth_direction,
                z,
            ));
            vertices.push(to_vertex(
                segment.end - band.width / 2.0 * orth_direction,
                z,
            ));

            indices.extend_from_slice(
                &[first_new_vertex, first_new_vertex + 1, first_new_vertex + 2],
            );
            indices.extend_from_slice(
                &[
                    first_new_vertex + 1,
                    first_new_vertex + 3,
                    first_new_vertex + 2,
                ],
            );
        } else {
            let angle_span = segment.length / segment.radius();
            let subdivisions = (angle_span / CURVE_LINEARIZATION_MAX_ANGLE)
                .max(1.0)
                .floor() as usize;
            let distance_per_subdivision = segment.length / (subdivisions as f32);

            let position = segment.start;
            let orth_direction = segment.start_direction().orthogonal();

            vertices.push(to_vertex(position + band.width / 2.0 * orth_direction, z));
            vertices.push(to_vertex(position - band.width / 2.0 * orth_direction, z));

            for subdivision in 0..subdivisions {
                let first_new_vertex = vertices.len() as u16;
                let distance = (subdivision + 1) as f32 * distance_per_subdivision;
                let position = segment.along(distance);
                let orth_direction = segment.direction_along(distance).orthogonal();

                vertices.push(to_vertex(position + band.width / 2.0 * orth_direction, z));
                vertices.push(to_vertex(position - band.width / 2.0 * orth_direction, z));

                indices.extend_from_slice(
                    &[first_new_vertex - 2, first_new_vertex - 1, first_new_vertex],
                );
                indices.extend_from_slice(
                    &[first_new_vertex - 1, first_new_vertex + 1, first_new_vertex],
                );
            }
        }
    }

    Geometry::new(vertices, indices)
}

pub fn dash_path<P: Path>(path: &P, dash_length: f32, gap_length: f32) -> Vec<P> {
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
pub static mut DEBUG_RENDERER: Option<RendererID> = None;

use kay::World;

pub fn add_debug_line(from: P2, to: P2, color: [f32; 3], z: f32, world: &mut World) {
    if let Some(line) = Segment::line(from, to) {
        let path = CPath::new(vec![line]);
        add_debug_path(path, color, z, world);
    }
}

pub fn add_debug_path(path: CPath, color: [f32; 3], z: f32, world: &mut World) {
    if let Some(renderer) = unsafe { DEBUG_RENDERER } {
        renderer.update_individual(
            0,
            50_000 + unsafe { LAST_DEBUG_THING },
            band_to_geometry(&Band::new(path, 0.2), z),
            Instance::with_color(color),
            true,
            world,
        );
        unsafe { LAST_DEBUG_THING += 1 }
    }
}

pub fn add_debug_point(point: P2, color: [f32; 3], z: f32, world: &mut World) {
    if let Some(renderer) = unsafe { DEBUG_RENDERER } {
        let geometry = Geometry::new(
            vec![
                Vertex { position: [point.x + -0.5, point.y + -0.5, z] },
                Vertex { position: [point.x + 0.5, point.y + -0.5, z] },
                Vertex { position: [point.x + 0.5, point.y + 0.5, z] },
                Vertex { position: [point.x + -0.5, point.y + 0.5, z] },
            ],
            vec![0, 1, 2, 2, 3, 0],
        );
        renderer.update_individual(
            0,
            50_000 + unsafe { LAST_DEBUG_THING },
            geometry,
            Instance::with_color(color),
            true,
            world,
        );
        unsafe { LAST_DEBUG_THING += 1 }
    }
}
