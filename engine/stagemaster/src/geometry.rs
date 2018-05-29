use descartes::{Path, Band, Segment, P2, N, FiniteCurve, WithUniqueOrthogonal};
use monet::{Mesh, Vertex, RendererID, Instance};

fn to_vertex(point: P2, z: N) -> Vertex {
    Vertex { position: [point.x, point.y, z] }
}

const CURVE_LINEARIZATION_MAX_ANGLE: f32 = 0.03;

pub fn band_to_mesh(band: &Band, z: N) -> Mesh {
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

            vertices.push(to_vertex(position + band.width_right * orth_direction, z));
            vertices.push(to_vertex(position - band.width_left * orth_direction, z));

            for subdivision in 0..subdivisions {
                let first_new_vertex = vertices.len() as u16;
                let distance = (subdivision + 1) as f32 * distance_per_subdivision;
                let position = segment.along(distance);
                let orth_direction = segment.direction_along(distance).orthogonal();

                vertices.push(to_vertex(position + band.width_right * orth_direction, z));
                vertices.push(to_vertex(position - band.width_left * orth_direction, z));

                indices.extend_from_slice(
                    &[first_new_vertex - 2, first_new_vertex - 1, first_new_vertex],
                );
                indices.extend_from_slice(
                    &[first_new_vertex - 1, first_new_vertex + 1, first_new_vertex],
                );
            }
        }
    }

    Mesh::new(vertices, indices)
}

pub fn dash_path(path: &Path, dash_length: f32, gap_length: f32) -> Vec<Path> {
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

static mut LAST_DEBUG_THING: u32 = 0;
pub static mut DEBUG_RENDERER: Option<RendererID> = None;

use kay::World;

pub fn add_debug_line(from: P2, to: P2, color: [f32; 3], z: f32, world: &mut World) {
    if let Some(line) = Segment::line(from, to) {
        let path = Path::new(vec![line].into()).unwrap();
        add_debug_path(path, color, z, world);
    }
}

pub fn add_debug_path(path: Path, color: [f32; 3], z: f32, world: &mut World) {
    if let Some(renderer) = unsafe { DEBUG_RENDERER } {
        renderer.update_individual(
            4_000_000_000 + unsafe { LAST_DEBUG_THING },
            band_to_mesh(&Band::new(path, 0.2), z),
            Instance::with_color(color),
            true,
            world,
        );
        unsafe { LAST_DEBUG_THING += 1 }
    }
}

pub fn add_debug_point(point: P2, color: [f32; 3], z: f32, world: &mut World) {
    if let Some(renderer) = unsafe { DEBUG_RENDERER } {
        let mesh = Mesh::new(
            vec![
                Vertex { position: [point.x + -0.5, point.y + -0.5, z] },
                Vertex { position: [point.x + 0.5, point.y + -0.5, z] },
                Vertex { position: [point.x + 0.5, point.y + 0.5, z] },
                Vertex { position: [point.x + -0.5, point.y + 0.5, z] },
            ],
            vec![0, 1, 2, 2, 3, 0],
        );
        renderer.update_individual(
            4_000_000_000 + unsafe { LAST_DEBUG_THING },
            mesh,
            Instance::with_color(color),
            true,
            world,
        );
        unsafe { LAST_DEBUG_THING += 1 }
    }
}
