pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, Into2d, Into3d, WithUniqueOrthogonal,
                    try_inverse};
use kay::World;

use {Renderer, RendererID};

pub trait ProjectionRequester {
    fn projected_3d(&mut self, position_3d: P3, world: &mut World);
}

impl Renderer {
    /// Critical
    pub fn project_2d_to_3d(
        &mut self,
        position_2d: P2,
        requester: ProjectionRequesterID,
        world: &mut World,
    ) {
        requester.projected_3d(self.project(position_2d), world);
    }

    pub fn project(&self, position_2d: P2) -> P3 {
        let eye = &self.scene.eye;
        let frame_size = self.render_context.window.get_framebuffer_dimensions();

        // mouse is on the close plane of the frustum
        let normalized_2d_position = V4::new(
            (position_2d.x / (frame_size.0 as N)) * 2.0 - 1.0,
            (-position_2d.y / (frame_size.1 as N)) * 2.0 + 1.0,
            -1.0,
            1.0,
        );

        let inverse_view = Iso3::look_at_rh(&eye.position, &eye.target, &eye.up)
            .to_homogeneous()
            .try_inverse()
            .unwrap();
        let inverse_perspective = Persp3::new(
            frame_size.0 as f32 / frame_size.1 as f32,
            eye.field_of_view,
            0.1,
            1000.0,
        ).as_matrix()
            .try_inverse()
            .unwrap();

        // converts from frustum to position relative to camera
        let mut position_from_camera = inverse_perspective * normalized_2d_position;
        // reinterpret that as a vector (direction)
        position_from_camera.w = 0.0;
        // convert into world coordinates
        let direction_into_world = inverse_view * position_from_camera;

        let direction_into_world_3d = V3::new(
            direction_into_world.x,
            direction_into_world.y,
            direction_into_world.z,
        );
        // / direction_into_world.w;

        let distance = -eye.position.z / direction_into_world_3d.z;
        // position in world
        eye.position + distance * direction_into_world_3d
    }
}

mod kay_auto;
pub use self::kay_auto::*;
