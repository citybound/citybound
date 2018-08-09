use descartes::{P2, CurvedPath};
use monet::{Mesh};
use style::dimensions::CONTROL_POINT_HANDLE_RADIUS;

pub fn static_meshes() -> Vec<(&'static str, Mesh)> {
    let dot_mesh = Mesh::from_path_as_band(
        &CurvedPath::circle(P2::new(0.0, 0.0), CONTROL_POINT_HANDLE_RADIUS)
            .unwrap()
            .to_line_path(),
        0.3,
        1.0,
    );

    vec![("GestureDot", dot_mesh)]
}

pub mod kay_auto;
pub use self::kay_auto::auto_setup;
