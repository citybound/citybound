use descartes::{LinePath, Band, P2};
use monet::{Mesh, Vertex, RendererID, Instance};

static mut LAST_DEBUG_THING: u32 = 0;
pub static mut DEBUG_RENDERER: Option<RendererID> = None;

use kay::World;

pub fn add_debug_line(from: P2, to: P2, color: [f32; 3], z: f32, world: &mut World) {
    if let Some(path) = LinePath::new(vec![from, to].into()) {
        add_debug_path(path, color, z, world);
    }
}

pub fn add_debug_path(path: LinePath, color: [f32; 3], z: f32, world: &mut World) {
    if let Some(renderer) = unsafe { DEBUG_RENDERER } {
        renderer.update_individual(
            4_000_000_000 + unsafe { LAST_DEBUG_THING },
            Mesh::from_band(&Band::new(path, 0.2), z),
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
                Vertex {
                    position: [point.x + -0.5, point.y + -0.5, z],
                },
                Vertex {
                    position: [point.x + 0.5, point.y + -0.5, z],
                },
                Vertex {
                    position: [point.x + 0.5, point.y + 0.5, z],
                },
                Vertex {
                    position: [point.x + -0.5, point.y + 0.5, z],
                },
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
